#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Entry {
    word: String,
    class: String,
    body: String,
}

impl Entry {
    #[inline(always)]
    pub fn new_gm(word: String, class: String, mut body: String) -> Self {
        body = body.replace("&amp;", "&");
        body = body.replace("`", "\\`");

        // TODO HACK
        body = body.replace("&emacr;", "ē");
        body = body.replace("&oelig;", "ø");
        body = body.replace("&Oogon;", "Ǫ");
        body = body.replace("&nbsp;", " ");
        body = body.replace("&oogon;", "ǫ");
        body = body.replace("&omacr;", "ō");
        body = body.replace("&ocedil;", "ǫ");
        body = body.replace("&kryss;", "♯");
        body = body.replace("&divide;", "÷");
        body = body.replace("&dagger;", "†");

        Entry {
            word, class, body
        }
    }
}

use std::fmt::{self, Display};

impl Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Entry{word, class, body} = self;

        write!(f, "**{}** _{}_{}{}", word, class, if body.len() > 20 {"\n"} else {": "}, body)
    }
}


const MSG_LIMIT: usize = 2000;

#[derive(Debug, Default, Clone)]
pub struct MsgBunch {
    pub messages: Vec<String>,
}

impl MsgBunch {
    fn new() -> Self {
        MsgBunch {
            messages: vec![String::with_capacity(MSG_LIMIT)]
        }
    }
}

#[derive(Debug)]
pub struct MsgBunchBuilder {
    inner: MsgBunch,
    chars_num: usize, 
    no_split_section: Option<(String, usize)>,
}

impl Default for MsgBunchBuilder {
    #[inline(always)]
    fn default() -> Self {
        MsgBunchBuilder::new()
    }
}

impl MsgBunchBuilder {
    #[inline]
    pub fn new() -> Self {
        MsgBunchBuilder {
            inner: MsgBunch::new(),
            chars_num: 0,
            no_split_section: None,
        }
    }

    pub fn add_string<S: AsRef<str>>(&mut self, s: S) -> &mut Self {
        let string_to_add = s.as_ref();
        let string_to_add_size = string_to_add.chars().count();

        if let Some((no_split_section, size)) = &mut self.no_split_section {
            *size += string_to_add_size;
            no_split_section.push_str(string_to_add);
        } else if self.chars_num + string_to_add_size > MSG_LIMIT {
            let cur_msg = self.inner.messages.last_mut().unwrap();
            let cur_msg_size = cur_msg.chars().count();

            let (s, index) = (cur_msg_size+1..).zip(string_to_add.char_indices()).map(|(s, (i, _))| (s, i)).nth(MSG_LIMIT-cur_msg_size).unwrap();
            debug_assert_eq!(s, MSG_LIMIT);

            cur_msg.push_str(&string_to_add[..index]);

            let new_cur_msg = string_to_add[index..].to_owned();
            let new_cur_msg_size = new_cur_msg.chars().count();

            self.inner.messages.push(string_to_add[index..].to_owned());
            self.chars_num = new_cur_msg_size;
        } else {
            self.inner.messages.last_mut().unwrap().push_str(string_to_add);
            self.inner.messages.push(String::with_capacity(MSG_LIMIT));
            self.chars_num = 0;
        }
        self
    }

    pub fn begin_section(&mut self) -> &mut Self {
        if self.no_split_section.is_none() {
            self.no_split_section = Some((String::new(), 0));
        }
        self
    }

    #[inline]
    pub fn is_in_section(&self) -> bool {
        self.no_split_section.is_some()
    }

    #[inline]
    pub fn end_section(&mut self) -> &mut Self {
        self.end_section_with(|c| match c {
            ';' | ',' | '.' | '?' | '!' | ')' | ':' | '-' => true,
            _ => false
        })
    }

    pub fn end_section_with<F: FnMut(char) -> bool>(&mut self, mut f: F) -> &mut Self {
        if let Some((mut no_split_section, size)) = self.no_split_section.take() {
            if self.chars_num + size > MSG_LIMIT {
                self.chars_num = size;

                let mut no_split_section_size = no_split_section.chars().count();

                // If the section is longer than the msg limit, we have to split it anyway
                // using the passed function to check charactes that should allow splits
                while no_split_section_size > MSG_LIMIT {
                    // take(MSG_LIMIT) so that it'll panic if it doesn't find something to split at before message limit
                    let (mut index, _) = no_split_section.char_indices().rev().skip(no_split_section_size-MSG_LIMIT).take(MSG_LIMIT).find(|(_, c)| f(*c)).unwrap();
                    index += 1;

                    while !no_split_section.is_char_boundary(index) {
                        index += 1;
                    }

                    let new_cur_msg = no_split_section.split_off(index);

                    let first_section = std::mem::replace(&mut no_split_section, new_cur_msg);
                    no_split_section_size = no_split_section.chars().count();

                    self.inner.messages.push(first_section);
                }
                self.inner.messages.push(no_split_section);
            } else {
                self.chars_num += size;
                self.inner.messages.last_mut().unwrap().push_str(&no_split_section)
            }
        }
        self
    }

    pub fn add_lines<S: AsRef<str>>(&mut self, lines: S) -> &mut Self {
        for line in lines.as_ref().lines() {
            self.begin_section().add_string(line).end_section();
        }

        self
    }

    pub fn entries(&mut self, entries: Vec<Entry>) -> &mut Self {
        for entry in entries {
            self.add_lines(entry.to_string());
        }

        self
    }

    #[inline]
    pub fn build(mut self) -> MsgBunch {
        self.end_section();
        self.inner
    }
}

pub fn num_to_super(c: char) -> char {
    match c {
        '-' => '⁻',
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        c => c
    }
}

#[inline]
pub fn to_superscript(src: &str) -> String {
    src.chars().map(num_to_super).collect()
}