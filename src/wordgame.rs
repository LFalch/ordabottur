use std::collections::HashMap;

use rand::prelude::*;
use rand::distributions::WeightedIndex;

use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::prelude::TypeMapKey;

/// Based on distributions on Wikipedia, please replace with others at some point.
const LETTER_WEIGHTS: [(char, u32); 29] = [
    ('a', 9_180),
    ('á', 1_240),
    ('b', 1_210),
    ('d', 2_240),
    ('ð', 2_660),
    ('e', 5_510),
    ('f', 1_930),
    ('g', 3_570),
    ('h', 1_900),
    ('i', 8_570),
    ('í', 1_710),
    ('j',   966),
    ('k', 3_150),
    ('l', 4_320),
    ('m', 3_780),
    ('n', 7_700),
    ('o', 3_060),
    ('ó', 1_010),
    ('p',   979),
    ('r', 8_890),
    ('s', 5_250),
    ('t', 5_890),
    ('u', 5_110),
    ('ú',   492),
    ('v', 3_100),
    ('y', 1_240),
    ('ý',   262),
    ('æ',   409),
    ('ø', 1_110),
];

fn weighted_index() -> WeightedIndex<u32> {
    WeightedIndex::new(LETTER_WEIGHTS.iter().map(|i| i.1)).unwrap()
}

/// Generate `n` random Faroese letters following a distribution of how common those characters are.
pub fn gen_random_chars(n: usize) -> Vec<char> {
    let mut v = Vec::with_capacity(n);
    let dist = weighted_index();
    let mut rng = thread_rng();

    for _ in 0..n {
        v.push(LETTER_WEIGHTS[dist.sample(&mut rng)].0);
    }
    v
}

pub type Table = [char; 16];

pub fn gen_table() -> Table {
    let mut v = [' '; 16];
    let dist = weighted_index();
    let mut rng = thread_rng();

    for c in &mut v {
        *c = LETTER_WEIGHTS[dist.sample(&mut rng)].0;
    }
    v
}

pub struct WordGameState {
    pub table: Table,
    pub taken_words: Vec<String>,
    pub guessers: HashMap<UserId, u32>,
    pub message: Message,
}

pub enum GuessError {
    AlreadyGuessed,
    WrongLetters,
    NotFound,
}

pub fn format_table(table: &Table) -> String {
    let &[
        a0, b0, c0, d0,
        a1, b1, c1, d1,
        a2, b2, c2, d2,
        a3, b3, c3, d3,
    ] = table;

    format!(
        "```\n{} {} {} {}\n{} {} {} {}\n{} {} {} {}\n{} {} {} {}\n```",
        a0, b0, c0, d0,
        a1, b1, c1, d1,
        a2, b2, c2, d2,
        a3, b3, c3, d3,
    )
}

impl WordGameState {
    pub fn new(table: Table, message: Message) -> Self {
        WordGameState {
            taken_words: Vec::new(),
            guessers: HashMap::new(),
            table,
            message
        }
    }
    pub async fn guess_word(&mut self, user: UserId, word: String) -> Result<(), GuessError> {
        let index_to_insert = match self.taken_words.binary_search(&word) {
            Ok(_) => return Err(GuessError::AlreadyGuessed),
            Err(i) => i,
        };

        let mut letters: Vec<char> = self.table.to_vec();
        letters.sort_unstable();
        for c in word.chars() {
            match letters.binary_search(&c) {
                Ok(i) => {
                    letters.remove(i);
                }
                Err(_) => return Err(GuessError::WrongLetters),
            }
        }

        if check_word(&word).await {
            self.taken_words.insert(index_to_insert, word);

            *self.guessers.entry(user).or_insert(0) += 1;

            Ok(())
        } else {
            Err(GuessError::NotFound)
        }
    }
}

impl TypeMapKey for WordGameState {
    type Value = Self;
}

use crate::dictionary::sprotin::search;

async fn check_word(mut s: &str) -> bool {
    // probably not neccessary 
    s = s.trim();
    let words = {
        let response_inflections = search(1, 1, s, true, false).await.unwrap();
        let response_sinflections = search(1, 1, s, false, false).await.unwrap();
        let mut words = response_inflections.words;
        words.extend(response_sinflections.words);
        words
    };

    for word in words {
        if s == word.search_word {
            return true;
        }
        if word.inflected_form.iter().map(|s| s.split('/')).flatten().map(|s| s.trim()).any(|w| w == s) {
            return true;
        }
    }
    false
}