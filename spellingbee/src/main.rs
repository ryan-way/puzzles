use std::{collections::HashSet, str::FromStr};

struct LetterBank {
    required: HashSet<char>,
    allowed: HashSet<char>,
}

impl FromStr for LetterBank {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let required: HashSet<char> = HashSet::from([s.chars().next().unwrap()]);
        let allowed: HashSet<char> = s.chars().collect();

        Ok(LetterBank { required, allowed })
    }
}

impl LetterBank {
    fn matches(&self, word: &str) -> bool {
        let hash: HashSet<char> = word.chars().collect();

        self.required.difference(&hash).count() == 0 && hash.difference(&self.allowed).count() == 0
    }
}

struct SpellingBeeSolver {
    letters: LetterBank,
    word_bank: Vec<&'static str>,
}

impl SpellingBeeSolver {
    fn new(letters: LetterBank, word_bank: Vec<&'static str>) -> Self {
        SpellingBeeSolver { letters, word_bank }
    }

    fn solve(&self) -> Vec<&'static str> {
        self.word_bank
            .iter()
            .filter(|word| self.letters.matches(word))
            .take(20)
            .cloned()
            .collect()
    }
}

fn main() {
    let mut word_bank: Vec<&'static str> = include_str!("word_bank.txt")
        .lines()
        .filter(|word| word.len() > 3)
        .collect();
    word_bank.sort_by(|a, b| a.len().cmp(&b.len()));
    let letters: LetterBank = include_str!("letters.txt").parse().unwrap();

    let solver = SpellingBeeSolver::new(letters, word_bank);
    let solution = solver.solve();

    println!("Solutions: {:?}", solution);
}
