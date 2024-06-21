import spacy
import re
from collections import defaultdict
import enum
import orjson
from tqdm import tqdm
import pandas as pd
from frozendict import frozendict
from itertools import islice


def write_jsonl(path, words):
    with open(path, "w") as f:
        for word in words:
            s = orjson.dumps(word)
            s = s.decode() + "\n"
            f.write(s)


# 1. Load base vocab
# Iterate large corpus
# 2. Do POS tagging on this corpus. For each word:
# - If not in base vocab AND not a noun, add it to vocab
# - Record POS tag and morphology for this word
# - Add POS tag + morphology to a count dict

# TODO:
# - harmoniser les stats
# - enlever mots pareils si on leur enleve leur accent ()

# https://github.com/chrplr/openlexicon/blob/master/datasets-info/Liste-de-mots-francais-Gutenberg/README-liste-francais-Gutenberg.md
blacklist = set(pd.read_csv("blacklist.csv")["word"])

words_reference = set(pd.read_csv("fr_words.csv")["word"]) - blacklist


lexique_pos_mapping = {
    "ADJ": "ADJ",
    "NOM": "NOUN",
    "VER": "VERB",
    "PRO": "PRON",
    "ADV": "ADV",
    "AUX": "AUX",
    "PRE": "ADP",
    "ONO": "ADV",
    "CON": "CCONJ",
}
def map_lexique_pos(lexique_pos):
    if lexique_pos in lexique_pos_mapping:
        return lexique_pos_mapping[lexique_pos]
    if pd.isna(lexique_pos):
        return "ADV"
    if "PRO" in lexique_pos:
        return "PRON"
    if "ART" in lexique_pos:
        return "DET"
    return "ADV"

def lexique_row_to_pos_morph_tuple(row):
    pos = map_lexique_pos(row['cgram'])
    morph = {}
    gender = row.get('gender')
    number = row.get('nombre')
    info = row.get('infover')
    if gender is not None and not pd.isna(gender):
        morph['Gender'] = lexique_gender_number_mapping[gender]
    if number is not None and not pd.isna(number):
        morph['Number'] = lexique_gender_number_mapping[number]
    if info is not None and not pd.isna(info):
        if '3' in info:
            morph['Person'] = '3'
        elif '2' in info:
            morph['Person'] = '2'
        elif '1' in info:
            morph['Person'] = '1'
    return (pos, frozendict(morph))
        
    
lexique_gender_number_mapping = {
    'f': "Fem",
    'm': 'Masc',
    's': 'Sing',
    'p': 'Plur'
}

lexique = pd.read_csv("Lexique383.tsv", sep="\t", usecols = ['ortho', 'cgram', 'genre', 'nombre', 'freqlemlivres', 'infover'])
lexique = lexique[lexique['freqlemlivres'] > 2]
lexique['tagging'] = lexique.apply(lexique_row_to_pos_morph_tuple, axis=1)
vocab = {w: defaultdict(int) for w in set(lexique['ortho'])}
for word in blacklist:
    if word in vocab:
        del vocab[word]

static_pos_tag = lexique.groupby('ortho').agg(list).reset_index().set_index('ortho').to_dict('index')
del lexique
    
encountered_vocab = set()
# https://github.com/Common-Voice/commonvoice-fr/issues/91
# https://wortschatz.uni-leipzig.de/en/download/French
doc_paths = [
    "./fra_mixed_2009_100K-sentences.txt",
    "./fra_wikipedia_2021_100K-sentences.txt",
]
spacy.prefer_gpu()
# nlp = spacy.load("fr_dep_news_trf")
nlp = spacy.load("fr_core_news_sm", disable=["lemmatizer", "ner"])

print(nlp.pipe_names)


class Morph(enum.IntEnum):
    Gender = 1  # Masc, Fem
    Number = 2  # 'Sing', 'Plur'
    Person = 3  # '3', '1'...


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
        if biggest[key] != value:
            return None
    return biggest


mappings = defaultdict(int)
regex_id = re.compile("[0-9]+\W+")
regex_apostrophes = re.compile("(\W[ldnmst])\W+(\w)")

new_words = defaultdict(int)
print("Tagging corpus...")


def print_doc(doc):
    for token in doc:
        print(token.text, token.pos_, token.dep_)


pos_n_grams = defaultdict(int)
batch_size = 1000
for doc_path in doc_paths:
    processed = 0
    print(f"Tagging {doc_path}")
    with open(doc_path) as file_in:
        while True:
            next_n_lines = list(islice(file_in, batch_size))
            processed += batch_size
            # if processed > 2000:
            #     break
            if processed % 2000 == 0:
                print(f"{processed} lines processed {processed/1000}%")
            if not next_n_lines:
                break
            for i, line in enumerate(next_n_lines):
                next_n_lines[i] = regex_id.sub("", line)
            for doc in nlp.pipe(next_n_lines):
                for index in range(len(doc) - 1):
                    token = doc[index]
                    word = token.text.lower()
                    if word not in words_reference:
                        continue
                    next_token = doc[index + 1]
                    token_pos = token.pos_
                    if token_pos == "PUNCT" or token_pos == "PROPN":
                        continue
                    token_morph = token.morph.to_dict()
                    if word not in vocab:
                        new_words[word] = 1
                        vocab[word] = defaultdict(int)
                    elif word in new_words:
                        new_words[word] += 1
                    encountered_vocab.add(word)
                    vocab[word][
                        (
                            token_pos,
                            frozendict(token_morph),
                        )
                    ] += 1
                    next_token_pos = next_token.pos_
                    if next_token_pos == "PUNCT":
                        continue
                    tup = (
                        token_pos,
                        keep_interesting_morph(token.morph),
                        next_token_pos,
                        keep_interesting_morph(next_token.morph),
                    )
                    pos_n_grams[(token_pos, next_token_pos)] += 1
                    if index < len(doc) - 2:
                        pos_n_grams[
                            (token_pos, next_token_pos, doc[index + 2].pos_)
                        ] += 1
                    if index < len(doc) - 3:
                        pos_n_grams[
                            (
                                token_pos,
                                next_token_pos,
                                doc[index + 2].pos_,
                                doc[index + 3].pos_,
                            )
                        ] += 1
                    mappings[tup] += 1


with open("stats1.txt", "w") as f:
    for word, occurence in sorted(new_words.items(), key=lambda x: x[1], reverse=True):
        f.write(f"{word}: {occurence}\n")
    f.write("Encountered:\n")
    for w in encountered_vocab:
        f.write(f"{w}\n")


for word, occurence in new_words.items():
    if occurence <= 3:
        del vocab[word]
        encountered_vocab.remove(word)

with open("pos_n_grams.jsonl", "w") as f:
    for t, nb in sorted(pos_n_grams.items(), key=lambda x: x[1], reverse=True):
        obj = {"pos": t, "occ": nb}
        s = orjson.dumps(obj).decode()
        if "SPACE" in s:
            continue
        f.write(f"{s}\n")

with open("tagging_stats.jsonl", "w") as f:
    for tag, nb in sorted(mappings.items(), key=lambda x: x[1], reverse=True):
        tag = (tag[0], tag[1], tag[2], tag[3])
        obj = {"tagging": tag, "nb": nb}
        s = orjson.dumps(obj)
        s = s.decode() + "\n"
        f.write(s)

final_vocab = []

remaining_words = set(vocab.keys()) - encountered_vocab

print(f"{len(remaining_words)} remaining words in vocab")
for index, word in tqdm(enumerate(remaining_words)):
    if word not in static_pos_tag:
        continue
    final_vocab.append(
        {
            "word": word,
            "pos": static_pos_tag[word]['tagging'][0][0],
            "morph": tuple(map(lambda x: x[1], static_pos_tag[word]['tagging']))
        }
    )

with open("stats_final_remaining.txt", "w") as f:
    for v in final_vocab:
        f.write(f"{v}\n")

with open("dump_vocab.txt", "w") as f:
    for word, tuple_stats in vocab.items():
        f.write(f"WORD {word}:\n")
        for t, occ in tuple_stats.items():
            f.write(f"{t}: {occ}\n")

for word in encountered_vocab:
    stats = vocab[word]
    if len(stats) == 0:
        final_vocab.append({"word": word, "pos": "", "morph": {}})
        continue
    sorted_by_occ = sorted(stats.items(), key=lambda x: x[1], reverse=True)
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
    for m in morph_to_keep:
        gender = m.get('Gender')
        if gender is not None and first_pos_morph[0] == "VERB":
            del m['Gender']
    to_add = {"word": word, "pos": first_pos_morph[0], "morph": morph_to_keep}
    final_vocab.append(to_add)

alpha_diacritic_regex = re.compile(r"[^A-Za-z_À-ÿ]")


def should_keep_word(vocab_item):
    should_keep = _should_keep_word(vocab_item)
    # if not should_keep:
    #     print(f'{vocab_item["word"]} removed')
    return should_keep


def _should_keep_word(vocab_item):
    # filter out PROPN and X and unidentified
    if (
        vocab_item["pos"] == "PROPN"
        or vocab_item["pos"] == "X"
        or vocab_item["pos"] == ""
    ):
        return False
    word = vocab_item["word"]
    if len(word) < 2:
        return False
    if alpha_diacritic_regex.search(word) is not None:
        return False
    return True


final_vocab = filter(should_keep_word, final_vocab)
final_vocab = sorted(
    final_vocab,
    key=lambda x: (
        len(x["word"]),
        x["word"],
    ),
)
write_jsonl("vocab.jsonl", final_vocab)
