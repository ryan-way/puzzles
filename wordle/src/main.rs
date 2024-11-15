#![feature(test)]
#![feature(iter_array_chunks)]

extern crate entity;
extern crate rayon;
extern crate test;

use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use std::usize;

use entity::prelude::*;
use entity::word;
use indicatif::ProgressBar;
use rayon::prelude::*;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::ExprTrait;
use sea_orm::sea_query::Func;
use sea_orm::{EntityTrait, QueryFilter};

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

pub struct WordProcessor {
    map: HashMap<char, Bitmask>,
}

impl WordProcessor {
    fn new(word: &String) -> Self {
        let mut map: HashMap<char, Bitmask> = HashMap::with_capacity(26);
        word.chars().enumerate().for_each(|(idx, c)| {
            map.entry(c).or_default().add(idx);
        });

        WordProcessor { map }
    }

    fn get(&self, c: char) -> Option<&Bitmask> {
        self.map.get(&c)
    }

    fn entries(&self) -> impl Iterator<Item = (&char, &Bitmask)> {
        self.map.iter()
    }
}

pub struct WordClues<'a> {
    clues: Clues,
    word: &'a String,
}

impl<'a> WordClues<'a> {
    fn from_clues(word: &'a String, clues: Clues) -> Self {
        WordClues { word, clues }
    }

    fn from_solution(word: &'a String, solution: &String) -> Self {
        let mut map: HashMap<usize, Color> = HashMap::with_capacity(5);

        let word_processor = WordProcessor::new(word);
        let solution_processor = WordProcessor::new(solution);

        word_processor.entries().for_each(|(&key, word_set)| {
            if let Some(solution_set) = solution_processor.get(key) {
                word_set
                    .intersection(solution_set)
                    .values()
                    .for_each(|value| {
                        map.insert(value, Color::GREEN);
                    });

                let max_yellows = solution_set
                    .values()
                    .filter(|&value| !word_set.has(value))
                    .count();
                let yellows: Vec<usize> = word_set
                    .values()
                    .filter(|value| !map.contains_key(value))
                    .take(max_yellows)
                    .collect();
                yellows.iter().for_each(|&value| {
                    map.insert(value, Color::YELLOW);
                })
            }
        });

        let mut colors: [Color; 5] = [Color::BLACK; 5];

        map.iter().for_each(|(&key, &value)| {
            colors[key] = value;
        });
        let clues = Clues(colors);

        WordClues { clues, word }
    }

    fn get_colors(&self) -> &Clues {
        &self.clues
    }
}

impl<'a> Into<Clues> for WordClues<'a> {
    fn into(self) -> Clues {
        self.clues
    }
}

pub struct WordSuggestor<'a> {
    word_bank: Vec<String>,
    word_clues: Vec<WordClues<'a>>,
}

impl<'a> WordSuggestor<'a> {
    pub fn new(word_bank: Vec<String>) -> Self {
        WordSuggestor {
            word_bank,
            word_clues: vec![],
        }
    }
    pub fn suggest_word<T>(&self, ranker: T) -> String
    where
        T: Ranker,
    {
        println!("Calculating possible solutions");
        let possible_solutions: Vec<&String> = self
            .word_bank
            .iter()
            .filter(|solution| {
                self.word_clues.iter().all(|clue| {
                    WordClues::from_solution(clue.word, solution).get_colors() == clue.get_colors()
                })
            })
            .collect();
        println!("Number of possible solutions: {}", possible_solutions.len());

        if possible_solutions.len() == 0 {
            return "".to_owned();
        }

        if possible_solutions.len() == 1 {
            return possible_solutions.first().unwrap().clone().clone();
        }

        println!("Calculating suggestion");
        let progress_bar = ProgressBar::new(self.word_bank.len() as u64);
        let suggestion = self
            .word_bank
            .iter()
            .max_by_key(|&word| {
                progress_bar.inc(1);
                ranker.rank(&possible_solutions, word)
            })
            .unwrap()
            .to_owned()
            .to_owned();

        // let score = ranker.rank(&possible_solutions, &suggestion);
        // println!("Suggestion {}, score: {}", suggestion, score);
        suggestion
    }

    pub fn add_clue(&mut self, word_clue: WordClues<'a>) {
        self.word_clues.push(word_clue);
    }
}

pub trait Ranker: Sync + Send {
    fn rank(&self, possible_solutions: &Vec<&String>, word: &String) -> usize;
}

pub struct LowestMaxBucketRanker;

impl LowestMaxBucketRanker {
    pub fn new() -> Self {
        LowestMaxBucketRanker {}
    }
}

impl Ranker for LowestMaxBucketRanker {
    fn rank(&self, possible_solutions: &Vec<&String>, word: &String) -> usize {
        let mut map = HashMap::<Clues, usize>::new();
        possible_solutions.len()
            - *possible_solutions
                .iter()
                .map(|solution| WordClues::from_solution(word, solution).into())
                .fold(&mut map, |acc, value| {
                    *acc.entry(value).or_default() += 1;
                    acc
                })
                .values()
                .max()
                .unwrap()
    }
}

pub struct LargestUniqueValuesRanker;

impl LargestUniqueValuesRanker {
    pub fn new() -> Self {
        LargestUniqueValuesRanker {}
    }
}

impl Ranker for LargestUniqueValuesRanker {
    fn rank(&self, possible_solutions: &Vec<&String>, word: &String) -> usize {
        possible_solutions
            .iter()
            .map(|solution| WordClues::from_solution(word, solution).into())
            .collect::<HashSet<Clues>>()
            .len()
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let db = entity::get_connection().await?;

    // let models = Word::find()
    //     .filter(Func::char_length(Expr::col(word::Column::Text))
    //         .eq(5))
    //     .all(&db)
    //     .await?;

    // let words = models.into_iter()
    //     .map(|model| model.text)
    //     .collect::<Vec<String>>();

    let words: Vec<String> = include_str!("word_bank.txt")
        .lines()
        .map(|s| s.to_owned())
        .collect();

    println!("created word bank");
    let mut word_suggestor = WordSuggestor::new(words);
    let clues: Vec<Vec<String>> = include_str!("clues.txt")
        .lines()
        .map(|s| s.split(" ").map(|value| value.to_owned()).collect())
        .collect();

    for clue in &clues {
        let word = &clue[0];

        let clues: Clues = clue[1].parse().unwrap();
        word_suggestor.add_clue(WordClues::from_clues(&word, clues));
    }

    println!(
        "Suggestion: {}",
        word_suggestor.suggest_word(LowestMaxBucketRanker::new())
    );

    Ok(())
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
            *WordClues::from_solution(&"saber".to_owned(), &"label".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"aheap".to_owned(), &"woken".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::YELLOW,
                Color::BLACK,
                Color::BLACK
            ])
        );

        assert_eq!(
            *WordClues::from_solution(&"serai".to_owned(), &"delve".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"yente".to_owned(), &"delve".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"blech".to_owned(), &"delve".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::YELLOW,
                Color::YELLOW,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"begem".to_owned(), &"delve".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::YELLOW,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"welke".to_owned(), &"delve".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"mommy".to_owned(), &"delve".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK
            ])
        );

        assert_eq!(
            *WordClues::from_solution(&"forge".to_owned(), &"forge".to_owned()).get_colors(),
            Clues([Color::GREEN; 5])
        );
        assert_eq!(
            *WordClues::from_solution(&"forte".to_owned(), &"forge".to_owned()).get_colors(),
            Clues([
                Color::GREEN,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"forze".to_owned(), &"forge".to_owned()).get_colors(),
            Clues([
                Color::GREEN,
                Color::GREEN,
                Color::GREEN,
                Color::BLACK,
                Color::GREEN
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"bafts".to_owned(), &"forge".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::YELLOW,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"murid".to_owned(), &"forge".to_owned()).get_colors(),
            Clues([
                Color::BLACK,
                Color::BLACK,
                Color::GREEN,
                Color::BLACK,
                Color::BLACK
            ])
        );
        assert_eq!(
            *WordClues::from_solution(&"soare".to_owned(), &"forge".to_owned()).get_colors(),
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
        let words: Vec<String> = include_str!("word_bank.txt")
            .lines()
            .map(|s| s.to_owned())
            .collect();
        let possible_solutions = words.iter().collect();
        let ranker = LargestUniqueValuesRanker::new();
        b.iter(|| ranker.rank(&possible_solutions, &words[0]));
    }

    #[bench]
    fn bench_lowest_ranker(b: &mut Bencher) {
        let words: Vec<String> = include_str!("word_bank.txt")
            .lines()
            .map(|s| s.to_owned())
            .collect();
        let possible_solutions = words.iter().collect();
        let ranker = LowestMaxBucketRanker::new();
        b.iter(|| ranker.rank(&possible_solutions, &words[0]));
    }

    #[bench]
    fn bench_clue_creation(b: &mut Bencher) {
        let first = "vixon".to_owned();
        let second = "apple".to_owned();

        b.iter(|| WordClues::from_solution(&first, &second));
    }

    #[bench]
    fn bench_word_processor(b: &mut Bencher) {
        let word = "vixon".to_owned();

        b.iter(|| WordProcessor::new(&word));
    }

    #[bench]
    fn bench_word_processor_hash_insertion(b: &mut Bencher) {
        let word = "vixon";
        b.iter(|| {
            let mut map: HashMap<char, Bitmask> = HashMap::with_capacity(26);
            word.chars().enumerate().fold(&mut map, |acc, (idx, c)| {
                acc.entry(c).or_default().add(idx);
                acc
            });
        });
    }

    #[bench]
    fn hashing_baseline(b: &mut Bencher) {
        let mut map: HashMap<char, Bitmask> = HashMap::with_capacity(0);
        b.iter(|| {
            map.entry('c').or_default().add(1);
        });
    }

    #[bench]
    fn bench_word_suggestor(b: &mut Bencher) {
        let word_bank = vec!["abaci", "ocuby", "thowt"]
            .iter()
            .map(|&value| value.to_owned())
            .collect::<Vec<String>>();

        let word_suggestor = WordSuggestor::new(word_bank);
        b.iter(|| word_suggestor.suggest_word(LowestMaxBucketRanker::new()));
    }
}
