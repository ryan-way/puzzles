#![feature(test)]
#![feature(iter_array_chunks)]

extern crate rayon;
extern crate test;

use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;

use indicatif::ProgressBar;
use rayon::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Color {
    GRAY,   // Don't know if the word contains this letter, initial state
    BLACK,  // The word does not contain this letter
    YELLOW, // The word does contain this letter
    GREEN,  // The position of this letter is known in the word
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Clues([Color; 5]);

impl FromStr for Clues {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Clues(
            s.chars()
                .array_chunks::<5>()
                .take(1)
                .map(|window: [char; 5]| {
                    window.map(|c| match c {
                        'b' => Color::BLACK,
                        'y' => Color::YELLOW,
                        'g' => Color::GREEN,
                        _ => panic!("Unsupported color {}", c),
                    })
                })
                .next()
                .unwrap(),
        ))
    }
}

#[derive(Debug)]
pub struct Bitmask(usize);

impl Bitmask {
    pub fn new() -> Self {
        Bitmask(0)
    }

    pub fn add(&mut self, value: usize) {
        self.0 |= 1 << value;
    }

    pub fn has(&self, value: usize) -> bool {
        (self.0 & 1 << value) > 0
    }

    pub fn remove(&mut self, value: usize) {
        if self.has(value) {
            self.0 ^= 1 << value;
        }
    }

    pub fn intersection(&self, other: &Bitmask) -> Bitmask {
        Bitmask(self.0 & other.0)
    }

    pub fn symmetric_difference(&self, other: &Bitmask) -> Bitmask {
        Bitmask((self.0 & other.0) ^ (self.0 | other.0))
    }

    pub fn values(&self) -> impl Iterator<Item = usize> {
        let value = self.0;
        (0..64).filter(move |idx| value & (1 << idx) > 0)
    }
}

impl Default for Bitmask {
    fn default() -> Self {
        Bitmask::new()
    }
}

pub struct WordClues<'a> {
    clues: Clues,
    word: &'a str,
}

impl<'a> WordClues<'a> {
    fn from_clues(word: &'a str, clues: Clues) -> Self {
        WordClues { word, clues }
    }

    fn from_solution(word: &'a str, solution: &str) -> Self {
        let mut occurrence: HashMap<char, usize> = HashMap::with_capacity(5);
        for c in solution.chars() {
            occurrence.insert(c, solution.chars().filter(|&a| a == c).count());
        }

        let mut colors: [Color; 5] = [Color::BLACK; 5];

        word
            .chars()
            .zip(solution.chars())
            .enumerate()
            .filter(|(_, (a, b))| a == b)
            .for_each(|(idx, (_, b))| {
                if let Some(value) = occurrence.get(&b) {
                    occurrence.insert(b, value - 1);
                }
                colors[idx] = Color::GREEN;
            });

        for (idx, c) in word.chars().enumerate() {
            if let Some(value) = occurrence.get(&c) {
                if *value > 0 && colors[idx] != Color::GREEN {
                    occurrence.insert(c, value - 1);
                    colors[idx] = Color::YELLOW;
                }
            }
        }

        let clues = Clues(colors);

        WordClues { clues, word }
    }

    fn get_colors(&self) -> &Clues {
        &self.clues
    }
}

impl<'a> From<WordClues<'a>> for Clues {
    fn from(value: WordClues<'a>) -> Self {
        value.clues
    }
}

pub struct WordSuggestor<'a> {
    word_bank: Vec<&'a str>,
    word_clues: Vec<&'a WordClues<'a>>,
}

impl<'a> WordSuggestor<'a> {
    pub fn new(word_bank: Vec<&'a str>) -> Self {
        WordSuggestor {
            word_bank,
            word_clues: vec![],
        }
    }
    pub fn suggest_word<T>(&self, ranker: &T, show_progress: bool) -> String
    where
        T: Ranker,
    {
        // if self.word_clues.len() == 0 {
        //     return "serai".to_owned();
        // }
        println!("Calculating possible solutions");
        let possible_solutions: Vec<&&str> = self
            .word_bank
            .iter()
            .filter(|&solution| {
                self.word_clues.iter().all(|clue| {
                    WordClues::from_solution(clue.word, solution).get_colors() == clue.get_colors()
                })
            })
            .collect();
        println!("Number of possible solutions: {}", possible_solutions.len());

        if possible_solutions.is_empty() {
            return "".to_owned();
        }

        if possible_solutions.len() == 1 {
            return possible_solutions.first().unwrap().to_string();
        }

        println!("Calculating suggestion");
        let progress_bar = if show_progress {
            ProgressBar::new(self.word_bank.len() as u64)
        } else {
            ProgressBar::hidden()
        };
        let suggestion = self
            .word_bank
            .par_iter()
            .max_by_key(|&word| {
                progress_bar.inc(1);
                ranker.rank(&possible_solutions, word)
            })
            .unwrap();

        suggestion.to_string()
    }

    pub fn add_clue(&mut self, word_clue: &'a WordClues<'a>) {
        self.word_clues.push(word_clue);
    }
}

pub trait Ranker: Sync + Send {
    fn rank(&self, possible_solutions: &[&&str], word: &str) -> usize;
}

pub struct LowestMaxBucketRanker;

impl LowestMaxBucketRanker {
    pub fn new() -> Self {
        LowestMaxBucketRanker {}
    }
}

impl Default for LowestMaxBucketRanker {
    fn default() -> Self {
        Self::new()
    }
}

impl Ranker for LowestMaxBucketRanker {
    fn rank(&self, possible_solutions: &[&&str], word: &str) -> usize {
        let mut map = HashMap::<Clues, usize>::new();
        possible_solutions.iter().for_each(|solution| {
            let word_clues = WordClues::from_solution(word, solution);
            *map.entry(word_clues.into()).or_default() += 1;
        });
        possible_solutions.len() - map.values().max().unwrap()
    }
}

pub struct LargestUniqueValuesRanker;

impl LargestUniqueValuesRanker {
    pub fn new() -> Self {
        LargestUniqueValuesRanker {}
    }
}

impl Default for LargestUniqueValuesRanker {
    fn default() -> Self {
        Self::new()
    }
}

impl Ranker for LargestUniqueValuesRanker {
    fn rank(&self, possible_solutions: &[&&str], word: &str) -> usize {
        possible_solutions
            .iter()
            .map(|solution| WordClues::from_solution(word, solution).into())
            .collect::<HashSet<Clues>>()
            .len()
    }
}
fn main() {
    let words: Vec<&str> = include_str!("../word_bank.txt")
        .lines()
        .collect();

    println!("created word bank");
    let mut word_suggestor = WordSuggestor::new(words);
    let processors: Vec<&str> = include_str!("../clues.txt")
        .lines()
        .map(|s| {
            let mut split = s.split(" ");
            split.next().unwrap()
        })
        .collect();
    let clues: Vec<Clues> = include_str!("../clues.txt")
        .lines()
        .map(|s| {
            let mut split = s.split(" ");
            split.next();
            split.next().unwrap().parse().unwrap()
        })
        .collect();

    let word_clues: Vec<WordClues> = processors
        .iter()
        .zip(clues.into_iter())
        .map(|(processor, clues)| WordClues::from_clues(processor, clues))
        .collect();

    for word_clue in &word_clues {
        word_suggestor.add_clue(word_clue);
    }

    println!(
        "Suggestion: {}",
        word_suggestor.suggest_word(&LargestUniqueValuesRanker::new(), true)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    mod bitmask {
        use super::*;

        #[test]
        fn test_init() {
            let mask = Bitmask::new();
            assert_eq!(mask.0, 0);
        }

        #[test]
        fn test_add() {
            let mut mask = Bitmask::new();
            mask.add(0);
            assert_eq!(mask.0, 1);

            mask.add(2);
            assert_eq!(mask.0, 5);
        }

        #[test]
        fn test_remove() {
            let mut mask = Bitmask::new();
            mask.add(3);
            assert_eq!(mask.0, 8);

            mask.remove(3);
            assert_eq!(mask.0, 0);
        }

        #[test]
        fn test_muli_add() {
            let mut mask = Bitmask::new();
            mask.add(3);
            assert_eq!(mask.0, 8);

            mask.add(3);
            assert_eq!(mask.0, 8);
        }

        #[test]
        fn test_muli_remove() {
            let mut mask = Bitmask::new();
            mask.add(3);
            assert_eq!(mask.0, 8);

            mask.remove(3);
            assert_eq!(mask.0, 0);

            mask.remove(3);
            assert_eq!(mask.0, 0);
        }

        #[test]
        fn test_values() {
            let mut mask = Bitmask::new();
            mask.add(3);
            mask.add(8);

            let values: Vec<usize> = mask.values().collect();
            println!("Values: {:?}", values);
            assert!(values.contains(&3));
            assert!(values.contains(&8));
        }

        #[test]
        fn test_intersection() {
            let mut first = Bitmask::new();
            first.add(1);
            first.add(2);
            first.add(5);
            first.add(7);

            let mut second = Bitmask::new();
            second.add(2);
            second.add(5);
            second.add(6);
            second.add(8);

            let intersection = first.intersection(&second);

            assert!(intersection.has(2));
            assert!(intersection.has(5));
        }

        #[test]
        fn test_difference() {
            let mut first = Bitmask::new();
            first.add(1);
            first.add(2);
            first.add(5);
            first.add(7);

            let mut second = Bitmask::new();
            second.add(2);
            second.add(5);
            second.add(6);
            second.add(8);

            let intersection = first.symmetric_difference(&second);

            assert!(intersection.has(1));
            assert!(intersection.has(6));
            assert!(intersection.has(7));
            assert!(intersection.has(8));
        }
    }

    #[test]
    fn test_colors() {
        assert_eq!(
            *WordClues::from_solution("saber", "label")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution("aheap", &"woken")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::YELLOW,
                Color::BLACK,
                Color::BLACK
            ])
        );

        assert_eq!(
            *WordClues::from_solution("serai", &"delve")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution("yente", "delve")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution("blech", "delve")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::YELLOW,
                Color::YELLOW,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution("begem", "delve")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::YELLOW,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution("welke", "delve")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution("mommy", "delve")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK
            ])
        );

        assert_eq!(
            *WordClues::from_solution("forge", "forge")
                .get_colors(),
            Clues([Color::GREEN; 5])
        );
        assert_eq!(
            *WordClues::from_solution("forte", "forge")
                .get_colors(),
            Clues([
                Color::GREEN,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution("forze", "forge")
                .get_colors(),
            Clues([
                Color::GREEN,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution("bafts", "forge")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::YELLOW,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution("murid", "forge")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution("soare", "forge")
                .get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::YELLOW,
                Color::GREEN
            ])
        );
    }

    #[bench]
    fn bench_unique_ranker(b: &mut Bencher) {
        let words: Vec<&str> = include_str!("../word_bank.txt")
            .lines()
            .collect();
        let possible_solutions: Vec<&&str> = words.iter().collect();
        let ranker = LargestUniqueValuesRanker::new();
        b.iter(|| ranker.rank(&possible_solutions, &words[0]));
    }

    #[bench]
    fn bench_lowest_ranker(b: &mut Bencher) {
        let words: Vec<&str> = include_str!("../word_bank.txt")
            .lines()
            .collect();
        let possible_solutions: Vec<&&str> = words.iter().collect();
        let ranker = LowestMaxBucketRanker::new();
        b.iter(|| ranker.rank(&possible_solutions, &words[0]));
    }

    #[bench]
    fn bench_clue_creation(b: &mut Bencher) {
        let first = "vixon";
        let second = "apple";

        b.iter(|| WordClues::from_solution(&first, &second));
    }

    #[bench]
    fn hashing_baseline(b: &mut Bencher) {
        let mut map: HashMap<char, Bitmask> = HashMap::with_capacity(0);
        b.iter(|| {
            map.entry('c').or_default().add(1);
        });
    }

    #[bench]
    fn bench_filter_word_bank(b: &mut Bencher) {
        let word_bank: Vec<&str> = vec!["abaci", "ocuby", "thowt"];
        let word_clues: Vec<WordClues> = vec![];

        b.iter(|| {
            word_bank
                .iter()
                .filter(|solution| {
                    word_clues.iter().all(|clue| {
                        WordClues::from_solution(clue.word, solution).get_colors()
                            == clue.get_colors()
                    })
                })
                .collect::<Vec<&&str>>()
        });
    }

    #[bench]
    fn bench_word_suggestor(b: &mut Bencher) {
        let word_bank: Vec<&str> = vec!["abaci", "ocuby", "thowt"];

        let word_suggestor = WordSuggestor::new(word_bank);
        let ranker = LowestMaxBucketRanker::new();
        b.iter(|| word_suggestor.suggest_word(&ranker, false));
    }
}
