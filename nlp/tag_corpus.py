import spacy
# from spacy.lang.fr.examples import sentences 
import re
from collections import defaultdict
import enum
import json
from tqdm import tqdm
# https://github.com/Common-Voice/commonvoice-fr/issues/91
# https://wortschatz.uni-leipzig.de/en/download/French
doc_path = './fra_mixed_2009_30K-sentences.txt'
# vocab = list(spacy.load("fr_core_news_sm").vocab.strings)
nlp = spacy.load("fr_dep_news_trf")

class Morph(enum.IntEnum):
    Gender = 1 # Masc, Fem
    Number = 2 # 'Sing', 'Plur'
    Person = 3 # '3', '1'...

mappings = defaultdict(int)
regex_id = re.compile('[0-9]+\W+')
regex_apostrophes = re.compile('(\W[ldnmst])\W+(\w)')
nb_line = 0
with open(doc_path) as file_in:
    for index, line in tqdm(enumerate(file_in)):
        if index > 100:
            break
        line = regex_id.sub('', line)
        line = regex_apostrophes.sub(r"\1'\2", line)
        doc = nlp(line)
        for index in range(len(doc) - 1):
            token = doc[index]
            next_token = doc[index + 1]
            token_pos = token.pos_
            next_token_pos = next_token.pos_
            if token_pos == 'PUNCT' or next_token_pos == 'PUNCT':
                continue
            # mappings[(token_pos, next_token_pos,)] += 1
            token_morph = token.morph.to_dict()
            # print(token, token_pos)
            next_token_morph = next_token.morph.to_dict()
            tup = (token_pos, token.morph, next_token_pos, next_token.morph)
            mappings[tup] += 1
            # for morph in list(Morph):
            #     key = morph.name
            #     key_id = str(morph.value)
            #     if key in token_morph:
            #         tup = (token_pos, key_id + token_morph[key], next_token_pos, key_id + str(next_token_morph.get(key)))
            #         mappings[tup] += 1

def keep_interesting_morph(input_morph):
    out_morph = {}
    for morph in list(Morph):
        key = morph.name
        m = input_morph.get(key)
        if len(m):
            out_morph[key] = m[0]
    return out_morph


with open('tagging_stats_test.jsonl', 'w') as f:
    for tag, nb in sorted(mappings.items(), key= lambda x: x[1], reverse=True):
        # for morph in list(Morph):
            #     key = morph.name
            #     key_id = str(morph.value)
        tag = (tag[0], keep_interesting_morph(tag[1]), tag[2], keep_interesting_morph(tag[3]))

        obj = {'tagging': tag, 'nb': nb}
        s = json.dumps(obj).encode('utf-8')
        s = s.decode() + '\n'
        f.write(s)
