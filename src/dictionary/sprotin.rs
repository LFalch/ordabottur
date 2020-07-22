use std::num::NonZeroUsize;
use reqwest::{
    blocking::get as reqwest_get
};
use serde::{Deserialize, Deserializer};
use scraper::Html;

use crate::util::MsgBunch;

#[derive(Debug, Clone, Deserialize)]
pub struct SprotinResponse {
    search_inflections: u8,
    search_description: u8,
    status: ResponseStatus,
    message: Option<String>,
    total: u32,
    from: u32,
    to: u32,
    time: f64,
    words: Vec<SprotinWord>,
    single_word: Option<SprotinWord>,
    related_words: Vec<()>,
    groups: Vec<SprotinGroup>,
    dictionary: SprotinDictionary,
    dictionaries_results: Vec<DictionaryResults>,
    similar_words: Vec<SimilarWord>,
    page: u16,
    searchfor: String,
    new_words: NewWordsStatus,
    popular_words: Vec<PopularWord>,
    searches_by_country: Vec<CountryWithSearches>,
    words_from_same_groups: Vec<()>
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResponseStatus {
    Success,
    NotFound
}

#[derive(Debug, Clone, Deserialize)]
struct DictionaryResults {
    id: u32,
    results: u32
}
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum NewWordsStatus {
    Status {status: String},
    List(Vec<NewWord>)
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NewWord {
    search_word: String,
    display_word: String,
    date: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PopularWord {
    search_word: String,
    quantity: u32,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SimilarWord {
    search_word: String,
    difference: u32,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CountryWithSearches {
    country: String,
    quantity: u64,
    percent: f32,
}
#[derive(Debug, Clone, Deserialize)]
struct SprotinGroup {
    id: u16,
    title: String,
    // seems to be a number in that string though
    words: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SprotinDictionary {
    id: u8,
    title: String,
    short_title: String,
    #[serde(rename = "Type")]
    dictionary_type: String,
    owner: String,
    // in dative
    owner_inflected: String,
    owner_url: String,
    owner_email: String,
    table: String,
    // hex rgb with # preceding
    color: String,
    // in html
    info: String,
    total_words: u64,
    total_searches: u64,
}

// fn<'de, D>(D) -> Result<T, D::Error> where D: Deserializer<'de>

fn deserialize_optional_vec<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Option::<Vec<String>>::deserialize(d).map(Option::unwrap_or_default)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SprotinWord {
    id: u64,
    image_filename: Option<String>,
    image_comment: Option<String>,
    image_owner: Option<String>,
    prepend_word: Option<String>,
    search_word: String,
    display_word: String,
    // Type hmm
    word_list: Option<String>,
    // Inflexional categories
    inflex_cats: Option<String>,
    short_inflected_form: Option<String>,
    #[serde(deserialize_with = "self::deserialize_optional_vec")]
    inflected_form: Vec<String>,
    // in html
    explanation: String,
    origin: Option<String>,
    origin_source: Option<String>,
    grammar_comment: Option<String>,
    // Type hmm
    word_nr: Option<u16>,
    index: u64,
    phonetic: Option<String>,
    // Type should be Date it's in yyyy-mm-dd hh:mm:ss
    date: String,
    groups: Vec<SprotinGroup>,
    short_inflection: Option<String>
}

impl SprotinWord {
    fn to_very_short_string(&self) -> String {
        if self.search_word != self.display_word {
            dbg!((&self.search_word, &self.display_word));
        }

        let mut s = format!("**{}**", self.display_word);

        if let Some(mut inflex_cats) = self.inflex_cats.clone() {
            // Use scraper here instead <span class="_c"> seems to be equivalent to <sup> here
            if let (Some(i), Some(j)) = (inflex_cats.find("<sup>"), inflex_cats.find("</sup>")) {
                inflex_cats = format!("{} {}{} ", &inflex_cats[..i], crate::util::to_superscript(&inflex_cats[i+5..j]), &inflex_cats[j+6..]);
            }

            let inflex_cats: String = Html::parse_fragment(&inflex_cats).tree.values().filter_map(|val| val.as_text().map(|t| t.text.to_string())).collect();

            s.push_str(&format!(" _{}_", inflex_cats));
        }

        if let Some(short_inflected_form) = &self.short_inflected_form {
            let short_inflected_form: String = Html::parse_fragment(short_inflected_form).tree.values().filter_map(|val| val.as_text().map(|t| t.text.to_string())).collect();

            s.push_str(&format!(", _{}_", short_inflected_form));
        }
        if let Some(short_inflection) = &self.short_inflection {
            s.push_str(&format!(", ²_{}_", short_inflection));
        }

        if let Some(phonetic) = &self.phonetic {
            let phonetic: String = Html::parse_fragment(phonetic).tree.values().filter_map(|val| val.as_text().map(|t| t.text.to_string())).collect();

            s.push_str(&format!(" {}", phonetic));
        }

        match (&self.origin, &self.origin_source) {
            (Some(o), None) | (None, Some(o)) => s.push_str(&format!(" (frá {})", o)),
            (Some(origin), Some(origin_source)) => s.push_str(&format!(" (frá {} {})", origin, origin_source)),
            (None, None) => (),
        }

        s
    }

    const SHORT_EXPLANATION_LENGTH: usize = 138;

    pub fn to_short_string(&self) -> String {
        let mut s = self.to_very_short_string();

        s.push_str(":  ");

        {
            // TODO interpret different classes appropriately (this rn just ignores all html tags and just retrieves the raw text)
            let explanation: String = Html::parse_fragment(&self.explanation).tree.values().filter_map(|val| val.as_text().map(|t| t.text.to_string())).collect();

            if explanation.len() >= Self::SHORT_EXPLANATION_LENGTH {
                let mut cutoff = Self::SHORT_EXPLANATION_LENGTH;
                
                while !explanation.is_char_boundary(cutoff) {
                    cutoff -= 1;
                }
    
                s.push_str(&explanation[..cutoff]);
                s.push('…');
            } else {
                s.push_str(&explanation);
            }
        }

        s
    }

    pub fn to_full_string(&self) -> String {
        if let Some(prepend_word) = &self.prepend_word {
            eprintln!("prepend_word: {}", prepend_word);
        }

        let mut s = self.to_very_short_string();

        s.push('\n');
        {
            // TODO interpret different classes appropriately (this rn just ignores all html tags and just retrieves the raw text)
            let explanation: String = Html::parse_fragment(&self.explanation).tree.values().filter_map(|val| val.as_text().map(|t| t.text.to_string())).collect();

            s.push_str(&explanation);
        }

        s.push('\n');
        s.push_str("Bendingar: ");
        for form in &self.inflected_form {
            s.push_str(&form);
            s.push(' ');
        }

        if let Some(grammar_comment) = &self.grammar_comment {
            s.push('\n');
            s.push_str(&grammar_comment);
        }

        s
    }
}


#[cfg(not(feature = "from_res_error_resolve"))]
#[inline(always)]
fn from_res(res: reqwest::blocking::Response) -> SprotinResponse {
    res.json().unwrap()
}
#[cfg(feature = "from_res_error_resolve")]
fn from_res(res: reqwest::blocking::Response) -> SprotinResponse {
    let s = res.text().unwrap();

    let pretty = {
        let json: serde_json::Value = serde_json::from_str(&s).unwrap();

        serde_json::ser::to_string_pretty(&json).unwrap()
    };

    match serde_json::from_str(&pretty) {
        Ok(j) => j,
        Err(e) => {
            use std::io::Write;

            let mut file = std::fs::File::create("pretty.json").unwrap();
            file.write_all(pretty.as_bytes()).unwrap();

            Err(e).unwrap()
        }
    }
}

pub fn search(dictionary_id: u8, dictionary_page: u16, search_for: &str, search_inflections: bool, search_descriptions: bool) -> Result<SprotinResponse, u16> {
    // This one doesn't seem to make a difference
    const SKIP_OTHER_DICTIONARIES_RESULTS: bool = true;
    // This is one gives us similar word suggestions if no results were found
    const SKIP_SIMILAR_WORDS: bool = false;

    let res = reqwest_get(&format!("https://sprotin.fo/dictionary_search_json.php?DictionaryId={}&DictionaryPage={}&SearchFor={}&SearchInflections={}&SearchDescriptions={}&Group={}&SkipOtherDictionariesResults={}&SkipSimilarWords={}",
        dictionary_id, dictionary_page, search_for, search_inflections as u8, search_descriptions as u8, "", SKIP_OTHER_DICTIONARIES_RESULTS as u8, SKIP_SIMILAR_WORDS as u8)).unwrap();

    if res.status().is_success() {
        Ok(from_res(res))
    } else {
        Err(res.status().as_u16())
    }
}

fn dictionary_name(i: u32) -> &'static str {
    match i {
        1 => "FØ-FØ",
        2 => "FØ-EN",
        3 => "EN-FØ",
        4 => "FØ-DA",
        5 => "DA-FØ",
        21 => "DA-FØ2",
        6 => "FØ-TÝ",
        7 => "TÝ-FØ",
        10 => "FØ-SP",
        20 => "SP-FØ",
        30 => "GR-FØ",
        9 => "FR-FØ",
        11 => "FØ-IT",
        12 => "RU-FØ",
        24 => "FØ-KI",
        26 => "KI-FØ",
        15 => "SAM",
        25 => "NAVN",
        22 => "ALFR",
        23 => "TILT",
        13 => "YRK",
        32 => "BUSK",
        _ => panic!("no such dictionary"),
    }
}

impl SprotinResponse {
    pub fn word(&self, word_nr: NonZeroUsize) -> Option<MsgBunch> {
        let mut msg_bunch = MsgBunch::new();

        msg_bunch.add_string(&self.words.get(word_nr.get()-1)?.to_full_string());

        Some(msg_bunch)
    }
    pub fn summary(self) -> MsgBunch {
        let SprotinResponse {
            message,
            status,
            total,
            from,
            to,
            time,
            mut words,
            single_word,
            related_words,
            similar_words,
            page,
            dictionaries_results,
            ..
        } = self;
        
        if !related_words.is_empty() {
            dbg!(related_words);
        }
        if single_word.is_some() {
            dbg!(single_word);
        }

        let mut msg_bunch = MsgBunch::new();

        if let Some(message) = &message {
            msg_bunch.add_string("__").add_string(message).add_string("__\n");
        }
        msg_bunch.add_string(&format!("Síða {}. Vísir úrslit {} - {} av {} ({:.3} sekund)\n", page, from, to, total, time));
        for result in dictionaries_results.into_iter().filter(|r| r.results > 0) {
            msg_bunch.add_string("**").add_string(dictionary_name(result.id)).add_string("** ").add_string(&format!("{}", result.results)).add_string(" ");
        }
        msg_bunch.add_string("\n\n");

        match status {
            ResponseStatus::NotFound => {
                if !similar_words.is_empty() {
                    let similar_words: Vec<_> = similar_words.into_iter().map(|w| format!("_{}_", w.search_word)).collect();

                    msg_bunch.add_string("Meinti tú: ").add_string(&similar_words.join(", "));
                }
            },
            ResponseStatus::Success => {
                match &*words {
                    [word] => {
                        msg_bunch.add_string("1. ").add_string(&word.to_full_string());
                    }
                    _ => {
                        // Only show 50
                        if words.len() > 50 {
                            words.resize_with(50, || unreachable!());
                        }
                        for (i, word) in (1..).zip(words.into_iter()) {
                            msg_bunch.add_string(&format!("{}. {}\n", i, word.to_short_string()));
                        }
                    }
                }
            }
        }
        msg_bunch
    }
}