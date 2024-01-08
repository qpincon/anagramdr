import json, spacy
import pandas as pd
from tqdm import tqdm

def read_jsonl(path):
    words = []
    with open(path, 'r') as f:
        for line in f:
            words.append(json.loads(line))
    return words

def write_jsonl(path, words):
    with open(path, 'w') as f:
        for word in words:
            s = json.dumps(word, ensure_ascii=False).encode('utf-8')
            s = s.decode() + '\n'
            f.write(s)

# vocab = list(spacy.load("fr_core_news_sm").vocab.strings)
# The French Lexicon Project: https://osf.io/f8kc4/
vocab = set(pd.read_excel('lexicon.xls')["item"])
nlp = spacy.load("fr_dep_news_trf")
# print(nlp.pipe_names)
# other_pipes = [pipe for pipe in nlp.pipe_names if pipe not in [ "transformer", "parser", "tagger", "morphologizer", "attribute_ruler"]]
# nlp.disable_pipes(*other_pipes)

allowed_chars = set([char for char in "azertyuiopqsdfghjklmwxcvbn'àÀâÂäÄéèÈëÉîÎïôÔöÖçÇ ,-"])
one_letter_words_letters = set([c for c in 'aàyô'])
def is_allowed(word):
    if type(word) is not str: return False
    if len(word) == 1 and (word[0] not in one_letter_words_letters): return False
    # if word[0] == '-' or word[-1] == '-': return False
    # if len(word) > 19 and word.count('-') > 1 and any(x.isupper() for x in word): return False
    word = word.lower()
    if not all([char in allowed_chars for char in word]): return False
    count = 0
    for c in range(len(word)-1):
        if word[c] == word[c+1]:
            count += 1
            if count > 2: return False
        else: count = 0
    return True

print('vocab lenght before', len(vocab))
vocab = list(filter(is_allowed, vocab))
print('vocab lenght after', len(vocab))

words = []
for word in tqdm(sorted(vocab, key=lambda w: (len(w), w))):
    token = nlp(word)[0]
    words.append({
        'word': word,
        'pos': token.pos_,
        'morph': token.morph.to_dict(),
    })

def post_process(words):
    words = list(filter(lambda w: is_allowed(w['word']), words))
    words = sorted(words, key=lambda w: (len(w['word']), w['word']))
    return words

write_jsonl('words_v2.jsonl', words)