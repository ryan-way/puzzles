# Wordle word suggester

## Use Cases
- Given:
  - Word bank

- Suggest a word to guess which will provide the optimal amount of new information
- Take additional clues as input from user based on a word that was guessed



## Word suggestion
- Maintain a list of potentional solutions. This list would be all the words from the word bank that match the known clues
- A word suggestion would a word chosen from the full word bank (not just solutions list) that is ranked based on how much information is gained by using it as a guess on average, across all the case where each word in the potential solutions list is the real solution. Ranking can be based on:
  - How many clues on average would be gained, weighted based clue color
  - How many potential solutions would be eliminated by guessing the word