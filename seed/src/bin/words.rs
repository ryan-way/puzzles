#![feature(iter_array_chunks)]
extern crate entity;
extern crate indicatif;

use entity::{prelude::*, word::ActiveModel};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use indicatif::ProgressBar;

const CHUNK_SIZE: usize = 5000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let words: Vec<entity::word::ActiveModel> = reqwest::get("https://raw.githubusercontent.com/dwyl/english-words/refs/heads/master/words_alpha.txt")
    .await?
    .text()
    .await?
    .lines()
    .map(|line| ActiveModel {
      text: Set(line.to_owned()),
      ..Default::default()
    })
    .collect();

  let db: DatabaseConnection = entity::get_connection().await?;

  let remainder = words.len() - (words.len() % CHUNK_SIZE);
  let remainder: Vec<ActiveModel> = words.iter().skip(remainder).cloned().collect();

  println!("Processing...");
  let pb = ProgressBar::new(words.len() as u64);
  for batch in words.into_iter().array_chunks::<CHUNK_SIZE>() {
    pb.inc(CHUNK_SIZE as u64);
    Word::insert_many(batch).exec(&db).await?;
  }

  if !remainder.is_empty() {
    let size = remainder.len();
    Word::insert_many(remainder).exec(&db).await?;
    pb.inc(size as u64);
  }

  pb.finish_and_clear();

  println!("Done!");

  Ok(())
}
