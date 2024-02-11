import spacy
import re
from collections import defaultdict
import enum
import orjson
from tqdm import tqdm
import pandas as pd
from frozendict import frozendict

def write_jsonl(path, words):
    with open(path, 'w') as f:
        for word in words:
            s = orjson.dumps(word)
            s = s.decode() + '\n'
            f.write(s)

# 1. Load base vocab
# Iterate large corpus
# 2. Do POS tagging on this corpus. For each word:
# - If not in base vocab AND not a noun, add it to vocab
# - Record POS tag and morphology for this word
# - Add POS tag + morphology to a count dict

# TODO:
# - If noun, plural, and finishes by "s", add singular word to vocab
# - have less total words: be more strict to added word somehow?


# See https://universaldependencies.org/u/pos/ and
# https://universaldependencies.org/format.html#morphological-annotation for POS and morph info
vocab = {w: defaultdict(int) for w in set(pd.read_excel('lexicon.xls')["item"])}
encountered_vocab = set()
# https://github.com/Common-Voice/commonvoice-fr/issues/91
# https://wortschatz.uni-leipzig.de/en/download/French
doc_paths = ['./fra_mixed_2009_100K-sentences.txt', './fra_wikipedia_2021_100K-sentences.txt']
nlp = spacy.load("fr_dep_news_trf")

class Morph(enum.IntEnum):
    Gender = 1 # Masc, Fem
    Number = 2 # 'Sing', 'Plur'
    Person = 3 # '3', '1'...

def keep_interesting_morph(input_morph):
    out_morph = {}
    for morph in list(Morph):
        key = morph.name
        m = input_morph.get(key)
        if len(m):
            out_morph[key] = m[0]
    return frozendict(out_morph)

def keep_interesting_morph_dict(input_morph):
    out_morph = {}
    for morph in list(Morph):
        key = morph.name
        m = input_morph.get(key)
        if m is not None:
            out_morph[key] = m
    return out_morph

# Return biggest dict if all key/value pair of the smallest dict are in the biggest one
def dict_intersect(d1, d2):
    smallest = d1 if len(d1) < len(d2) else d2
    biggest = d2 if len(d1) < len(d2) else d1
    for key, value in smallest.items():
        if key not in biggest:
            return None
        if biggest[key] != value: return None
    return biggest

mappings = defaultdict(int)
regex_id = re.compile('[0-9]+\W+')
regex_apostrophes = re.compile('(\W[ldnmst])\W+(\w)')

new_words = defaultdict(int)
print('Tagging corpus...')
for doc_path in doc_paths:
    print(f'Tagging {doc_path}')
    with open(doc_path) as file_in:
        for line_nb, line in tqdm(enumerate(file_in), total=100000):
            # if line_nb > 400: break
            line = regex_id.sub('', line)
            line = regex_apostrophes.sub(r"\1'\2", line)
            doc = nlp(line)
            for index in range(len(doc) - 1):
                token = doc[index]
                word = token.text
                encountered_vocab.add(word)
                if word not in vocab:
                    new_words[word] = 1
                    vocab[word] = defaultdict(int)
                elif word in new_words:
                    new_words[word] += 1
                next_token = doc[index + 1]
                token_pos = token.pos_
                next_token_pos = next_token.pos_
                if token_pos == 'PUNCT' or next_token_pos == 'PUNCT':
                    continue
                token_morph = token.morph.to_dict()
                next_token_morph = next_token.morph.to_dict()
                tup = (token_pos, keep_interesting_morph(token.morph), next_token_pos, keep_interesting_morph(next_token.morph))
                mappings[tup] += 1
                vocab[word][(token_pos, frozendict(token_morph),)] += 1

for word, occurence in new_words.items():
    if occurence <= 3:
        del vocab[word]
        encountered_vocab.remove(word)

# delete upper case versions of word if already in vocab
for word in list(vocab.keys()):
    if any(x.isupper() for x in word) and word.lower() in vocab:
        del vocab[word]
        encountered_vocab.remove(word)

with open('tagging_stats.jsonl', 'w') as f:
    for tag, nb in sorted(mappings.items(), key= lambda x: x[1], reverse=True):
        tag = (tag[0], tag[1], tag[2], tag[3])
        # tag = (tag[0], keep_interesting_morph(tag[1]), tag[2], keep_interesting_morph(tag[3]))
        obj = {'tagging': tag, 'nb': nb}
        s = orjson.dumps(obj)
        s = s.decode() + '\n'
        f.write(s)

final_vocab = []

remaining_words = set(vocab.keys()) - encountered_vocab
print(f'Tagging {len(remaining_words)} remaining words in vocab...')
for index, word in tqdm(enumerate(remaining_words)):
    # if index > 100: break
    token = nlp(word)[0]
    final_vocab.append({
        'word': word,
        'pos': token.pos_,
        'morph': (keep_interesting_morph_dict(token.morph.to_dict()),),
    })

for word in encountered_vocab:
    stats = vocab[word]
    if len(stats) == 0:
        final_vocab.append({'word': word, 'pos': '', 'morph': {}})
        continue
    sorted_by_occ = sorted(stats.items(), key= lambda x: x[1], reverse=True)
    first_pos_morph = sorted_by_occ[0][0]
    morph_to_keep = [keep_interesting_morph_dict(first_pos_morph[1])]
    for (pos, morph), occ in sorted_by_occ[1:]:
        if pos == first_pos_morph[0] and occ > 10:
            to_add = keep_interesting_morph_dict(morph)
            should_add = True
            for existing in morph_to_keep:
                if existing == to_add:
                    should_add = False
                    continue
                biggest = dict_intersect(existing, to_add)
                if biggest is not None:
                    should_add = False
                    existing.update(biggest)
                    continue
            if should_add:
                morph_to_keep.append(to_add)
    final_vocab.append({'word': word, 'pos': first_pos_morph[0], 'morph': morph_to_keep})

alpha_diacritic_regex = re.compile(r'[^A-Za-z_À-ÿ]')
def should_keep_word(vocab_item):
    # filter out PROPN and X and unidentified
    if vocab_item['pos'] == 'PROPN' or vocab_item['pos'] == 'X' or vocab_item['pos'] == '': return False
    word = vocab_item['word']
    if len(word) < 2: return False
    if word.isupper(): return False
    if alpha_diacritic_regex.search(word) is not None: return False
    return True

final_vocab = filter(should_keep_word, final_vocab)
final_vocab = sorted(final_vocab, key=lambda x: (len(x['word']), x['word'],))
write_jsonl('vocab.jsonl', final_vocab)