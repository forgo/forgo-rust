//! `trimComparatorWhitespace` (node-semver's
//! `internal/trim-comparator-whitespace.js`): joins an operator to its
//! version (`> 1.2.3` to `>1.2.3`) exactly where the regular expression it
//! replaced would have.

use super::regex::is_js_space;
use alloc::string::String;
use alloc::vec::Vec;

const MAX_LENGTH: usize = 256;
const MAX_SAFE_BUILD_LENGTH: usize = MAX_LENGTH - 6;

/// The last two runs of digits found, so they aren't read again.
struct DigitRuns {
    start: isize,
    end: isize,
    previous_start: isize,
    previous_end: isize,
}

struct Text {
    chars: Vec<char>,
}

impl Text {
    fn at(&self, i: usize) -> Option<char> {
        self.chars.get(i).copied()
    }

    fn is_space(&self, i: usize) -> bool {
        self.at(i).is_some_and(is_js_space)
    }

    fn is_prefix(&self, i: usize) -> bool {
        matches!(self.at(i), Some('v' | '=')) || self.is_space(i)
    }

    fn is_digit(&self, i: usize) -> bool {
        self.at(i).is_some_and(|c| c.is_ascii_digit())
    }

    fn is_letter(&self, i: usize) -> bool {
        self.at(i).is_some_and(|c| c.is_ascii_alphabetic())
    }

    fn is_ident(&self, i: usize) -> bool {
        self.is_digit(i) || self.is_letter(i) || self.at(i) == Some('-')
    }

    fn is_version_start(&self, i: usize) -> bool {
        self.is_digit(i) || matches!(self.at(i), Some('x' | 'X' | '*'))
    }

    fn digits_end(&self, start: usize, limit: usize, runs: &mut DigitRuns) -> usize {
        if !self.is_digit(start) {
            return start;
        }
        let s = start as isize;
        if s >= runs.start && s < runs.end {
            return (start + limit).min(runs.end as usize);
        }
        if s >= runs.previous_start && s < runs.previous_end {
            return (start + limit).min(runs.previous_end as usize);
        }
        let mut end = start + 1;
        while self.is_digit(end) {
            end += 1;
        }
        runs.previous_start = runs.start;
        runs.previous_end = runs.end;
        runs.start = s;
        runs.end = end as isize;
        (start + limit).min(end)
    }

    fn version_number_end(&self, start: usize, loose: bool, runs: &mut DigitRuns) -> usize {
        if !loose && self.at(start) == Some('0') {
            return start + 1;
        }
        let limit = if loose { MAX_LENGTH } else { MAX_LENGTH + 1 };
        self.digits_end(start, limit, runs)
    }

    fn prerelease_identifier_end(&self, start: usize, loose: bool, runs: &mut DigitRuns) -> usize {
        let digits_end = self.digits_end(start, MAX_LENGTH, runs);
        if self.is_letter(digits_end) || self.at(digits_end) == Some('-') {
            let mut end = digits_end + 1;
            let limit = end + MAX_SAFE_BUILD_LENGTH;
            while end < limit && self.is_ident(end) {
                end += 1;
            }
            return end;
        }
        self.version_number_end(start, loose, runs)
    }

    fn prerelease_end(&self, start: usize, loose: bool, runs: &mut DigitRuns) -> usize {
        let hyphen = self.at(start) == Some('-');
        if !loose && !hyphen {
            return start;
        }
        let id_start = start + usize::from(hyphen);
        let mut end = self.prerelease_identifier_end(id_start, loose, runs);
        if end == id_start {
            if !loose || !hyphen {
                return start;
            }
            end = self.prerelease_identifier_end(start, loose, runs);
        }
        while self.at(end) == Some('.') {
            let next = self.prerelease_identifier_end(end + 1, loose, runs);
            if next == end + 1 {
                break;
            }
            end = next;
        }
        end
    }

    fn build_identifier_end(&self, start: usize) -> usize {
        let mut end = start;
        while end - start < MAX_SAFE_BUILD_LENGTH && self.is_ident(end) {
            end += 1;
        }
        end
    }

    fn build_end(&self, start: usize) -> usize {
        if self.at(start) != Some('+') {
            return start;
        }
        let mut end = self.build_identifier_end(start + 1);
        if end == start + 1 {
            return start;
        }
        while self.at(end) == Some('.') {
            let next = self.build_identifier_end(end + 1);
            if next == end + 1 {
                break;
            }
            end = next;
        }
        end
    }

    fn loose_full_version_end(&self, start: usize, runs: &mut DigitRuns) -> usize {
        let mut end = start;
        for component in 0..3 {
            let next = self.version_number_end(end, true, runs);
            if next == end {
                return start;
            }
            end = next;
            if component < 2 {
                if self.at(end) != Some('.') {
                    return start;
                }
                end += 1;
            }
        }
        let pre_end = self.prerelease_end(end, true, runs);
        self.build_end(pre_end)
    }

    fn partial_component_end(&self, start: usize, runs: &mut DigitRuns) -> usize {
        if self.is_digit(start) {
            return self.version_number_end(start, false, runs);
        }
        if self.is_version_start(start) {
            start + 1
        } else {
            start
        }
    }

    fn partial_version_end(&self, start: usize, runs: &mut DigitRuns) -> usize {
        let mut end = self.partial_component_end(start, runs);
        for _ in 0..2 {
            if self.at(end) != Some('.') {
                return end;
            }
            let next = self.partial_component_end(end + 1, runs);
            if next == end + 1 {
                return end;
            }
            end = next;
        }
        let pre_end = self.prerelease_end(end, false, runs);
        self.build_end(pre_end)
    }

    fn version_end(&self, start: usize, runs: &mut DigitRuns) -> usize {
        let full = self.loose_full_version_end(start, runs);
        if full != start {
            full
        } else {
            self.partial_version_end(start, runs)
        }
    }

    fn removable_space(&self, match_start: usize) -> Option<usize> {
        let mut position = match_start;
        if self.is_space(position) {
            position += 1;
        }
        if matches!(self.at(position), Some('<' | '>')) {
            position += 1;
        }
        if self.at(position) == Some('=') {
            position += 1;
        }
        self.is_space(position).then_some(position)
    }
}

/// `trimComparatorWhitespace(range)`.
pub(crate) fn trim_comparator_whitespace(range: &str) -> String {
    let text = Text {
        chars: range.chars().collect(),
    };
    let len = text.chars.len();
    let mut result = String::with_capacity(range.len());
    let mut copy_from = 0;
    let mut cursor = 0;
    let mut runs = DigitRuns {
        start: -1,
        end: -1,
        previous_start: -1,
        previous_end: -1,
    };
    while cursor < len {
        let match_start = cursor;
        if matches!(text.at(cursor), Some('<' | '>')) {
            cursor += 1;
        } else if !text.is_prefix(cursor) && !text.is_version_start(cursor) {
            cursor += 1;
            continue;
        }
        while cursor < len && text.is_prefix(cursor) {
            cursor += 1;
        }
        if !text.is_version_start(cursor) {
            continue;
        }
        if let Some(space) = text.removable_space(match_start) {
            result.extend(&text.chars[copy_from..space]);
            copy_from = space + 1;
        }
        cursor = text.version_end(cursor, &mut runs);
    }
    result.extend(&text.chars[copy_from.min(len)..]);
    result
}
