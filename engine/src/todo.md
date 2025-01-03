- [x] Have max 128 matchables words, transform Matching::matched to u128
- [ ] Change letter_pool to non-vector and store eventual duplicate letters alongside
  - Tricky as we encode diacritics: store all used letters somewhere and change letter_pool to indexes to those letters?
- [x] Compute best permutations at the end
- [x] Use parallel processing for best permutations
- [x] Pick at random matchables words while length > 1000
- [x] Score less expressions with lots of low letters words

Design:
- [x] Renverser fleche pour fermer, ajouter "retour"
- [x] Ajouter color picker pour la couleur du gif
- [x] Gérer les erreurs côté back
- [x] Afficher erreur dans search bar quand > à un nombre de charactères
- [ ] Ajouter un endpoint pour l'export pour tracking
- [x] Ajouter page pour juste GifExporter
- [x] Ajouter un bouton "Encore!" avec tooltip qui explique pourquoi
- ~[ ] Garder le focus quand on lance une recherche pour pouvoir re-generer facilement~
- [x] Ajouter options avancées: recherche exacte, recherche avec un mot en particulier
- [x] Ajouter durée en seconde près du slider
- [x] Monter limite de lettres
- [x] Ajouter footer
- [x] Debugger majuscules dans expr initiale
- [x] Bouger liste de mots vers la gauche quand drawer ouvert
- [x] Export gif: ajouter route qui ouvre un blob?
- [x] Avoir un bouton "retour en haut"
- [x] Engine: mot à inclure feature
- [x] Ajouter loader
- [x] SEO (title et meta)
- [x] Améliorer favicon?
- [x] Setup nginx
- [x] Setup tacking (GoAccess)
- [x] nginx rate limiting
- [x] Lien vers outil d'export
- [x] Remettre back sur ordi
- [x] Message plus clair quand aucun anagramme trouvé
- [x] Validation input page principale (nombre + taille)
- [ ] Exclure un mot ?
- [x] Police + grosse sur les paramètres sur mobile
- [x] Share : encode URI

Baseline:

2366 matchable words
montceau les mines: 119.47ms
2715 matchable words
alain chabat le meilleur: 612.33ms
1086 matchable words
le marquis de sade: 364.65ms
286 matchable words
j'ai la belle vie madame: 391.53ms


Static "matched" size:

2366 matchable words
montceau les mines: 107.38ms
2715 matchable words
alain chabat le meilleur: 479.58ms
1086 matchable words
le marquis de sade: 350.23ms
286 matchable words
j'ai la belle vie madame: 386.82ms


Faster hashing algorithm:

Time to find best permutations: 8.22ms
montceau les mines: 97.19ms
2715 matchable words
Time to find best permutations: 23.27ms
alain chabat le meilleur: 414.09ms
1086 matchable words
Time to find best permutations: 77.54ms
le marquis de sade: 193.23ms
286 matchable words
Time to find best permutations: 114.57ms
j'ai la belle vie madame: 189.50ms


No more nested hash table:

2366 matchable words
Time to find best permutations: 7.27ms
montceau les mines: 98.24ms
2715 matchable words
Time to find best permutations: 19.84ms
alain chabat le meilleur: 430.39ms
1086 matchable words
Time to find best permutations: 63.64ms
le marquis de sade: 191.94ms
286 matchable words
Time to find best permutations: 90.19ms
j'ai la belle vie madame: 168.38ms


Rayon for finding best permutations:

2366 matchable words
Time to find best permutations: 3.15ms
montceau les mines: 87.98ms
2715 matchable words
Time to find best permutations: 5.08ms
alain chabat le meilleur: 397.71ms
1086 matchable words
Time to find best permutations: 11.85ms
le marquis de sade: 126.75ms
286 matchable words
Time to find best permutations: 10.99ms
j'ai la belle vie madame: 79.19ms


1000 matchables words:
Time to find best permutations: 15.01ms
Found 9999 anagrams
montceau les mines: 118.33ms
1000 matchable words
Time to find best permutations: 19.73ms
Found 9999 anagrams
alain chabat le meilleur: 360.06ms
1000 matchable words
Time to find best permutations: 15.90ms
Found 9999 anagrams
le marquis de sade: 394.80ms
286 matchable words
Time to find best permutations: 9.87ms
Found 9999 anagrams
j'ai la belle vie madame: 81.65ms