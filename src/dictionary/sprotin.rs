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
        if !self.inflected_form.is_empty() {
            s.push_str(&self.inflection_table());
            s.push('\n');
        }

        if let Some(grammar_comment) = &self.grammar_comment {
            s.push('\n');
            s.push_str(&grammar_comment);
        }

        s
    }

    // TODO kinda hacky, but done after the JS making the tables on Sprotin itself
    pub fn inflection_table(&self) -> String {
        let SprotinWord{inflected_form, ..} = self;

        match &**inflected_form {
            // verb
            [infinitive, present_3p, past_sg, past_pl, supine, past_part] => {
                const SG_COLUMN_TITLE: &str = "eintal/sg.";
                let sg_column_width = [present_3p, past_sg].iter().map(|f| f.chars().count()).max().unwrap().max(SG_COLUMN_TITLE.chars().count());
                const PL_COLUMN_TITLE: &str = "fleirtal/pl.";
                let pl_column_width = [infinitive, past_pl].iter().map(|f| f.chars().count()).max().unwrap().max(PL_COLUMN_TITLE.chars().count());

                format!(r"```
navnháttur/infinitive                            | {:sg$} |
lýsingarháttur í tátíð / supine                  | {:sg$} |
  Bendingar í tíð / conjugations                 | {:sg$} | {:pl$} |
3. persónur í nútid / 3rd sg. present            | {:sg$} | {:pl$} |
eintal   í tátíð / sg. past                      | {:sg$} | {:pl$} |
lýsingarháttur í tátíð, k. hvørfall / past part. | {:sg$} | {:pl$} |
```", infinitive, supine, SG_COLUMN_TITLE, PL_COLUMN_TITLE, present_3p, infinitive, past_sg, past_pl, past_part, "", sg = sg_column_width, pl = pl_column_width)
            }
            // noun
            [nom_sg, acc_sg, dat_sg, gen_sg, nom_pl, acc_pl, dat_pl, gen_pl
            ,nom_sd, acc_sd, dat_sd, gen_sd, nom_pd, acc_pd, dat_pd, gen_pd] => {
                const INDEFINITE_COLUMN_TITLE: &str = "óbundið / indef.";
                let i_column = vec![nom_sg, acc_sg, dat_sg, gen_sg, nom_pl, acc_pl, dat_pl, gen_pl];
                let i_column_width = i_column.iter().map(|f| f.chars().count()).max().unwrap().max(INDEFINITE_COLUMN_TITLE.chars().count());
                let mut i_column = i_column.into_iter();
                const DEFINITE_COLUMN_TITLE: &str = "bundið / def.";
                let d_column = vec![nom_sd, acc_sd, dat_sd, gen_sd, nom_pd, acc_pd, dat_pd, gen_pd];
                let d_column_width = d_column.iter().map(|f| f.chars().count()).max().unwrap().max(DEFINITE_COLUMN_TITLE.chars().count());
                let mut d_column = d_column.into_iter();

                "```\n".to_owned() +
                &format!("  eintal/sg.    | {:2$} | {:3$} |\n", INDEFINITE_COLUMN_TITLE, DEFINITE_COLUMN_TITLE, i_column_width, d_column_width) +
                &format!("hvørfall/nom    | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                &format!("hvønnfall/acc   | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                &format!("hvørjumfall/dat | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                &format!("hvørsfall/gen   | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                         "  fleirtal/pl.\n" +
                &format!("hvørfall/nom    | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                &format!("hvønnfall/acc   | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                &format!("hvørjumfall/dat | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                &format!("hvørsfall/gen   | {:2$} | {:3$} |\n", i_column.next().unwrap(), d_column.next().unwrap(), i_column_width, d_column_width) +
                "```"
            }
            // adjective
            [m_nom_sg, m_acc_sg, m_dat_sg, m_gen_sg, m_nom_pl, m_acc_pl, m_dat_pl, m_gen_pl
            ,f_nom_sg, f_acc_sg, f_dat_sg, f_gen_sg, f_nom_pl, f_acc_pl, f_dat_pl, f_gen_pl
            ,n_nom_sg, n_acc_sg, n_dat_sg, n_gen_sg, n_nom_pl, n_acc_pl, n_dat_pl, n_gen_pl] => {
                const M_COLUMN_TITLE: &str = "masc/k";
                let m_column = vec![m_nom_sg, m_acc_sg, m_dat_sg, m_gen_sg, m_nom_pl, m_acc_pl, m_dat_pl, m_gen_pl];
                let m_column_width = m_column.iter().map(|f| f.chars().count()).max().unwrap().max(M_COLUMN_TITLE.chars().count());
                let mut m_column = m_column.into_iter();
                const F_COLUMN_TITLE: &str = "fem/kv";
                let f_column = vec![f_nom_sg, f_acc_sg, f_dat_sg, f_gen_sg, f_nom_pl, f_acc_pl, f_dat_pl, f_gen_pl];
                let f_column_width = f_column.iter().map(|f| f.chars().count()).max().unwrap().max(F_COLUMN_TITLE.chars().count());
                let mut f_column = f_column.into_iter();
                const N_COLUMN_TITLE: &str = "neut/h";
                let n_column = vec![n_nom_sg, n_acc_sg, n_dat_sg, n_gen_sg, n_nom_pl, n_acc_pl, n_dat_pl, n_gen_pl];
                let n_column_width = n_column.iter().map(|f| f.chars().count()).max().unwrap().max(N_COLUMN_TITLE.chars().count());
                let mut n_column = n_column.into_iter();

                "```\n".to_owned() +
                &format!("  eintal/sg     | {:3$} | {:4$} | {:5$} |\n", M_COLUMN_TITLE, F_COLUMN_TITLE, N_COLUMN_TITLE, m_column_width, f_column_width, n_column_width) +
                &format!("hvørfall/nom    | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                &format!("hvønnfall/acc   | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                &format!("hvørjumfall/dat | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                &format!("hvørsfall/gen   | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                         "  fleirtal/pl.\n" +
                &format!("hvørfall/nom    | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                &format!("hvønnfall/acc   | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                &format!("hvørjumfall/dat | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                &format!("hvørsfall/gen   | {:3$} | {:4$} | {:5$} |\n", m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m_column_width, f_column_width, n_column_width) +
                "```"
            }
            f => format!("Unknown inflectional paradigm:\n{}", f.join(", ")),
        }
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