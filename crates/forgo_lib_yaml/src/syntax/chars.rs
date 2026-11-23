//! YAML 1.2.2 character classes (spec chapter 5).

/// `c-printable`: the characters a YAML stream may contain.
#[inline]
pub(crate) fn is_printable(c: char) -> bool {
    matches!(c,
        '\t' | '\n' | '\r' | '\u{20}'..='\u{7E}' | '\u{85}'
        | '\u{A0}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

/// `nb-json`: allowed inside quoted scalars (tab and everything from space up).
#[inline]
pub(crate) fn is_json_char(c: char) -> bool {
    c == '\t' || c >= '\u{20}'
}

/// `b-char`. NEL, LS and PS are ordinary characters in YAML 1.2.
#[inline]
pub(crate) fn is_break(c: char) -> bool {
    c == '\n' || c == '\r'
}

/// `s-white`.
#[inline]
pub(crate) fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// `nb-char`: printable, not a line break, not a byte order mark.
#[inline]
pub(crate) fn is_nb_char(c: char) -> bool {
    is_printable(c) && !is_break(c) && c != '\u{FEFF}'
}

/// `ns-char`: an `nb-char` that isn't white space.
#[inline]
pub(crate) fn is_ns_char(c: char) -> bool {
    is_nb_char(c) && !is_blank(c)
}

/// `c-flow-indicator`.
#[inline]
pub(crate) fn is_flow_indicator(c: char) -> bool {
    matches!(c, ',' | '[' | ']' | '{' | '}')
}

/// `c-indicator`.
#[inline]
pub(crate) fn is_indicator(c: char) -> bool {
    matches!(
        c,
        '-' | '?'
            | ':'
            | ','
            | '['
            | ']'
            | '{'
            | '}'
            | '#'
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
    )
}

/// `ns-word-char`: ASCII letters, digits and `-`.
#[inline]
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// `ns-uri-char`, except that `%` is accepted alone; callers validate the
/// two hex digits that must follow it.
#[inline]
pub(crate) fn is_uri_char(c: char) -> bool {
    is_word_char(c)
        || matches!(
            c,
            '%' | '#'
                | ';'
                | '/'
                | '?'
                | ':'
                | '@'
                | '&'
                | '='
                | '+'
                | '$'
                | ','
                | '_'
                | '.'
                | '!'
                | '~'
                | '*'
                | '\''
                | '('
                | ')'
                | '['
                | ']'
        )
}

/// `ns-tag-char`: a URI character other than `!` and the flow indicators.
#[inline]
pub(crate) fn is_tag_char(c: char) -> bool {
    is_uri_char(c) && c != '!' && !is_flow_indicator(c)
}

/// `ns-anchor-char`: an `ns-char` other than the flow indicators.
#[inline]
pub(crate) fn is_anchor_char(c: char) -> bool {
    is_ns_char(c) && !is_flow_indicator(c)
}

/// `ns-plain-safe(c)`: in flow context the flow indicators end a plain scalar.
#[inline]
pub(crate) fn is_plain_safe(c: char, flow: bool) -> bool {
    is_ns_char(c) && !(flow && is_flow_indicator(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printable_set() {
        assert!(is_printable('a') && is_printable('\t') && is_printable('\u{85}'));
        assert!(is_printable('\u{FEFF}'));
        for c in ['\0', '\u{7}', '\u{7F}', '\u{9F}', '\u{FFFE}', '\u{FFFF}'] {
            assert!(!is_printable(c), "{c:?}");
        }
        assert!(!is_nb_char('\u{FEFF}') && !is_ns_char(' '));
        assert!(is_nb_char('\u{2028}'), "LS is not a line break in YAML 1.2");
    }

    #[test]
    fn plain_safety_depends_on_context() {
        assert!(is_plain_safe(']', false));
        assert!(!is_plain_safe(']', true));
        assert!(is_anchor_char(':') && !is_anchor_char(','));
        assert!(is_tag_char('%') && !is_tag_char('!') && !is_tag_char('{'));
    }
}
