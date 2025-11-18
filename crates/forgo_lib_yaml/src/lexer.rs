// crates/forgo_lib_yaml/src/lexer.rs
use crate::char_validator::{control_char_error_message, is_forbidden_control_char};
use crate::util::{is_plain_scalar_char, leading_spaces};

#[derive(Clone, Debug, PartialEq)]
pub struct StrTok {
    pub text: String,
    pub quoted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Newline,
    Indent(usize),
    Str(StrTok), // bare or quoted scalar token
    /// '---' at beginning of a line (possibly followed by whitespace/comments)
    DocStart,
    /// '...' at beginning of a line (possibly followed by whitespace/comments)
    DocEnd,
    Colon,
    Dash, // "- " sequence marker
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Pipe,            // |
    Gt,              // >
    Plus,            // + (for chomping)
    Minus,           // - (for chomping)
    Amp,             // & (anchor)
    Star,            // * (alias)
    Question,        // ? (complex key marker)
    Tag(String),     // YAML tag: !str, !!str, !e!thing, !<tag:example.com,2000:app/other>
    Comment(String), // "# ..." until EOL, without '#'
    Eof,
    TabIndent, // leading '\t' used for indentation (error)
    Error(String), // Lexer error with message
}

#[derive(Clone)]
pub struct Lexer<'a> {
    s: &'a str,
    pub(crate) i: usize,
    pub(crate) line_start: bool,
    pub(crate) saw_cr: bool,
}

impl<'a> Lexer<'a> {
    #[inline]
    pub(crate) fn bump_newline(&mut self) {
        // Consume '\n', '\r\n', or bare '\r'
        match self.peek() {
            Some('\n') => {
                self.bump();
            }
            Some('\r') => {
                self.bump();
                self.saw_cr = true; // record CR seen
                if matches!(self.peek(), Some('\n')) {
                    self.bump();
                }
            }
            _ => {}
        }
        self.line_start = true;
    }
    pub fn new(s: &'a str) -> Self {
        let mut me = Self {
            s,
            i: 0,
            line_start: true,
            saw_cr: false,
        };
        // Skip UTF-8 BOM if present
        if me.peek() == Some('\u{FEFF}') {
            me.bump();
        }
        me
    }

    fn peek_slice(&self, n: usize) -> Option<&'a str> {
        let mut end = self.i;
        for _ in 0..n {
            if let Some(ch) = self.s[end..].chars().next() {
                end += ch.len_utf8();
            } else {
                return None;
            }
        }
        Some(&self.s[self.i..end])
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }
    pub(crate) fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.i += ch.len_utf8();
        Some(ch)
    }
    fn eat_while<F: Fn(char) -> bool>(&mut self, f: F) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if f(c) {
                out.push(c);
                self.bump();
            } else {
                break;
            }
        }
        out
    }

    pub fn next_token(&mut self) -> Tok {
        if self.i >= self.s.len() {
            return Tok::Eof;
        }

        // Check for forbidden control characters
        if let Some(ch) = self.peek() {
            if is_forbidden_control_char(ch) {
                self.bump(); // consume the bad character
                return Tok::Error(control_char_error_message(ch));
            }
        }

        if self.line_start {
            // compute indent or detect blank/comment lines
            let spaces = leading_spaces(&self.s[self.i..]);
            self.i += spaces;

            // Reject a literal tab that appears in the indentation column
            if matches!(self.peek(), Some('\t')) {
                self.bump();
                self.line_start = true;
                return Tok::TabIndent;
            }

            // Blank line?
            match self.peek() {
                Some('\n') | Some('\r') => {
                    self.bump_newline();
                    return Tok::Newline;
                }
                Some('#') => {
                    self.bump();
                    let text = self
                        .eat_while(|c| c != '\n' && c != '\r')
                        .trim()
                        .to_string();
                    self.bump_newline();
                    return Tok::Comment(text);
                }
                Some(_) => {
                    // >>> NEW: detect '---' / '...' at true BOL (indent must be 0)
                    if spaces == 0 {
                        if let Some(s3) = self.peek_slice(3) {
                            if s3 == "---" || s3 == "..." {
                                // consume marker
                                self.bump();
                                self.bump();
                                self.bump();
                                // Only consume whitespace after the marker
                                // If we hit '#', consume comment. Otherwise, leave content for next token.
                                while let Some(c) = self.peek() {
                                    if c == ' ' {
                                        self.bump();
                                    } else if c == '#' {
                                        // consume inline comment
                                        self.bump();
                                        while let Some(c2) = self.peek() {
                                            if c2 == '\n' || c2 == '\r' {
                                                break;
                                            }
                                            self.bump();
                                        }
                                        break;
                                    } else {
                                        // Content on same line as marker - leave it for next token
                                        break;
                                    }
                                }
                                // we did not consume newline; next call will return Newline or content
                                self.line_start = false;
                                return if s3 == "---" {
                                    Tok::DocStart
                                } else {
                                    Tok::DocEnd
                                };
                            }
                        }
                    }
                    // otherwise: it's a content line -> return its logical indent
                    self.line_start = false;
                    return Tok::Indent(spaces);
                }
                None => return Tok::Eof,
            }
        }

        match self.peek().unwrap() {
            '\r' => {
                self.bump_newline();
                Tok::Newline
            }
            '\n' => {
                self.bump_newline();
                Tok::Newline
            }
            ':' => {
                self.bump();
                Tok::Colon
            }
            '[' => {
                self.bump();
                Tok::LBracket
            }
            ']' => {
                self.bump();
                Tok::RBracket
            }
            '{' => {
                self.bump();
                Tok::LBrace
            }
            '}' => {
                self.bump();
                Tok::RBrace
            }
            ',' => {
                self.bump();
                Tok::Comma
            }
            '|' => {
                self.bump();
                Tok::Pipe
            }
            '>' => {
                self.bump();
                Tok::Gt
            }
            '+' => {
                self.bump();
                Tok::Plus
            }
            '-' => {
                self.bump();
                // Check what follows the '-'
                match self.peek() {
                    Some(' ') => {
                        self.bump();
                        Tok::Dash // '- ' for list items
                    }
                    Some('\t') | Some('\n') | Some('\r') | None => {
                        Tok::Minus // standalone '-' (chomping indicator)
                    }
                    Some(c) if is_plain_scalar_char(c) => {
                        // '-' followed by a plain scalar char - collect as part of plain scalar
                        let mut word = String::from("-");
                        word.push_str(&self.eat_while(is_plain_scalar_char));
                        Tok::Str(StrTok {
                            text: word,
                            quoted: false,
                        })
                    }
                    _ => {
                        Tok::Minus // standalone '-' before special char
                    }
                }
            }
            '&' => {
                self.bump();
                Tok::Amp
            }
            '*' => {
                self.bump();
                Tok::Star
            }
            '?' => {
                self.bump();
                Tok::Question
            }
            '!' => {
                // YAML tag: !str, !!str, !e!thing, !<tag:example.com,2000:app/other>
                self.bump();
                let mut tag = String::from("!");

                // Check for verbatim tag: !<...>
                if self.peek() == Some('<') {
                    self.bump();
                    tag.push('<');
                    // Collect everything until '>'
                    while let Some(ch) = self.peek() {
                        self.bump();
                        tag.push(ch);
                        if ch == '>' {
                            break;
                        }
                    }
                    return Tok::Tag(tag);
                }

                // Check for !! (secondary tag namespace)
                if self.peek() == Some('!') {
                    self.bump();
                    tag.push('!');
                }

                // Collect tag name: letters, digits, '-', '_' (and potentially '!' for named handles)
                while let Some(ch) = self.peek() {
                    if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '!' {
                        self.bump();
                        tag.push(ch);
                    } else {
                        break;
                    }
                }

                Tok::Tag(tag)
            }
            '"' => {
                self.bump();
                let mut out = String::new();
                let mut terminated = false;
                while let Some(c) = self.peek() {
                    self.bump();
                    match c {
                        '"' => {
                            terminated = true;
                            break;
                        }
                        '\\' => {
                            if let Some(n) = self.peek() {
                                self.bump();
                                match n {
                                    // YAML 1.2.2 Section 5.7 - Escaped Characters
                                    '0' => out.push('\0'),           // null
                                    'a' => out.push('\x07'),         // bell
                                    'b' => out.push('\x08'),         // backspace
                                    't' => out.push('\t'),           // tab
                                    'n' => out.push('\n'),           // line feed
                                    'v' => out.push('\x0B'),         // vertical tab
                                    'f' => out.push('\x0C'),         // form feed
                                    'r' => out.push('\r'),           // carriage return
                                    'e' => out.push('\x1B'),         // escape
                                    ' ' => out.push(' '),            // space
                                    '"' => out.push('"'),            // double quote
                                    '/' => out.push('/'),            // slash
                                    '\\' => out.push('\\'),          // backslash
                                    'N' => out.push('\u{0085}'),     // next line (NEL)
                                    '_' => out.push('\u{00A0}'),     // non-breaking space
                                    'L' => out.push('\u{2028}'),     // line separator
                                    'P' => out.push('\u{2029}'),     // paragraph separator
                                    '\n' | '\r' => {
                                        // Escaped line break - line folding
                                        // Consume the newline and any following whitespace/indentation
                                        if n == '\r' {
                                            // Check for CRLF
                                            if self.peek() == Some('\n') {
                                                self.bump();
                                            }
                                        }
                                        // Skip any leading whitespace on the next line
                                        while let Some(ch) = self.peek() {
                                            if matches!(ch, ' ' | '\t') {
                                                self.bump();
                                            } else {
                                                break;
                                            }
                                        }
                                        // The newline and whitespace are removed (line folding)
                                    }
                                    'x' => {
                                        // \xNN - 2 hex digits
                                        let mut hex = String::new();
                                        for _ in 0..2 {
                                            if let Some(h) = self.peek() {
                                                if h.is_ascii_hexdigit() {
                                                    self.bump();
                                                    hex.push(h);
                                                } else {
                                                    return Tok::Error(format!("Invalid or incomplete \\x escape: expected 2 hex digits, got '{}'", hex));
                                                }
                                            } else {
                                                return Tok::Error(format!("Incomplete \\x escape: expected 2 hex digits, got '{}'", hex));
                                            }
                                        }
                                        if let Ok(code) = u8::from_str_radix(&hex, 16) {
                                            out.push(code as char);
                                        } else {
                                            return Tok::Error(format!("Invalid \\x escape sequence: \\x{}", hex));
                                        }
                                    }
                                    'u' => {
                                        // \uNNNN - 4 hex digits
                                        let mut hex = String::new();
                                        for _ in 0..4 {
                                            if let Some(h) = self.peek() {
                                                if h.is_ascii_hexdigit() {
                                                    self.bump();
                                                    hex.push(h);
                                                } else {
                                                    return Tok::Error(format!("Invalid or incomplete \\u escape: expected 4 hex digits, got '{}'", hex));
                                                }
                                            } else {
                                                return Tok::Error(format!("Incomplete \\u escape: expected 4 hex digits, got '{}'", hex));
                                            }
                                        }
                                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                            if let Some(ch) = char::from_u32(code) {
                                                // Check for invalid surrogates
                                                if (0xD800..=0xDFFF).contains(&code) {
                                                    return Tok::Error(format!("Invalid \\u escape: U+{:04X} is a surrogate code point", code));
                                                }
                                                out.push(ch);
                                            } else {
                                                return Tok::Error(format!("Invalid \\u escape: U+{:04X} is not a valid Unicode scalar", code));
                                            }
                                        } else {
                                            return Tok::Error(format!("Invalid \\u escape sequence: \\u{}", hex));
                                        }
                                    }
                                    'U' => {
                                        // \UNNNNNNNN - 8 hex digits
                                        let mut hex = String::new();
                                        for _ in 0..8 {
                                            if let Some(h) = self.peek() {
                                                if h.is_ascii_hexdigit() {
                                                    self.bump();
                                                    hex.push(h);
                                                } else {
                                                    return Tok::Error(format!("Invalid or incomplete \\U escape: expected 8 hex digits, got '{}'", hex));
                                                }
                                            } else {
                                                return Tok::Error(format!("Incomplete \\U escape: expected 8 hex digits, got '{}'", hex));
                                            }
                                        }
                                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                            if let Some(ch) = char::from_u32(code) {
                                                // Check for invalid surrogates
                                                if (0xD800..=0xDFFF).contains(&code) {
                                                    return Tok::Error(format!("Invalid \\U escape: U+{:08X} is a surrogate code point", code));
                                                }
                                                out.push(ch);
                                            } else {
                                                return Tok::Error(format!("Invalid \\U escape: U+{:08X} is not a valid Unicode scalar", code));
                                            }
                                        } else {
                                            return Tok::Error(format!("Invalid \\U escape sequence: \\U{}", hex));
                                        }
                                    }
                                    other => {
                                        return Tok::Error(format!("Invalid escape sequence: \\{}", other));
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        _ => out.push(c),
                    }
                }
                if !terminated {
                    return Tok::Error("Unterminated double-quoted string".to_string());
                }
                Tok::Str(StrTok {
                    text: out,
                    quoted: true,
                })
            }
            '\'' => {
                self.bump();
                let mut out = String::new();
                let mut terminated = false;
                while let Some(c) = self.peek() {
                    self.bump();
                    match c {
                        '\'' => {
                            // Check for escaped single quote ''
                            if matches!(self.peek(), Some('\'')) {
                                self.bump();
                                out.push('\'');
                            } else {
                                terminated = true;
                                break;
                            }
                        }
                        _ => out.push(c),
                    }
                }
                if !terminated {
                    return Tok::Error("Unterminated single-quoted string".to_string());
                }
                Tok::Str(StrTok {
                    text: out,
                    quoted: true,
                })
            }
            ' ' => {
                self.eat_while(|c| c == ' ');
                self.next_token()
            }
            '#' => {
                self.bump();
                let text = self.eat_while(|c| c != '\n').trim().to_string();
                Tok::Comment(text)
            }
            c if is_plain_scalar_char(c) => {
                let word = self.eat_while(is_plain_scalar_char);
                Tok::Str(StrTok {
                    text: word,
                    quoted: false,
                })
            }
            other => {
                self.bump();
                Tok::Str(StrTok {
                    text: other.to_string(),
                    quoted: false,
                })
            }
        }
    }
}
