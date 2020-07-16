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
        body = body.replace("&ocedil;", "");
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

#[derive(Debug, Default)]
pub struct MsgBunch {
    pub messages: Vec<String>,
}

impl MsgBunch {
    pub fn new() -> Self {
        MsgBunch {
            messages: vec![String::with_capacity(MSG_LIMIT)]
        }
    }
    pub fn add_string(&mut self, mut s: &str) -> &mut Self {
        let mut len = self.messages.last().unwrap().len();

        if len + s.len() > MSG_LIMIT {
            while len + s.len() > MSG_LIMIT {
                let split_index = MSG_LIMIT - len;

                {
                    let last_message = self.messages.last_mut().unwrap();
                    last_message.push_str(&s[..split_index]);
                    debug_assert_eq!(last_message.len(), MSG_LIMIT);
                }
                self.messages.push(String::with_capacity(MSG_LIMIT));

                len = 0;
                s = &s[split_index..];
            }
        }

        let last_message = self.messages.last_mut().unwrap();
        last_message.push_str(s);

        self
    }
    pub fn entries(&mut self, entries: Vec<Entry>) -> &mut Self {
        self.messages.push(String::with_capacity(MSG_LIMIT));

        for entry in entries {
            let entry_text = format!("{}\n", entry);

            self.add_string(&entry_text);
        }

        self
    }
}