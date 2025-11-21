// crates/forgo_lib_yaml/src/lexer.rs
use crate::char_validator::{control_char_error_message, is_forbidden_control_char};
use crate::util::{is_plain_scalar_char, leading_spaces};

#[derive(Clone, Debug, PartialEq)]
pub struct StrTok {
    pub text: String,
    pub quoted: bool,
    /// True if this quoted string spanned multiple source lines (contains literal newlines in source)
    pub multiline: bool,
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
    /// Track flow collection depth: incremented by '[' and '{', decremented by ']' and '}'
    /// Used to determine context for plain scalar character validation
    flow_depth: usize,
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
            flow_depth: 0,
        };
        // Skip UTF-8 BOM if present
        if me.peek() == Some('\u{FEFF}') {
            me.bump();
        }
        me
    }

    /// Check if a character is valid in a plain scalar given current context
    /// Per YAML 1.2.2 §7.3.3: In block context, ] and } are allowed in plain scalars
    /// In flow context, they terminate flow collections
    fn is_plain_scalar_char_in_context(&self, c: char) -> bool {
        // In block context (flow_depth == 0), allow ] and }
        if self.flow_depth == 0 {
            match c {
                ']' | '}' => return true,
                _ => {}
            }
        }
        // For all other cases, use the standard check
        is_plain_scalar_char(c)
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

            // Handle tabs after leading spaces
            // Per YAML 1.2.2 §6.2: Tabs are *forbidden* for determining indentation level
            // but allowed as separators between tokens
            //
            // The key insight: if a line starts with spaces followed by a tab followed by
            // non-whitespace content, the tab is being used for indentation (invalid).
            // But if the line has content, then tabs within that content are separators (valid).
            //
            // For now, reject tabs that appear to establish indentation (no preceding spaces,
            // or tabs used to indent nested content)
            if spaces == 0 && matches!(self.peek(), Some('\t')) {
                // Tab at column 0 - definitely indentation, not separator
                return Tok::TabIndent;
            }

            // Blank line or space-only line?
            match self.peek() {
                Some('\n') | Some('\r') => {
                    // YAML 1.2.2 §8.1.3: For block scalars, space-only lines with indentation
                    // exceeding the content indent create ambiguity. To allow the parser to
                    // validate this, emit Tok::Indent(spaces) even for blank lines, rather
                    // than consuming them as Tok::Newline immediately.
                    //
                    // Only do this if spaces > 0 - truly empty lines remain as Tok::Newline
                    if spaces > 0 {
                        self.line_start = false;
                        return Tok::Indent(spaces);
                    }
                    // Empty line (no spaces): return Newline directly
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
                self.flow_depth += 1;
                Tok::LBracket
            }
            ']' => {
                self.bump();
                if self.flow_depth > 0 {
                    self.flow_depth -= 1;
                }
                // YAML 1.2.2 §6.5: Comments must be separated from content by whitespace
                // Check if '#' immediately follows without whitespace
                if matches!(self.peek(), Some('#')) {
                    return Tok::Error("comment must be separated from content by whitespace".into());
                }
                Tok::RBracket
            }
            '{' => {
                self.bump();
                self.flow_depth += 1;
                Tok::LBrace
            }
            '}' => {
                self.bump();
                if self.flow_depth > 0 {
                    self.flow_depth -= 1;
                }
                Tok::RBrace
            }
            ',' => {
                self.bump();
                // YAML 1.2.2 §6.5: Comments must be separated from content by whitespace
                // Check if '#' immediately follows without whitespace
                if matches!(self.peek(), Some('#')) {
                    return Tok::Error("comment must be separated from content by whitespace".into());
                }
                Tok::Comma
            }
            '|' => {
                self.bump();
                // YAML 1.2.2 §6.5: Comments must be separated from content by whitespace
                // Check if '#' immediately follows without whitespace (like "|#comment")
                if matches!(self.peek(), Some('#')) {
                    return Tok::Error("comment must be separated from content by whitespace".into());
                }
                Tok::Pipe
            }
            '>' => {
                self.bump();
                // YAML 1.2.2 §6.5: Comments must be separated from content by whitespace
                // Check if '#' immediately follows without whitespace (like ">#comment")
                if matches!(self.peek(), Some('#')) {
                    return Tok::Error("comment must be separated from content by whitespace".into());
                }
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
                    Some(c) if self.is_plain_scalar_char_in_context(c) => {
                        // '-' followed by a plain scalar char - collect as part of plain scalar
                        let mut word = String::from("-");
                        let flow_depth = self.flow_depth;
                        word.push_str(&self.eat_while(|ch| {
                            if flow_depth == 0 && matches!(ch, ']' | '}') {
                                true
                            } else {
                                is_plain_scalar_char(ch)
                            }
                        }));
                        Tok::Str(StrTok {
                            text: word,
                            quoted: false,
                            multiline: false,
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

                // YAML 1.2.2 §6.9: Tags must be separated from following content by whitespace
                // Invalid characters like {}, comma after a tag indicate malformed syntax
                if let Some(ch) = self.peek() {
                    if matches!(ch, '{' | '}' | '[' | ']' | ',') {
                        return Tok::Error(format!("invalid tag format: '{}' cannot be followed by '{}' without whitespace", tag, ch));
                    }
                }

                Tok::Tag(tag)
            }
            '"' => {
                self.bump();
                let mut out = String::new();
                let mut terminated = false;
                let mut spanned_lines = false; // Track if string contains literal newlines in source
                while let Some(c) = self.peek() {
                    self.bump();
                    match c {
                        '"' => {
                            terminated = true;
                            break;
                        }
                        '\n' | '\r' => {
                            // Literal newline in the source (not escaped)
                            spanned_lines = true;
                            out.push(c);

                            // YAML 1.2.2 §7.3.1 & §9.1: Document markers at start of line
                            // terminate even quoted scalars
                            // Check if next line starts with --- or ... followed by whitespace/EOF
                            if let Some(s3) = self.peek_slice(3) {
                                if s3 == "---" || s3 == "..." {
                                    // Check if followed by whitespace, newline, or EOF
                                    let temp_i = self.i + 3;
                                    let is_marker = if temp_i >= self.s.len() {
                                        true // EOF after marker
                                    } else {
                                        matches!(self.s.chars().nth(temp_i), Some(' ') | Some('\t') | Some('\n') | Some('\r') | Some('#'))
                                    };

                                    if is_marker {
                                        // Document marker found - quoted string is unterminated
                                        return Tok::Error(format!(
                                            "unterminated quoted scalar: document marker '{}' at start of line inside quoted string",
                                            s3
                                        ));
                                    }
                                }
                            }
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
                // YAML 1.2.2 §6.5: Comments must be separated from content by whitespace
                // Check if '#' immediately follows closing quote without whitespace
                if matches!(self.peek(), Some('#')) {
                    return Tok::Error("comment must be separated from content by whitespace".into());
                }
                Tok::Str(StrTok {
                    text: out,
                    quoted: true,
                    multiline: spanned_lines,
                })
            }
            '\'' => {
                self.bump();
                let mut out = String::new();
                let mut terminated = false;
                let mut spanned_lines = false; // Track if string contains literal newlines in source
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
                        '\n' | '\r' => {
                            // Literal newline in the source
                            spanned_lines = true;
                            out.push(c);
                        }
                        _ => out.push(c),
                    }
                }
                if !terminated {
                    return Tok::Error("Unterminated single-quoted string".to_string());
                }
                // YAML 1.2.2 §6.5: Comments must be separated from content by whitespace
                // Check if '#' immediately follows closing quote without whitespace
                if matches!(self.peek(), Some('#')) {
                    return Tok::Error("comment must be separated from content by whitespace".into());
                }
                Tok::Str(StrTok {
                    text: out,
                    quoted: true,
                    multiline: spanned_lines,
                })
            }
            ' ' => {
                self.eat_while(|c| c == ' ');
                self.next_token()
            }
            '\t' => {
                // Tabs mid-line are treated as trailing whitespace
                // (per YAML 1.2.2 §6.2: tabs are allowed as trailing whitespace)
                self.eat_while(|c| c == ' ' || c == '\t');
                self.next_token()
            }
            '#' => {
                self.bump();
                let text = self.eat_while(|c| c != '\n').trim().to_string();
                Tok::Comment(text)
            }
            c if self.is_plain_scalar_char_in_context(c) => {
                let flow_depth = self.flow_depth;
                let word = self.eat_while(|ch| {
                    if flow_depth == 0 && matches!(ch, ']' | '}') {
                        true
                    } else {
                        is_plain_scalar_char(ch)
                    }
                });
                Tok::Str(StrTok {
                    text: word,
                    quoted: false,
                    multiline: false,
                })
            }
            other => {
                self.bump();
                Tok::Str(StrTok {
                    text: other.to_string(),
                    quoted: false,
                    multiline: false,
                })
            }
        }
    }
}
