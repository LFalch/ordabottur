use reqwest::{
    blocking::get as reqwest_get
};
use serde_json::from_reader;
use serde::{Deserialize, Deserializer};

use crate::util::MsgBunch;

#[derive(Debug, Clone, Deserialize)]
struct SprotinResponse {
    search_inflections: u8,
    search_description: u8,
    status: String,
    message: Option<String>,
    total: u32,
    from: u32,
    to: u32,
    time: f64,
    words: Vec<SprotinWord>,
    single_word: Option<SprotinWord>,
    related_words: Vec<()>,
    groups: Vec<()>,
    dictionary: SprotinDictionary,
    dictionaries_results: Vec<DictionaryResults>,
    similar_words: Vec<()>,
    page: u16,
    searchfor: String,
    new_words: NewWordsStatus,
    popular_words: Vec<PopularWord>,
    searches_by_country: Vec<CountryWithSearches>,
    words_from_same_groups: Vec<()>
}

#[derive(Debug, Clone, Deserialize)]
struct DictionaryResults {
    id: u32,
    results: u32
}
#[derive(Debug, Clone, Deserialize)]
struct NewWordsStatus {
    status: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PopularWord {
    search_word: String,
    quantity: u32,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CountryWithSearches {
    country: String,
    quantity: u64,
    percent: f32,
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
    inflex_cats: String,
    short_inflected_form: Option<String>,
    #[serde(deserialize_with = "self::deserialize_optional_vec")]
    inflected_form: Vec<String>,
    // in html
    explanation: String,
    origin: Option<String>,
    origin_source: Option<String>,
    grammar_comment: Option<String>,
    // Type hmm
    word_nr: Option<String>,
    index: u64,
    phonetic: Option<String>,
    // Type should be Date it's in yyyy-mm-dd hh:mm:ss
    date: String,
    groups: Vec<()>,
    short_inflection: Option<String>
}

impl SprotinWord {
    pub fn tos(&self) -> String {
        if let Some(prepend_word) = &self.prepend_word {
            eprintln!("prepend_word: {}", prepend_word);
        }

        if self.search_word != self.display_word {
            dbg!((&self.search_word, &self.display_word));
        }

        let mut s = format!("**{}** _{}_", self.display_word, self.inflex_cats);
        if let Some(short_inflected_form) = &self.short_inflected_form {
            s.push_str(&format!(", _{}_", short_inflected_form));
        }
        if let Some(short_inflection) = &self.short_inflection {
            s.push_str(&format!(", ²_{}_", short_inflection));
        }

        if let Some(phonetic) = &self.phonetic {
            s.push_str(&format!(" {}", phonetic));
        }

        match (&self.origin, &self.origin_source) {
            (Some(o), None) | (None, Some(o)) => s.push_str(&format!(" (frá {})", o)),
            (Some(origin), Some(origin_source)) => s.push_str(&format!(" (frá {} {})", origin, origin_source)),
            (None, None) => (),
        }
        s.push('\n');
        s.push_str(&self.explanation);

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

pub fn search(dictionary_id: u8, dictionary_page: u16, search_for: &str, search_inflections: bool, search_descriptions: bool, skip_other_dictionaries_results: bool, skip_similar_words: bool) -> Result<MsgBunch, u16> {
    let res = reqwest_get(&format!("https://sprotin.fo/dictionary_search_json.php?DictionaryId={}&DictionaryPage={}&SearchFor={}&SearchInflections={}&SearchDescriptions={}&Group={}&SkipOtherDictionariesResults={}&SkipSimilarWords={}",
        dictionary_id, dictionary_page, search_for, search_inflections as u8, search_descriptions as u8, "", skip_other_dictionaries_results as u8, skip_similar_words as u8)).unwrap();

    if res.status().is_success() {
        let SprotinResponse {
            message,
            total,
            from,
            to,
            time,
            words,
            single_word,
            related_words,
            similar_words,
            page,
            ..
        } = from_reader(res).unwrap();

        let mut msg_bunch = MsgBunch::new();

        if let Some(message) = &message {
            msg_bunch.add_string("Message: ").add_string(message).add_string("\n");
        }
        msg_bunch.add_string(&format!("Vísir úrslit {} - {} av {} ({:.3} sekund)\n", from, to, total, time));
        if let Some(single_word) = single_word {
            msg_bunch
                .add_string("Einfalt orð: ")
                .add_string(&single_word.tos())
                .add_string("\n");
        }
        for word in words {
            msg_bunch.add_string(&word.tos()).add_string("\n");
        }


        msg_bunch.add_string("\n").add_string("Síða ").add_string(&format!("{}", page));

        Ok(msg_bunch)
    } else {
        Err(res.status().as_u16())
    }
}