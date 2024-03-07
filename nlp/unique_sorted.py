import json
from unidecode import unidecode

nb_original = 0
sorted_letters = set()

with open('words.jsonl', 'r') as f:
    for line in f:
        nb_original += 1
        sorted_letters.add(''.join(sorted(unidecode(json.loads(line)['word'].lower()))))

print(f'{nb_original} original words, {len(sorted_letters)} when sorted')