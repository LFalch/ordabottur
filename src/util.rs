#![warn(missing_docs)]
//! Module with various useful things for manipulating text for use with Discord.

#[derive(Debug, Clone, Eq, PartialEq)]
/// A dictionary entry from Grunnmanuskriptet
pub struct Entry {
    word: String,
    class: String,
    body: String,
}

impl Entry {
    #[inline(always)]
    /// Create a new entry from the word form, its class (part of speech)
    /// and a body with an explanation of the word.
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
/// A collection of strings that each are below 2000 characters
/// so that they can be sent as each their own Discord message
pub struct MsgBunch {
    /// The strings
    /// 
    /// ## Note:
    /// 
    /// It's a logic error to make any of these strings bigger than 2000
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
/// Builder for an [`MsgBunch`]
/// 
/// Has functionality that can be used to help not
/// ruining formatting across multiple messages
/// by marking certain sections of texts as fitting together
/// and therefore not ideal to split.
/// 
/// The message builder will instead try its best to only split between sections.
/// Although if a section is too big, you can specify how it should try that section,
/// or - by default - it will try to only split at punctuation or parentheses.
pub struct MsgBunchBuilder {
    /// The partial bunch being built
    pub inner: MsgBunch,
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
    /// Makes a new empty bunch
    /// 
    /// Allocates 2000 bytes of space
    pub fn new() -> Self {
        MsgBunchBuilder {
            inner: MsgBunch::new(),
            chars_num: 0,
            no_split_section: None,
        }
    }

    /// Adds a string to the message bunch.
    /// If the builder is currently in a section
    /// it will add to that section in order to manage where messages are split.
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
            self.chars_num += string_to_add_size;
        }
        self
    }

    /// Puts all following text into a section.
    /// 
    /// Calling this multiple times before ending the section does nothing
    pub fn begin_section(&mut self) -> &mut Self {
        if self.no_split_section.is_none() {
            self.no_split_section = Some((String::new(), 0));
        }
        self
    }

    #[inline]
    /// Whether a section has previously been begun and not yet ended.
    pub fn is_in_section(&self) -> bool {
        self.no_split_section.is_some()
    }

    #[inline]
    /// Ends the section trying to split at section borders if a message gets too big.
    /// The same as `end_section_with` but with a default set of characters to split at.
    pub fn end_section(&mut self) -> &mut Self {
        self.end_section_with(|c| matches!(c, ';' | ',' | '.' | '?' | '!' | ')' | ':' | '-'))
    }
    
    /// Ends this section trying to split at section borders, but if the section is too big
    /// uses the given callback to determine what characters to split at.
    /// 
    /// Does nothing if no section currently has begun.
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

    /// Adds each of the lines in the given string as a section.
    ///
    /// Adds a trailing newline even if none is in the given string
    pub fn add_lines<S: AsRef<str>>(&mut self, lines: S) -> &mut Self {
        for line in lines.as_ref().lines() {
            self.begin_section().add_string(line).add_string("\n").end_section();
        }

        self
    }

    /// Adds an [`Entry`] to the message bunch.
    /// 
    /// This does not make its own section, and if you need it to be a section,
    /// you should call `.begin_section()` and `.end_section()` around it.
    pub fn entries(&mut self, entries: Vec<Entry>) -> &mut Self {
        for entry in entries {
            self.add_lines(entry.to_string());
        }

        self
    }

    #[inline]
    /// Ends whatever sections the last begun section if there was one
    /// and returns the resulting bunch.
    pub fn build(mut self) -> MsgBunch {
        self.end_section();
        self.inner
    }
}

/// Splits the string into three with the beginning whitespace in the 0th field
/// and the trailing whitespace in the 2nd field.Activity
/// 
/// If the string contains only whitespace, it'll be considered trailing whitespace
/// and fields 0 and 1 will be empty
pub fn split_trim(s: &str) -> (&str, &str, &str) {
    let end_trim_index = s.rfind(|c: char| !c.is_whitespace()).map(|i| {
        i + s[i..].chars().next().unwrap().len_utf8()
    }).unwrap_or(0);
    
    let (start, end_trim) = s.split_at(end_trim_index);
    
    let front_trim_index = start.find(|c: char| !c.is_whitespace()).unwrap_or(end_trim_index);

    let (front_trim, text) = start.split_at(front_trim_index);

    (front_trim, text, end_trim)
}

#[cfg(test)]
mod tests {
    use super::split_trim;
    #[test]
    fn test_split_trim() {
        assert_eq!(split_trim("hestetest"), ("", "hestetest", ""));
        assert_eq!(split_trim("   hest  \n\n asdg \t\n"), ("   ", "hest  \n\n asdg", " \t\n"));
        assert_eq!(split_trim("\n"), ("", "", "\n"));
        assert_eq!(split_trim(" "), ("", "", " "));
    }
}

/// Turns - and 0-9 into their superscript variants.
/// Passes other characters through without change.
pub fn to_super(c: char) -> char {
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
/// Turns - and 0-9 into their subscript variants.
/// Passes other characters through without change.
pub fn to_sub(c: char) -> char {
    match c {
        '-' => '₋',
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        c => c
    }
}

#[inline]
/// Applies [`to_sup`] to each of the characters in a given string.
pub fn to_superscript(src: &str) -> String {
    src.chars().map(to_super).collect()
}
#[inline]
/// Applies [`to_sub`] to each of the characters in a given string.
pub fn to_subscript(src: &str) -> String {
    src.chars().map(to_sub).collect()
}