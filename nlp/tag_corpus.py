import spacy
# from spacy.lang.fr.examples import sentences 
import re
from collections import defaultdict
import enum
import json

doc_path = './fra_mixed_2009_30K-sentences.txt'
# vocab = list(spacy.load("fr_core_news_sm").vocab.strings)
nlp = spacy.load("fr_dep_news_trf")

class Morph(enum.IntEnum):
    Gender = 1
    Number = 2
    Person = 3

mappings = defaultdict(int)
regex_id = re.compile('[0-9]+\W+')
regex_apostrophes = re.compile('(\W[ldnmst])\W+(\w)')
nb_line = 0
with open(doc_path) as file_in:
    for line in file_in:
        nb_line += 1
        if nb_line % 200 == 0:
            print('Line {}'.format(nb_line))
        line = regex_id.sub('', line)
        line = regex_apostrophes.sub(r"\1'\2", line)
        doc = nlp(line)
        for index in range(len(doc) - 1):
            token = doc[index]
            next_token = doc[index + 1]
            token_pos = token.pos_
            next_token_pos = next_token.pos_
            mappings[(token_pos, next_token_pos,)] += 1
            token_morph = token.morph.to_dict()
            next_token_morph = next_token.morph.to_dict()
            for morph in list(Morph):
                key = morph.name
                key_id = str(morph.value)
                if key in token_morph:
                    tup = (token_pos, key_id + token_morph[key], next_token_pos, key_id + str(next_token_morph.get(key)))
                    mappings[tup] += 1

with open('tagging_stats.jsonl', 'w') as f:
    for tag, nb in mappings.items():
        obj = {'tagging': tag, 'nb': nb}
        s = json.dumps(obj).encode('utf-8')
        s = s.decode() + '\n'
        f.write(s)

# with open('words.json', 'w') as f:
#     json.dump(f, mappings)