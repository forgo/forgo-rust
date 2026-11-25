//! A small regular expression engine with JavaScript's semantics, for the
//! patterns node-semver uses.
//!
//! npm's range syntax is defined by node-semver's regular expressions and
//! the order it applies them in, quirks included. Running the same
//! patterns with the same semantics (leftmost match, alternatives in
//! order, greedy quantifiers, JavaScript's `\s`) reproduces its results
//! exactly; `tests/npm.rs` checks that against node-semver itself.
//!
//! Patterns are compiled by `const fn`s into static instruction tables,
//! so nothing is compiled at run time. Matching backtracks with an
//! explicit stack (no recursion), and a step budget bounds the work.
//!
//! Supported syntax: literals, `\d`, `\s`, escaped punctuation, classes
//! (`[a-z0-9-]`, `[^\d]`), groups `(…)` and `(?:…)`, `|`, `?`, `*`, `+`,
//! `{m,n}` (on single characters), `^` and `$`. Anything else fails to
//! compile (at compile time).

use alloc::vec::Vec;

/// A set of characters.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Class {
    lo: u64,
    hi: u64,
    /// Includes JavaScript's non-ASCII whitespace (`\s`).
    space: bool,
    negated: bool,
}

/// The characters JavaScript's `\s` (and `String.prototype.trim`)
/// matches outside ASCII.
const JS_SPACE_NON_ASCII: [char; 19] = [
    '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}',
    '\u{205f}', '\u{3000}', '\u{feff}',
];

/// Whether JavaScript's `\s` matches `c`.
pub(crate) fn is_js_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ') || JS_SPACE_NON_ASCII.contains(&c)
}

impl Class {
    const EMPTY: Class = Class {
        lo: 0,
        hi: 0,
        space: false,
        negated: false,
    };

    const fn with_byte(mut self, b: u8) -> Class {
        if b < 64 {
            self.lo |= 1 << b;
        } else {
            self.hi |= 1 << (b - 64);
        }
        self
    }

    const fn with_range(mut self, a: u8, b: u8) -> Class {
        let mut c = a;
        while c <= b {
            self = self.with_byte(c);
            c += 1;
        }
        self
    }

    const fn negate(mut self) -> Class {
        self.negated = !self.negated;
        self
    }

    const fn with_space(self) -> Class {
        let mut c = self
            .with_byte(b'\t')
            .with_byte(b'\n')
            .with_byte(0x0b)
            .with_byte(0x0c)
            .with_byte(b'\r')
            .with_byte(b' ');
        c.space = true;
        c
    }

    #[inline]
    fn matches(&self, c: char) -> bool {
        let n = c as u32;
        let m = if n < 64 {
            self.lo & (1 << n) != 0
        } else if n < 128 {
            self.hi & (1 << (n - 64)) != 0
        } else {
            self.space && JS_SPACE_NON_ASCII.contains(&c)
        };
        m != self.negated
    }

    #[inline]
    fn matches_byte(&self, b: u8) -> bool {
        let m = if b < 64 {
            self.lo & (1 << b) != 0
        } else {
            self.hi & (1 << (b - 64)) != 0
        };
        m != self.negated
    }
}

/// Unbounded repetition.
const INF: u16 = u16::MAX;

/// What one character must be: an ASCII byte, or in one of [`CLASSES`].
#[derive(Clone, Copy, Debug)]
pub(crate) enum Set {
    Byte(u8),
    Class(u8),
}

/// The character classes node-semver's patterns use (each pattern's
/// classes must be in this table; compiling fails otherwise).
const CLASSES: [Class; 7] = [
    Class::EMPTY.with_range(b'0', b'9'),
    Class::EMPTY.with_space(),
    Class::EMPTY.with_range(b'1', b'9'),
    Class::EMPTY
        .with_range(b'a', b'z')
        .with_range(b'A', b'Z')
        .with_byte(b'-'),
    Class::EMPTY
        .with_range(b'a', b'z')
        .with_range(b'A', b'Z')
        .with_range(b'0', b'9')
        .with_byte(b'-'),
    Class::EMPTY.with_byte(b'v').with_byte(b'=').with_space(),
    Class::EMPTY.with_range(b'0', b'9').negate(),
];

impl Set {
    const fn of(class: Class) -> Set {
        // A single ASCII byte (not negated, no `\s`).
        if !class.negated && !class.space && (class.lo.count_ones() + class.hi.count_ones()) == 1 {
            let b = if class.lo != 0 {
                class.lo.trailing_zeros()
            } else {
                64 + class.hi.trailing_zeros()
            };
            return Set::Byte(b as u8);
        }
        let mut i = 0;
        while i < CLASSES.len() {
            let c = CLASSES[i];
            if c.lo == class.lo
                && c.hi == class.hi
                && c.space == class.space
                && c.negated == class.negated
            {
                return Set::Class(i as u8);
            }
            i += 1;
        }
        panic!("character class not in CLASSES")
    }

    #[inline]
    fn matches_byte(self, b: u8) -> bool {
        match self {
            Set::Byte(x) => x == b,
            Set::Class(i) => CLASSES[i as usize].matches_byte(b),
        }
    }

    #[inline]
    fn matches(self, c: char) -> bool {
        match self {
            Set::Byte(_) => false,
            Set::Class(i) => CLASSES[i as usize].matches(c),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Inst {
    /// One character in the set.
    One(Set),
    /// `min` to `max` characters in the set, as many as possible first.
    Repeat(Set, u16, u16),
    /// Try the first target; on failure, the second.
    Split(u16, u16),
    Jmp(u16),
    /// Record the position in a capture slot.
    Save(u8),
    Start,
    End,
    Match,
}

/// A compiled pattern.
pub(crate) struct Regex {
    insts: &'static [Inst],
    /// Capture groups, counting the whole match as group 0.
    #[cfg_attr(not(test), allow(dead_code))]
    groups: usize,
    /// The pattern starts with `^`: it can only match at position 0.
    anchored: bool,
}

/// Compiles a pattern into at most `N` instructions.
pub(crate) struct Prog<const N: usize> {
    pub(crate) insts: [Inst; N],
    pub(crate) len: usize,
    pub(crate) groups: usize,
    pub(crate) anchored: bool,
}

struct Compiler<'s, const N: usize> {
    src: &'s [u8],
    insts: [Inst; N],
    len: usize,
    groups: usize,
}

/// The kind and extent of an atom.
#[derive(Clone, Copy)]
enum Atom {
    Single(Class),
    Start,
    End,
    Group {
        from: usize,
        to: usize,
        capture: bool,
    },
}

impl<'s, const N: usize> Compiler<'s, N> {
    const fn emit(&mut self, inst: Inst) -> usize {
        assert!(self.len < N, "regex program too long");
        self.insts[self.len] = inst;
        self.len += 1;
        self.len - 1
    }

    /// The index after the group, class or escape starting at `i`.
    const fn skip_class(&self, mut i: usize) -> usize {
        // `i` is at `[`.
        i += 1;
        if i < self.src.len() && self.src[i] == b'^' {
            i += 1;
        }
        while self.src[i] != b']' {
            if self.src[i] == b'\\' {
                i += 1;
            }
            i += 1;
        }
        i + 1
    }

    const fn close_paren(&self, mut i: usize) -> usize {
        // `i` is at `(`; returns the index of the matching `)`.
        let mut depth = 0;
        loop {
            match self.src[i] {
                b'\\' => i += 2,
                b'[' => i = self.skip_class(i),
                b'(' => {
                    depth += 1;
                    i += 1;
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return i;
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }

    const fn escape(c: u8) -> Class {
        match c {
            b'd' => Class::EMPTY.with_range(b'0', b'9'),
            b's' => Class::EMPTY.with_space(),
            b'.' | b'+' | b'*' | b'^' | b'$' | b'\\' | b'-' | b'?' | b'(' | b')' | b'[' | b']'
            | b'{' | b'}' | b'|' | b'/' => Class::EMPTY.with_byte(c),
            _ => panic!("unsupported escape"),
        }
    }

    const fn class(&self, mut i: usize) -> Class {
        // `i` is at `[`.
        i += 1;
        let mut c = Class::EMPTY;
        let mut negated = false;
        if self.src[i] == b'^' {
            negated = true;
            i += 1;
        }
        while self.src[i] != b']' {
            let b = self.src[i];
            if b == b'\\' {
                let e = Self::escape(self.src[i + 1]);
                c.lo |= e.lo;
                c.hi |= e.hi;
                c.space |= e.space;
                i += 2;
            } else if i + 2 < self.src.len() && self.src[i + 1] == b'-' && self.src[i + 2] != b']' {
                c = c.with_range(b, self.src[i + 2]);
                i += 3;
            } else {
                c = c.with_byte(b);
                i += 1;
            }
        }
        c.negated = negated;
        c
    }

    /// The atom at `i`, and the index after it.
    const fn atom(&self, i: usize) -> (Atom, usize) {
        let b = self.src[i];
        match b {
            b'(' => {
                let close = self.close_paren(i);
                let non_capture = self.src[i + 1] == b'?';
                if non_capture {
                    assert!(self.src[i + 2] == b':', "only (?: groups are supported");
                }
                let from = if non_capture { i + 3 } else { i + 1 };
                (
                    Atom::Group {
                        from,
                        to: close,
                        capture: !non_capture,
                    },
                    close + 1,
                )
            }
            b'[' => (Atom::Single(self.class(i)), self.skip_class(i)),
            b'\\' => (Atom::Single(Self::escape(self.src[i + 1])), i + 2),
            b'^' => (Atom::Start, i + 1),
            b'$' => (Atom::End, i + 1),
            b'.' | b'*' | b'+' | b'?' | b'{' | b')' | b']' | b'}' => panic!("unsupported syntax"),
            _ => (Atom::Single(Class::EMPTY.with_byte(b)), i + 1),
        }
    }

    const fn number(&self, mut i: usize) -> (u16, usize) {
        let mut n: u16 = 0;
        while self.src[i].is_ascii_digit() {
            n = n * 10 + (self.src[i] - b'0') as u16;
            i += 1;
        }
        (n, i)
    }

    /// The quantifier at `i` (`(1, 1)` if none), and the index after it.
    const fn quantifier(&self, i: usize, end: usize) -> (u16, u16, usize) {
        if i >= end {
            return (1, 1, i);
        }
        let q = match self.src[i] {
            b'?' => (0, 1, i + 1),
            b'*' => (0, INF, i + 1),
            b'+' => (1, INF, i + 1),
            b'{' => {
                let (min, j) = self.number(i + 1);
                if self.src[j] == b'}' {
                    (min, min, j + 1)
                } else {
                    assert!(self.src[j] == b',', "bad quantifier");
                    let (max, k) = if self.src[j + 1] == b'}' {
                        (INF, j + 1)
                    } else {
                        self.number(j + 1)
                    };
                    assert!(self.src[k] == b'}', "bad quantifier");
                    (min, max, k + 1)
                }
            }
            _ => return (1, 1, i),
        };
        assert!(
            q.2 >= end || self.src[q.2] != b'?',
            "lazy quantifiers are not supported"
        );
        q
    }

    const fn alternation(&mut self, from: usize, to: usize) {
        // Top-level `|` positions in from..to.
        let mut bars = [0usize; 16];
        let mut n = 0;
        let mut i = from;
        while i < to {
            match self.src[i] {
                b'\\' => i += 2,
                b'[' => i = self.skip_class(i),
                b'(' => i = self.close_paren(i) + 1,
                b'|' => {
                    bars[n] = i;
                    n += 1;
                    i += 1;
                }
                _ => i += 1,
            }
        }
        if n == 0 {
            self.sequence(from, to);
            return;
        }
        let mut jumps = [0usize; 16];
        let mut start = from;
        let mut k = 0;
        while k <= n {
            let end = if k < n { bars[k] } else { to };
            if k < n {
                let split = self.emit(Inst::Split(0, 0));
                self.sequence(start, end);
                jumps[k] = self.emit(Inst::Jmp(0));
                self.insts[split] = Inst::Split((split + 1) as u16, self.len as u16);
            } else {
                self.sequence(start, end);
            }
            start = end + 1;
            k += 1;
        }
        let mut k = 0;
        while k < n {
            self.insts[jumps[k]] = Inst::Jmp(self.len as u16);
            k += 1;
        }
    }

    const fn sequence(&mut self, from: usize, to: usize) {
        let mut i = from;
        while i < to {
            let (atom, after) = self.atom(i);
            let (min, max, next) = self.quantifier(after, to);
            match atom {
                Atom::Single(class) => {
                    let set = Set::of(class);
                    if min == 1 && max == 1 {
                        self.emit(Inst::One(set));
                    } else {
                        self.emit(Inst::Repeat(set, min, max));
                    }
                }
                Atom::Start => {
                    assert!(min == 1 && max == 1, "quantified anchor");
                    self.emit(Inst::Start);
                }
                Atom::End => {
                    assert!(min == 1 && max == 1, "quantified anchor");
                    self.emit(Inst::End);
                }
                Atom::Group { from, to, capture } => {
                    if min == 1 && max == 1 {
                        self.group(from, to, capture);
                    } else if min == 0 && max == 1 {
                        let split = self.emit(Inst::Split(0, 0));
                        self.group(from, to, capture);
                        self.insts[split] = Inst::Split((split + 1) as u16, self.len as u16);
                    } else if min == 0 && max == INF {
                        let split = self.emit(Inst::Split(0, 0));
                        self.group(from, to, capture);
                        self.emit(Inst::Jmp(split as u16));
                        self.insts[split] = Inst::Split((split + 1) as u16, self.len as u16);
                    } else if min == 1 && max == INF {
                        let top = self.len;
                        self.group(from, to, capture);
                        self.emit(Inst::Split(top as u16, (self.len + 1) as u16));
                    } else {
                        panic!("counted repetition of a group");
                    }
                }
            }
            i = next;
        }
    }

    const fn group(&mut self, from: usize, to: usize, capture: bool) {
        if capture {
            self.groups += 1;
            let g = self.groups;
            self.emit(Inst::Save((2 * g) as u8));
            self.alternation(from, to);
            self.emit(Inst::Save((2 * g + 1) as u8));
        } else {
            self.alternation(from, to);
        }
    }
}

/// Compile `src` into at most `N` instructions.
pub(crate) const fn compile<const N: usize>(src: &str) -> Prog<N> {
    let mut c = Compiler::<N> {
        src: src.as_bytes(),
        insts: [Inst::Match; N],
        len: 0,
        groups: 0,
    };
    c.alternation(0, src.len());
    c.emit(Inst::Match);
    assert!(c.groups < MAX_GROUPS, "too many capture groups");
    Prog {
        insts: c.insts,
        len: c.len,
        groups: c.groups + 1,
        anchored: !src.is_empty() && src.as_bytes()[0] == b'^',
    }
}

/// The number of instructions `src` compiles to.
pub(crate) const fn measure(src: &str) -> usize {
    compile::<1024>(src).len
}

/// Declare a pattern compiled at build time.
macro_rules! regex {
    ($name:ident = $src:expr) => {
        pub(crate) static $name: $crate::npm::regex::Regex = {
            const SRC: &str = $src;
            const LEN: usize = $crate::npm::regex::measure(SRC);
            const PROG: $crate::npm::regex::Prog<LEN> = $crate::npm::regex::compile::<LEN>(SRC);
            static INSTS: [$crate::npm::regex::Inst; LEN] = PROG.insts;
            $crate::npm::regex::Regex::new(&INSTS, PROG.groups, PROG.anchored)
        };
    };
}
pub(crate) use regex;

/// Capture slots: start and end of each group (`usize::MAX` when the
/// group didn't take part in the match).
pub(crate) const MAX_GROUPS: usize = 16;

/// A match: the byte range of each group (group 0 is the whole match).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Captures {
    slots: [usize; 2 * MAX_GROUPS],
}

impl Captures {
    const UNSET: usize = usize::MAX;

    /// Group `i`'s text, if it took part in the match.
    pub(crate) fn get<'t>(&self, text: &'t str, i: usize) -> Option<&'t str> {
        let (s, e) = (self.slots[2 * i], self.slots[2 * i + 1]);
        (s != Self::UNSET && e != Self::UNSET).then(|| &text[s..e])
    }

    pub(crate) fn start(&self) -> usize {
        self.slots[0]
    }

    pub(crate) fn end(&self) -> usize {
        self.slots[1]
    }
}

enum Backtrack {
    /// Resume at `pc` with the input at `pos`.
    Resume { pc: usize, pos: usize },
    /// Restore a capture slot.
    Restore { slot: usize, value: usize },
    /// A greedy repeat that can give back characters: resume at `pc` with
    /// one fewer character than `pos` (not going below `min_pos`).
    Repeat {
        pc: usize,
        min_pos: usize,
        pos: usize,
    },
}

/// The most backtracking steps one search may take: node-semver's
/// patterns need far fewer on any input it accepts.
const STEP_BUDGET: usize = 2_000_000;

/// The search ran out of its step budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TooComplex;

impl Regex {
    pub(crate) const fn new(insts: &'static [Inst], groups: usize, anchored: bool) -> Regex {
        Regex {
            insts,
            groups,
            anchored,
        }
    }

    /// The first match starting at or after byte `from` (JavaScript's
    /// `exec` with `lastIndex = from`).
    pub(crate) fn find_at(&self, text: &str, from: usize) -> Result<Option<Captures>, TooComplex> {
        let mut stack = Vec::new();
        let mut budget = STEP_BUDGET;
        let mut start = from;
        loop {
            if let Some(c) = self.match_at(text, start, &mut stack, &mut budget)? {
                return Ok(Some(c));
            }
            if self.anchored || start >= text.len() {
                return Ok(None);
            }
            start += text[start..].chars().next().map_or(1, char::len_utf8);
        }
    }

    /// The first match (`text.match(re)` for a non-global pattern).
    pub(crate) fn find(&self, text: &str) -> Result<Option<Captures>, TooComplex> {
        self.find_at(text, 0)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn groups(&self) -> usize {
        self.groups
    }

    fn match_at(
        &self,
        text: &str,
        start: usize,
        stack: &mut Vec<Backtrack>,
        budget: &mut usize,
    ) -> Result<Option<Captures>, TooComplex> {
        let insts = self.insts;
        let bytes = text.as_bytes();
        let mut caps = Captures {
            slots: [Captures::UNSET; 2 * MAX_GROUPS],
        };
        caps.slots[0] = start;
        stack.clear();
        let mut pc = 0;
        let mut pos = start;
        loop {
            if *budget == 0 {
                return Err(TooComplex);
            }
            *budget -= 1;
            let ok = match insts[pc] {
                Inst::One(class) => match bytes.get(pos) {
                    Some(&b) if b < 0x80 => {
                        if class.matches_byte(b) {
                            pos += 1;
                            pc += 1;
                            true
                        } else {
                            false
                        }
                    }
                    Some(_) => {
                        let c = text[pos..].chars().next().expect("char boundary");
                        if class.matches(c) {
                            pos += c.len_utf8();
                            pc += 1;
                            true
                        } else {
                            false
                        }
                    }
                    None => false,
                },
                Inst::Repeat(class, min, max) => {
                    let begin = pos;
                    let mut count = 0u16;
                    let mut min_pos = if min == 0 { pos } else { usize::MAX };
                    while count < max {
                        let Some(&b) = bytes.get(pos) else { break };
                        let len = if b < 0x80 {
                            if !class.matches_byte(b) {
                                break;
                            }
                            1
                        } else {
                            let c = text[pos..].chars().next().expect("char boundary");
                            if !class.matches(c) {
                                break;
                            }
                            c.len_utf8()
                        };
                        pos += len;
                        count += 1;
                        if count == min {
                            min_pos = pos;
                        }
                    }
                    if count < min {
                        false
                    } else {
                        if pos > min_pos {
                            stack.push(Backtrack::Repeat {
                                pc: pc + 1,
                                min_pos,
                                pos,
                            });
                        }
                        let _ = begin;
                        pc += 1;
                        true
                    }
                }
                Inst::Split(a, b) => {
                    stack.push(Backtrack::Resume {
                        pc: b as usize,
                        pos,
                    });
                    pc = a as usize;
                    true
                }
                Inst::Jmp(a) => {
                    pc = a as usize;
                    true
                }
                Inst::Save(slot) => {
                    let slot = slot as usize;
                    stack.push(Backtrack::Restore {
                        slot,
                        value: caps.slots[slot],
                    });
                    caps.slots[slot] = pos;
                    pc += 1;
                    true
                }
                Inst::Start => {
                    if pos == 0 {
                        pc += 1;
                        true
                    } else {
                        false
                    }
                }
                Inst::End => {
                    if pos == bytes.len() {
                        pc += 1;
                        true
                    } else {
                        false
                    }
                }
                Inst::Match => {
                    caps.slots[1] = pos;
                    return Ok(Some(caps));
                }
            };
            if ok {
                continue;
            }
            // Backtrack.
            loop {
                match stack.pop() {
                    None => return Ok(None),
                    Some(Backtrack::Restore { slot, value }) => caps.slots[slot] = value,
                    Some(Backtrack::Resume { pc: p, pos: q }) => {
                        pc = p;
                        pos = q;
                        break;
                    }
                    Some(Backtrack::Repeat {
                        pc: p,
                        min_pos,
                        pos: q,
                    }) => {
                        // Give back one character.
                        let mut back = q - 1;
                        while !text.is_char_boundary(back) {
                            back -= 1;
                        }
                        if back > min_pos {
                            stack.push(Backtrack::Repeat {
                                pc: p,
                                min_pos,
                                pos: back,
                            });
                        }
                        pc = p;
                        pos = back;
                        break;
                    }
                }
            }
        }
    }
}

/// `text.replace(re, f)`: the first match (or, `global`, every match)
/// replaced by `f(captures)`.
pub(crate) fn replace(
    re: &Regex,
    text: &str,
    global: bool,
    mut f: impl FnMut(&Captures) -> alloc::string::String,
) -> Result<alloc::string::String, TooComplex> {
    let mut out = alloc::string::String::with_capacity(text.len());
    let mut last = 0;
    let mut from = 0;
    while from <= text.len() {
        let Some(c) = re.find_at(text, from)? else {
            break;
        };
        out.push_str(&text[last..c.start()]);
        out.push_str(&f(&c));
        last = c.end();
        if !global {
            break;
        }
        from = if c.end() == c.start() {
            // An empty match: move on one character.
            c.end() + text[c.end()..].chars().next().map_or(1, char::len_utf8)
        } else {
            c.end()
        };
    }
    out.push_str(&text[last.min(text.len())..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    regex!(DIGITS = r"(\d{1,3})(?:\.(\d+))?");
    regex!(ANCHORED = r"^v?(0|[1-9]\d*)$");
    regex!(ALT = r"^(?:a|ab)(c)?$");
    regex!(SPACE = r"(\s{0,1})(?:~>?)\s{1,1}");
    regex!(NEG = r"(^|[^\d])(\d{1,16})(?:$|[^\d])");

    fn groups(re: &Regex, text: &str) -> Option<alloc::vec::Vec<Option<alloc::string::String>>> {
        let c = re.find(text).unwrap()?;
        Some(
            (0..re.groups())
                .map(|i| c.get(text, i).map(alloc::string::String::from))
                .collect(),
        )
    }

    fn s(x: &str) -> Option<alloc::string::String> {
        Some(x.into())
    }

    #[test]
    fn matches_like_javascript() {
        assert_eq!(
            groups(&DIGITS, "ab1234.5"),
            Some(alloc::vec![s("123"), s("123"), None])
        );
        assert_eq!(
            groups(&DIGITS, "12.34"),
            Some(alloc::vec![s("12.34"), s("12"), s("34")])
        );
        assert_eq!(
            groups(&ANCHORED, "v10"),
            Some(alloc::vec![s("v10"), s("10")])
        );
        assert_eq!(groups(&ANCHORED, "010"), None);
        // Alternatives in order, backtracking into the second.
        assert_eq!(groups(&ALT, "abc"), Some(alloc::vec![s("abc"), s("c")]));
        assert_eq!(groups(&ALT, "ab"), Some(alloc::vec![s("ab"), None]));
        assert_eq!(groups(&SPACE, "a ~ 1"), Some(alloc::vec![s(" ~ "), s(" ")]));
        assert_eq!(
            groups(&SPACE, "~\u{a0}1"),
            Some(alloc::vec![s("~\u{a0}"), s("")])
        );
        assert_eq!(
            groups(&NEG, "x1.2"),
            Some(alloc::vec![s("x1."), s("x"), s("1")])
        );
    }

    #[test]
    fn replaces_like_javascript() {
        let r = replace(&SPACE, "~ 1 || ~> 2", true, |c| {
            alloc::format!("{}~", c.get("~ 1 || ~> 2", 1).unwrap_or(""))
        })
        .unwrap();
        assert_eq!(r, "~1 || ~2");
    }
}
