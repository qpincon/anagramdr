import spacy
import re
from collections import defaultdict
import enum
import orjson
from tqdm import tqdm
import pandas as pd

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
# - Only add word to vocab IF in more than 2 sentences
# - Remove "X" Pos tagging from vocab, and "PROPN" (nom propre)
# - Remove all acronyms from vocab
# - If noun, plural, and finishes by "s", add singular word to vocab
# - Remove URLs
# - Use Wikiperdia corpora as well


# See https://universaldependencies.org/u/pos/ and
# https://universaldependencies.org/format.html#morphological-annotation for POS and morph info
vocab = {w: defaultdict(int) for w in set(pd.read_excel('lexicon.xls')["item"])}
encountered_vocab = set()
# https://github.com/Common-Voice/commonvoice-fr/issues/91
# https://wortschatz.uni-leipzig.de/en/download/French
doc_path = './fra_mixed_2009_100K-sentences.txt'
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
    return out_morph

mappings = defaultdict(int)
regex_id = re.compile('[0-9]+\W+')
regex_apostrophes = re.compile('(\W[ldnmst])\W+(\w)')

print('Tagging corpus...')
with open(doc_path) as file_in:
    for index, line in tqdm(enumerate(file_in), total=100000):
        line = regex_id.sub('', line)
        line = regex_apostrophes.sub(r"\1'\2", line)
        doc = nlp(line)
        for index in range(len(doc) - 1):
            token = doc[index]
            word = token.text
            encountered_vocab.add(word)
            if word not in vocab:
                vocab[word] = defaultdict(int)
            next_token = doc[index + 1]
            token_pos = token.pos_
            next_token_pos = next_token.pos_
            if token_pos == 'PUNCT' or next_token_pos == 'PUNCT':
                continue
            token_morph = token.morph.to_dict()
            next_token_morph = next_token.morph.to_dict()
            tup = (token_pos, token.morph, next_token_pos, next_token.morph)
            mappings[tup] += 1
            vocab[word][(token_pos, str(token.morph),)] += 1

# pos_morph = (token_pos, str(token.morph),)
#             if token not in vocab:
#                 vocab[token] = {}
#             if pos_morph not in vocab[token]:
#                 vocab[token][pos_morph] = 0
#             vocab[token][pos_morph] += 1

with open('tagging_stats_test.jsonl', 'w') as f:
    for tag, nb in sorted(mappings.items(), key= lambda x: x[1], reverse=True):
        tag = (tag[0], keep_interesting_morph(tag[1]), tag[2], keep_interesting_morph(tag[3]))
        obj = {'tagging': tag, 'nb': nb}
        # s = orjson.dumps(obj).encode('utf-8')
        s = orjson.dumps(obj)
        s = s.decode() + '\n'
        f.write(s)

final_vocab = []

remaining_words = set(vocab.keys()) - encountered_vocab
print(f'Tagging {len(remaining_words)} remaining words in vocab...')
for word in tqdm(remaining_words):
    token = nlp(word)[0]
    final_vocab.append({
        'word': word,
        'pos': token.pos_,
        'morph': token.morph.to_dict(),
    })

for word in encountered_vocab:
    stats = vocab[word]
    if len(stats) == 0:
        final_vocab.append({'word': word, 'pos': '', 'morph': {}})
        continue
    sorted_by_occ = sorted(stats.items(), key= lambda x: x[1], reverse=True)
    first_pos_morph = sorted_by_occ[0][0]
    final_vocab.append({'word': word, 'pos': first_pos_morph[0], 'morph': first_pos_morph[1]})

final_vocab = sorted(final_vocab, key=lambda x: (len(x['word']), x['word'],))
write_jsonl('vocab.jsonl', final_vocab)