// crates/forgo_lib_yaml/src/char_validator.rs
//! Character validation for YAML 1.2.2 compliance.
//!
//! YAML 1.2.2 Section 5.8 specifies which characters are allowed:
//! - Printable characters (x09, x0A, x0D, x20-x7E, x85, xA0-xD7FF, xE000-xFFFD, x10000-x10FFFF)
//! - Specifically FORBIDDEN: C0 controls (except tab/LF/CR), DEL, C1 controls, surrogates

/// Check if a character is a forbidden control character per YAML 1.2.2 Section 5.8
pub fn is_forbidden_control_char(c: char) -> bool {
    let code = c as u32;

    // C0 controls (0x00-0x1F) EXCEPT allowed ones (tab=0x09, LF=0x0A, CR=0x0D)
    if code <= 0x1F {
        return !matches!(code, 0x09 | 0x0A | 0x0D);
    }

    // DEL (0x7F)
    if code == 0x7F {
        return true;
    }

    // C1 controls (0x80-0x9F) EXCEPT NEL (0x85)
    if (0x80..=0x9F).contains(&code) {
        return code != 0x85;
    }

    false
}

/// Get a descriptive error message for a forbidden control character
pub fn control_char_error_message(c: char) -> String {
    let code = c as u32;

    match code {
        0x00 => "NULL character (U+0000) is not allowed".to_string(),
        0x08 => "backspace character (U+0008) is not allowed".to_string(),
        0x0B => "vertical tab character (U+000B) is not allowed".to_string(),
        0x0C => "form feed character (U+000C) is not allowed".to_string(),
        0x7F => "DELETE character (U+007F) is not allowed".to_string(),
        0x80..=0x9F if code != 0x85 => {
            format!("C1 control character (U+{:04X}) is not allowed", code)
        }
        _ if code <= 0x1F => {
            format!("control character (U+{:04X}) is not allowed", code)
        }
        _ => format!("invalid character (U+{:04X})", code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_characters() {
        // Tab, LF, CR are allowed
        assert!(!is_forbidden_control_char('\t'));
        assert!(!is_forbidden_control_char('\n'));
        assert!(!is_forbidden_control_char('\r'));

        // NEL is allowed
        assert!(!is_forbidden_control_char('\u{0085}'));

        // Regular printable chars
        assert!(!is_forbidden_control_char(' '));
        assert!(!is_forbidden_control_char('A'));
        assert!(!is_forbidden_control_char('~'));
    }

    #[test]
    fn test_forbidden_c0_controls() {
        assert!(is_forbidden_control_char('\u{0000}')); // NULL
        assert!(is_forbidden_control_char('\u{0008}')); // Backspace
        assert!(is_forbidden_control_char('\u{000B}')); // Vertical tab
        assert!(is_forbidden_control_char('\u{000C}')); // Form feed
    }

    #[test]
    fn test_forbidden_del() {
        assert!(is_forbidden_control_char('\u{007F}')); // DEL
    }

    #[test]
    fn test_forbidden_c1_controls() {
        assert!(is_forbidden_control_char('\u{0080}'));
        assert!(is_forbidden_control_char('\u{0090}'));
        assert!(is_forbidden_control_char('\u{009F}'));
    }
}
