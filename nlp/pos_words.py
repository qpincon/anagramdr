import spacy
import json

def read_jsonl(path):
    mappings = {}
    with open(path, 'r') as f:
        for line in f:
            word_def = json.loads(line)
            word = list(word_def.keys())[0]
            attrs = list(word_def.values())[0]
            mappings[word] = attrs
    return mappings

def write_jsonl(path, words):
    with open(path, 'w') as f:
        for key, val in words.items():
            obj = {'word': key}
            obj.update(val)
            s = json.dumps(obj, ensure_ascii=False).encode('utf-8')
            s = s.decode() + '\n'
            f.write(s)

doc_path = './fra_mixed_2009_30K-sentences.txt'
vocab = list(spacy.load("fr_core_news_sm").vocab.strings)
nlp = spacy.load("fr_dep_news_trf")

allowed_chars = set([char for char in "azertyuiopqsdfghjklmwxcvbn'àâäéèëîïôö-ç"])

def is_allowed(word):
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

words = {}
for word in vocab:
    token = nlp(word)[0]
    words[token.text] = {
        'pos': token.pos_,
        'morph': token.morph.to_dict(),
    }

write_jsonl('words.jsonl', words)