#![allow(dead_code)]

use std::num::NonZeroUsize;
use reqwest::{
    Response,
    get as reqwest_get
};
use serde::{Deserialize, Deserializer};
use scraper::{Html, Node};

use crate::util::{MsgBunchBuilder, MsgBunch, to_subscript, to_superscript, split_trim};

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
    pub words: Vec<SprotinWord>,
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

#[derive(Debug, Copy, Clone, Default)]
struct CalculatedStyle {
    bold: bool,
    italics: bool,
    underline: bool,
    strikethrough: bool,
    superscript: bool,
    subscript: bool,
}

#[derive(Debug, Copy, Clone)]
struct Style {
    bold: Option<bool>,
    italics: Option<bool>,
    underline: Option<bool>,
    strikethrough: Option<bool>,
    superscript: Option<bool>,
    subscript: Option<bool>,
    newline_follows: bool,
}

const EMPTY: Style = Style {
    bold: None,
    italics: None,
    underline: None,
    strikethrough: None,
    superscript: None,
    subscript: None,
    newline_follows: false,
};
const DEFAULT: Style = Style {
    bold: Some(false),
    italics: Some(false),
    strikethrough: None,
    underline: None,
    superscript: None,
    subscript: None,
    newline_follows: false,
};
const ITALICS: Style = Style {
    italics: Some(true),
    .. DEFAULT
};

impl Style {
    fn from_element_name(s: &str) -> Self {
        match s {
            "b" | "strong" => Style { bold: Some(true), .. EMPTY},
            "i" | "em" => Style { italics: Some(true), .. EMPTY},
            "a" | "mark" => Style { underline: Some(true), .. EMPTY},
            "del" => Style { strikethrough: Some(true), .. EMPTY},
            "sub" => Style { subscript: Some(true), .. EMPTY},
            "sup" => Style { superscript: Some(true), .. EMPTY},
            _ => EMPTY,
        }
    }

    fn from_class(s: &str) -> Self {
        match s {
            "_eind" | "_h" | "_H" | "_smb" | "_p" | "_p1" | "_p2" | "_m" | "_D" => DEFAULT,
            "word_link" => Style { underline: Some(true), .. EMPTY},
            "_r" => Style { bold: Some(true), newline_follows: true, .. DEFAULT},
            "dictionary_number_bold" | "_R" | "_u" | "_l" | "_s" | "_a" | "_a2" | "_A" => Style { bold: Some(true), .. DEFAULT},
            "_d" | "_k" => Style { italics: Some(true), .. DEFAULT},
            "_c" => Style { superscript: Some(true), italics: Some(true), .. DEFAULT},
            _ => EMPTY,
        }
    }
    fn calculate(self) -> CalculatedStyle {
        CalculatedStyle {
            bold: self.bold.unwrap_or(false),
            italics: self.italics.unwrap_or(false),
            underline: self.underline.unwrap_or(false),
            strikethrough: self.strikethrough.unwrap_or(false),
            superscript: self.superscript.unwrap_or(false),
            subscript: self.subscript.unwrap_or(false),
        }
    }
}

impl std::ops::BitOr for Style {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Style {
            bold: self.bold.or(rhs.bold),
            italics: self.italics.or(rhs.italics),
            underline: self.underline.or(rhs.underline),
            strikethrough: self.strikethrough.or(rhs.strikethrough),
            superscript: self.superscript.or(rhs.superscript),
            subscript: self.subscript.or(rhs.subscript),
            newline_follows: self.newline_follows,
        }
    }
}

#[derive(Debug, Clone)]
struct DiscordStylisedTextBuilder {
    buf: String,
    last_style: CalculatedStyle,
    last_whitespace_length: usize,
}

impl DiscordStylisedTextBuilder {
    fn new() -> Self {
        DiscordStylisedTextBuilder {
            buf: String::new(),
            last_style: CalculatedStyle::default(),
            last_whitespace_length: 0,
        }
    }
    fn push_str(&mut self, s: &str, style: CalculatedStyle) {
        use std::borrow::Cow;
        let c: Cow<_>;
        if style.superscript {
            c = to_superscript(s).into();
        } else if style.subscript {
            c = to_subscript(s).into();
        } else {
            c = s.into();
        }

        fn changed(a: bool, b: bool) -> Option<bool> {
            if a == b {
                None
            } else {
                Some(b)
            }
        }
        fn tos(s: &str, b: Option<bool>, start: bool) -> &str {
            match (start, b) {
                (true, Some(true)) => s,
                (false, Some(false)) => s,
                _ => "",
            } 
        }

        let bold_changed = changed(self.last_style.bold, style.bold);
        let italics_changed = changed(self.last_style.italics, style.italics);
        let underline_changed = changed(self.last_style.underline, style.underline);
        let strikethrough_changed = changed(self.last_style.strikethrough, style.strikethrough);

        let any_changed = bold_changed.is_some() || italics_changed.is_some() || underline_changed.is_some() || strikethrough_changed.is_some();

        let s = c.as_ref();

        if !any_changed {
            self.buf.push_str(s);
            self.last_whitespace_length = s.rfind(|c: char| !c.is_whitespace()).map(|i| s.len() - i - 1).unwrap_or_else(|| s.len());
        } else {
            let (start_whitespace, text, end_whitespace) = split_trim(s);
            let start_whitespace = self.buf.split_off(self.buf.len() - self.last_whitespace_length) + start_whitespace;

            self.buf.push_str(tos("__", underline_changed, false));
            self.buf.push_str(tos("**", bold_changed, false));
            self.buf.push_str(tos("_", italics_changed, false));
            self.buf.push_str(tos("~~", strikethrough_changed, false));

            self.buf.push_str(&start_whitespace);

            self.buf.push_str(tos("~~", strikethrough_changed, true));
            self.buf.push_str(tos("_", italics_changed, true));
            self.buf.push_str(tos("**", bold_changed, true));
            self.buf.push_str(tos("__", underline_changed, true));
            self.buf.push_str(text);
            self.buf.push_str(end_whitespace);

            self.last_whitespace_length = end_whitespace.len();
        }

        self.last_style = style;
    }
    fn build(self) -> String {
        let DiscordStylisedTextBuilder {
            last_style,
            mut buf,
            last_whitespace_length: _,
        } = self;

        if last_style.underline {
            buf.push_str("__")
        }
        if last_style.italics {
            buf.push('_')
        }
        if last_style.bold {
            buf.push_str("**")
        }
        if last_style.strikethrough {
            buf.push_str("~~")
        }
        
        buf
    }
}

fn parse_children(ret: &mut DiscordStylisedTextBuilder, children: ::ego_tree::iter::Children<Node>, style: Style) {
    for child in children {
        match child.value() {
            Node::Element(elem) => {
                let elem_style = style | Style::from_element_name(dbg!(elem.name()));
                let style = elem.classes().fold(elem_style, |acc, b| Style::from_class(dbg!(b)) | acc);
                parse_children(ret, child.children(), style)
            }
            Node::Text(text) => {
                let s = text.text.to_string();

                ret.push_str(&dbg!(s), style.calculate());
            }
            _ => ()
        }
    }
    if style.newline_follows {
        ret.push_str("\n", style.calculate());
    }
}

// TODO interpret different classes appropriately (this rn just ignores all html tags and just retrieves the raw text)
fn html_to_discord_markup(s: &str, style: Style) -> String {
    let mut ret = DiscordStylisedTextBuilder::new();

    eprintln!("{}", s);
    let html = Html::parse_fragment(s);
    parse_children(&mut ret, html.tree.root().children(), style);

    ret.build()
}

fn deserialize_optional_vec<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Option::<Vec<String>>::deserialize(d).map(Option::unwrap_or_default)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SprotinWord {
    id: u64,
    image_filename: Option<String>,
    image_comment: Option<String>,
    image_owner: Option<String>,
    prepend_word: Option<String>,
    pub search_word: String,
    pub display_word: String,
    // Type hmm
    word_list: Option<String>,
    // Inflexional categories
    inflex_cats: Option<String>,
    short_inflected_form: Option<String>,
    #[serde(deserialize_with = "self::deserialize_optional_vec")]
    pub inflected_form: Vec<String>,
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

        if let Some(short_inflected_form) = &self.short_inflected_form {
            let short_inflected_form = html_to_discord_markup(&short_inflected_form.replace('\r', "").replace('\n', ""), EMPTY);

            s.push_str(&format!(" {}", short_inflected_form));
        }
        if let Some(inflex_cats) = &self.inflex_cats {
            let inflex_cats = html_to_discord_markup(inflex_cats, ITALICS);

            s.push_str(&format!(" {}", inflex_cats));
        }
        if let Some(grammar_comment) = &self.grammar_comment {
            let grammar_comment = html_to_discord_markup(grammar_comment, ITALICS);

            s.push_str(&format!(" {}", grammar_comment));
        }
        if let Some(short_inflection) = &self.short_inflection {
            s.push_str(&format!(", ²{}", short_inflection));
        }

        if let Some(phonetic) = &self.phonetic {
            let phonetic: String = html_to_discord_markup(phonetic, EMPTY);

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

        s.push_str(": ");

        {
            let explanation: String = html_to_discord_markup(&self.explanation, EMPTY);

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

    pub fn to_full_string(&self, mmb: &mut MsgBunchBuilder) {
        mmb.begin_section();
        if let Some(prepend_word) = &self.prepend_word {
            eprintln!("prepend_word: {}", prepend_word);
        }

        mmb.add_string(self.to_very_short_string()).add_string("\n").end_section();

        {
            let explanation: String = html_to_discord_markup(&self.explanation, EMPTY);

            mmb.add_lines(explanation);
        }

        if !self.inflected_form.is_empty() {
            mmb.begin_section().add_string(&self.inflection_table()).add_string("\n").end_section();
        }

        for msg in &mmb.inner.messages {
            eprint!("{}", msg);
        }
        eprintln!();
    }

    // TODO kinda hacky, but done after the JS making the tables on Sprotin itself
    pub fn inflection_table(&self) -> String {
        let SprotinWord{inflected_form, ..} = self;

        match &**inflected_form {
            // verb
            [infinitive, present_3p, past_sg, past_pl, supine, past_part] => {
                const SG_COLUMN_TITLE: &str = "eintal/sg";
                let sg_column_width = [present_3p, past_sg].iter().map(|f| f.chars().count()).max().unwrap().max(SG_COLUMN_TITLE.chars().count());
                const PL_COLUMN_TITLE: &str = "fleirtal/pl";
                let pl_column_width = [infinitive, past_pl].iter().map(|f| f.chars().count()).max().unwrap().max(PL_COLUMN_TITLE.chars().count());

                format!(r"```
navnháttur/infinitive                            | {:sg$} |
lýsingarháttur í tátíð / supine                  | {:sg$} |
  Bendingar í tíð / conjugations                 | {:sg$} | {:pl$} |
3. persónur í nútíð / 3rd sg. present            | {:sg$} | {:pl$} |
eintal   í tátíð / sg. past                      | {:sg$} | {:pl$} |
lýsingarháttur í tátíð, k. hvørfall / past part. | {:sg$} | {:pl$} |
```", infinitive, supine, SG_COLUMN_TITLE, PL_COLUMN_TITLE, present_3p, infinitive, past_sg, past_pl, past_part, "", sg = sg_column_width, pl = pl_column_width)
            }
            // noun
            [nom_sg, acc_sg, dat_sg, gen_sg, nom_pl, acc_pl, dat_pl, gen_pl
            ,nom_sd, acc_sd, dat_sd, gen_sd, nom_pd, acc_pd, dat_pd, gen_pd] => {
                const INDEFINITE_COLUMN_TITLE: &str = "ób./indef";
                let i_column = vec![nom_sg, acc_sg, dat_sg, gen_sg, nom_pl, acc_pl, dat_pl, gen_pl];
                let i_column_width = i_column.iter().map(|f| f.chars().count()).max().unwrap().max(INDEFINITE_COLUMN_TITLE.chars().count());
                let mut i_column = i_column.into_iter();
                const DEFINITE_COLUMN_TITLE: &str = "b./def";
                let d_column = vec![nom_sd, acc_sd, dat_sd, gen_sd, nom_pd, acc_pd, dat_pd, gen_pd];
                let d_column_width = d_column.iter().map(|f| f.chars().count()).max().unwrap().max(DEFINITE_COLUMN_TITLE.chars().count());
                let mut d_column = d_column.into_iter();

                format!(r"```
  eintal/sg.    | {:i$} | {:d$} | 
hvørfall/nom    | {:i$} | {:d$} | 
hvønnfall/acc   | {:i$} | {:d$} | 
hvørjumfall/dat | {:i$} | {:d$} | 
hvørsfall/gen   | {:i$} | {:d$} |
  fleirtal/pl.\n
hvørfall/nom    | {:i$} | {:d$} |
hvønnfall/acc   | {:i$} | {:d$} |
hvørjumfall/dat | {:i$} | {:d$} |
hvørsfall/gen   | {:i$} | {:d$} |
```", INDEFINITE_COLUMN_TITLE, DEFINITE_COLUMN_TITLE, i_column.next().unwrap(), d_column.next().unwrap(), i_column.next().unwrap(),
                d_column.next().unwrap(), i_column.next().unwrap(), d_column.next().unwrap(), i_column.next().unwrap(), d_column.next().unwrap(),
                i_column.next().unwrap(), d_column.next().unwrap(), i_column.next().unwrap(), d_column.next().unwrap(), i_column.next().unwrap(),
                d_column.next().unwrap(), i_column.next().unwrap(), d_column.next().unwrap(), i = i_column_width, d = d_column_width)
            }
            // adjective
            [m_nom_sg, m_acc_sg, m_dat_sg, m_gen_sg, m_nom_pl, m_acc_pl, m_dat_pl, m_gen_pl
            ,f_nom_sg, f_acc_sg, f_dat_sg, f_gen_sg, f_nom_pl, f_acc_pl, f_dat_pl, f_gen_pl
            ,n_nom_sg, n_acc_sg, n_dat_sg, n_gen_sg, n_nom_pl, n_acc_pl, n_dat_pl, n_gen_pl] => {
                const M_COLUMN_TITLE: &str = "k./masc";
                let m_column = vec![m_nom_sg, m_acc_sg, m_dat_sg, m_gen_sg, m_nom_pl, m_acc_pl, m_dat_pl, m_gen_pl];
                let m_column_width = m_column.iter().map(|f| f.chars().count()).max().unwrap().max(M_COLUMN_TITLE.chars().count());
                let mut m_column = m_column.into_iter();
                const F_COLUMN_TITLE: &str = "kv./fem";
                let f_column = vec![f_nom_sg, f_acc_sg, f_dat_sg, f_gen_sg, f_nom_pl, f_acc_pl, f_dat_pl, f_gen_pl];
                let f_column_width = f_column.iter().map(|f| f.chars().count()).max().unwrap().max(F_COLUMN_TITLE.chars().count());
                let mut f_column = f_column.into_iter();
                const N_COLUMN_TITLE: &str = "h./neut";
                let n_column = vec![n_nom_sg, n_acc_sg, n_dat_sg, n_gen_sg, n_nom_pl, n_acc_pl, n_dat_pl, n_gen_pl];
                let n_column_width = n_column.iter().map(|f| f.chars().count()).max().unwrap().max(N_COLUMN_TITLE.chars().count());
                let mut n_column = n_column.into_iter();

                format!(r"```
  eintal/sg     | {:m$} | {:f$} | {:n$} |
hvørfall/nom    | {:m$} | {:f$} | {:n$} |
hvønnfall/acc   | {:m$} | {:f$} | {:n$} |
hvørjumfall/dat | {:m$} | {:f$} | {:n$} |
hvørsfall/gen   | {:m$} | {:f$} | {:n$} |
  fleirtal/pl.
hvørfall/nom    | {:m$} | {:f$} | {:n$} |
hvønnfall/acc   | {:m$} | {:f$} | {:n$} |
hvørjumfall/dat | {:m$} | {:f$} | {:n$} |
hvørsfall/gen   | {:m$} | {:f$} | {:n$} |
```", M_COLUMN_TITLE, F_COLUMN_TITLE, N_COLUMN_TITLE,
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(),
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(),
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(),
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(),
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(),
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(),
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(),
                    m_column.next().unwrap(), f_column.next().unwrap(), n_column.next().unwrap(), m = m_column_width, f = f_column_width, n = n_column_width)
            }
            f => format!("Unknown inflectional paradigm:\n{}", f.join(", ")),
        }
    }
}


#[cfg(not(feature = "from_res_error_resolve"))]
#[inline(always)]
async fn from_res(res: Response) -> SprotinResponse {
    res.json().await.unwrap()
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

pub async fn search(dictionary_id: u8, dictionary_page: u16, search_for: &str, search_inflections: bool, search_descriptions: bool) -> Result<SprotinResponse, u16> {
    // This one doesn't seem to make a difference
    const SKIP_OTHER_DICTIONARIES_RESULTS: bool = true;
    // This is one gives us similar word suggestions if no results were found
    const SKIP_SIMILAR_WORDS: bool = false;

    let res = reqwest_get(&format!("https://sprotin.fo/dictionary_search_json.php?DictionaryId={}&DictionaryPage={}&SearchFor={}&SearchInflections={}&SearchDescriptions={}&Group={}&SkipOtherDictionariesResults={}&SkipSimilarWords={}",
        dictionary_id, dictionary_page, search_for, search_inflections as u8, search_descriptions as u8, "", SKIP_OTHER_DICTIONARIES_RESULTS as u8, SKIP_SIMILAR_WORDS as u8)).await.unwrap();

    if res.status().is_success() {
        Ok(from_res(res).await)
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
        27 => "FØ-JA",
        28 => "JA-FØ",
        15 => "SAM",
        25 => "NAVN",
        22 => "ALFR",
        23 => "TILT",
        13 => "YRK",
        32 => "BUSK",
        _ => "????",
    }
}

impl SprotinResponse {
    pub fn word(&self, word_nr: NonZeroUsize) -> Option<MsgBunch> {
        let mut mmb = MsgBunchBuilder::new();

        self.words.get(word_nr.get()-1)?.to_full_string(&mut mmb);

        Some(mmb.build())
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

        let mut mmb = MsgBunchBuilder::new();

        mmb.begin_section();

        if let Some(message) = &message {
            mmb.add_string("__").add_string(message).add_string("__\n").end_section().begin_section();
        }
        mmb.add_string(&format!("Síða {}. Vísir úrslit {} - {} av {} ({:.3} sekund)\n", page, from, to, total, time)).end_section().begin_section();
        for result in dictionaries_results.into_iter().filter(|r| r.results > 0) {
            mmb.add_string("**").add_string(dictionary_name(result.id)).add_string("** ").add_string(&format!("{}", result.results)).add_string(" ");
        }
        mmb.add_string("\n\n").end_section();

        match status {
            ResponseStatus::NotFound => {
                if !similar_words.is_empty() {
                    let similar_words: Vec<_> = similar_words.into_iter().map(|w| format!("_{}_", w.search_word)).collect();

                    mmb.begin_section().add_string("Meinti tú: ").add_string(&similar_words.join(", ")).end_section();
                }
            },
            ResponseStatus::Success => {
                match &*words {
                    [word] => {
                        word.to_full_string(mmb.begin_section().add_string("1. "));
                    }
                    _ => {
                        // Only show 50
                        if words.len() > 50 {
                            words.resize_with(50, || unreachable!());
                        }
                        for (i, word) in (1..).zip(words.into_iter()) {
                            mmb.begin_section().add_string(&format!("{}. {}\n", i, word.to_short_string())).end_section();
                        }
                    }
                }
            }
        }
        mmb.build()
    }
}