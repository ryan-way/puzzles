#![feature(test)]

extern crate entity;
extern crate test;
extern crate rayon;

use std::collections::HashMap;
use std::collections::HashSet;

use entity::word;
use entity::prelude::*;
use rayon::prelude::*;
use indicatif::ProgressBar;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::ExprTrait;
use sea_orm::sea_query::Func;
use sea_orm::{EntityTrait, QueryFilter};


#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Color {
    GRAY,   // Don't know if the word contains this letter, initial state
    BLACK,  // The word does not contain this letter
    YELLOW, // The word does contain this letter
    GREEN,  // The position of this letter is known in the word
}

pub struct WordProcessor {
    map: HashMap<char, HashSet<usize>>,
}

impl WordProcessor {
    fn new(word: &String) -> Self {
        let mut map: HashMap<char, HashSet<usize>> = HashMap::new();
        word.chars().enumerate().for_each(|(idx, c)| {
            map.entry(c).or_default().insert(idx);
        });
        return WordProcessor {
            map,
        }
    }

    fn get(&self, c: char) -> Option<&HashSet<usize>> {
        self.map.get(&c)
    }

    fn entries(&self) -> impl Iterator<Item = (&char, &HashSet<usize>)> {
        self.map.iter()
    }
}

pub struct Clue<'a> {
    colors: [Color; 5],
    word: &'a String,
}

impl<'a> Clue<'a> {
    fn from_colors(word: &'a String, colors: [Color; 5]) -> Self {
        Clue {
            word,
            colors,
        }
    }

    fn from_solution(word: &'a String, solution: &String) -> Self {
        let mut map = HashMap::new();

        let word_processor = WordProcessor::new(word);
        let solution_processor = WordProcessor::new(solution);

        word_processor.entries().for_each(|(&key, word_set)| {
            if let Some(solution_set) = solution_processor.get(key) {
                word_set.intersection(solution_set).for_each(|value| {
                    map.insert(value, Color::GREEN);
                });

                word_set.symmetric_difference(solution_set).for_each(|value| {
                    map.insert(value, Color::YELLOW);
                })
            } else {
                word_set.iter().for_each(|value| {
                    map.insert(value, Color::BLACK);
                });
            }
        });
        
        let mut colors: [Color; 5] = [Color::BLACK; 5];

        map.iter().for_each(|(&&key, &value)| {
            colors[key] = value;
        });

        Clue {
            colors,
            word,
        }
    }

    fn get_colors(&self) -> &[Color; 5] {
        &self.colors
    }
}

impl<'a> Into<[Color;5]> for Clue<'a> {
    fn into(self) -> [Color;5] {
        self.colors
    }
}


pub struct WordSuggestor<'a> {
    word_bank: Vec<String>,
    clues: Vec<Clue<'a>>,
}


impl<'a> WordSuggestor<'a> {
    pub fn new(word_bank: Vec<String>) -> Self {
        WordSuggestor { word_bank, clues: vec![] }
    }
    pub fn suggest_word(&self) -> String {
        println!("Calculating possible solutions");
        let progress_bar = ProgressBar::new(self.word_bank.len() as u64);
        let possible_solutions: Vec<&String> = self.word_bank.iter()
            .filter(|solution|{
                progress_bar.inc(1);
                self.clues.iter().all(|clue| 
                    Clue::from_solution(clue.word, solution).get_colors() == clue.get_colors()
                )
            })
            .collect();
        progress_bar.finish_with_message("Done!");
        println!("Number of possible solutions: {}", possible_solutions.len());

        println!("Calculating suggestion");
        let progress_bar = ProgressBar::new(self.word_bank.len() as u64);
        self.word_bank.iter().max_by_key(|&word| {
            progress_bar.inc(1);
            possible_solutions.iter()
                .map(|solution| Clue::from_solution(word, solution).into())
                .collect::<HashSet<[Color; 5]>>()
                .len()
        }).unwrap()
        .to_owned()
        .to_owned()
    }

    pub fn add_clue(&mut self, clue: Clue<'a>) {
        self.clues.push(clue);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let db = entity::get_connection().await?;

    let models = Word::find()
        .filter(Func::char_length(Expr::col(word::Column::Text))
            .eq(5))
        .all(&db)
        .await?;

    let words = models.into_iter()
        .map(|model| model.text)
        .collect::<Vec<String>>();
    println!("created word bank");
    let word_suggestor = WordSuggestor::new(words);

    println!("Suggestion: {}", word_suggestor.suggest_word());

    Ok(())
}

#[cfg(test)]
mod tests {
}
