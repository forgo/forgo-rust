//! [`Num`]: a version number of any size.

use crate::error::{Error, ErrorKind, Part};
use alloc::boxed::Box;
use alloc::string::String;
use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

/// A non-negative integer of any size: the major, minor or patch number of
/// a [`Version`](crate::Version).
///
/// SemVer sets no upper limit on these numbers, so neither does `Num`.
/// Values that fit in a `u64` are stored inline (the common case: no
/// allocation); larger ones as their decimal digits. Every value has
/// exactly one representation, so `==` and hashing are by value.
///
/// ```
/// use forgo_lib_semver::Num;
///
/// let n = Num::new(u64::MAX);
/// assert_eq!(n.to_u64(), Some(u64::MAX));
/// let next = n.succ();
/// assert_eq!(next.to_string(), "18446744073709551616");
/// assert_eq!(next.to_u64(), None);
/// assert!(next > n);
/// assert_eq!(Num::new(7), 7);
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Num(Repr);

#[derive(Clone, PartialEq, Eq, Hash)]
enum Repr {
    Small(u64),
    /// The decimal digits of a value above `u64::MAX`, without leading
    /// zeros. (`Small` fits in the bytes beside the pointer's niche, so
    /// `Num` is 16 bytes.)
    Big(Box<str>),
}

/// `u64::MAX` in decimal.
const U64_MAX_DIGITS: &str = "18446744073709551615";

impl Num {
    /// Zero.
    pub const ZERO: Num = Num(Repr::Small(0));

    /// A number from a `u64`.
    pub const fn new(n: u64) -> Num {
        Num(Repr::Small(n))
    }

    /// The value, if it fits in a `u64`.
    #[inline]
    pub fn to_u64(&self) -> Option<u64> {
        match self.0 {
            Repr::Small(n) => Some(n),
            Repr::Big(_) => None,
        }
    }

    /// Whether this is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        matches!(self.0, Repr::Small(0))
    }

    /// This number plus one.
    pub fn succ(&self) -> Num {
        match &self.0 {
            Repr::Small(n) => match n.checked_add(1) {
                Some(m) => Num::new(m),
                None => Num(Repr::Big("18446744073709551616".into())),
            },
            Repr::Big(digits) => {
                let mut d: alloc::vec::Vec<u8> = digits.as_bytes().to_vec();
                let mut i = d.len();
                loop {
                    if i == 0 {
                        d.insert(0, b'1');
                        break;
                    }
                    i -= 1;
                    if d[i] == b'9' {
                        d[i] = b'0';
                    } else {
                        d[i] += 1;
                        break;
                    }
                }
                let s = String::from_utf8(d).expect("ASCII digits");
                Num(Repr::Big(s.into_boxed_str()))
            }
        }
    }

    /// The number written with `digits` (ASCII digits, at least one,
    /// without leading zeros unless the number is zero).
    pub(crate) fn from_canonical_digits(digits: &str) -> Num {
        debug_assert!(!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
        debug_assert!(digits == "0" || !digits.starts_with('0'));
        let fits = digits.len() < U64_MAX_DIGITS.len()
            || (digits.len() == U64_MAX_DIGITS.len() && digits <= U64_MAX_DIGITS);
        if fits {
            let mut n = 0u64;
            for b in digits.bytes() {
                n = n * 10 + u64::from(b - b'0');
            }
            Num::new(n)
        } else {
            Num(Repr::Big(digits.into()))
        }
    }

    /// The number written with `digits` (ASCII digits, at least one),
    /// ignoring leading zeros.
    pub(crate) fn from_digits_lenient(digits: &str) -> Num {
        let trimmed = digits.trim_start_matches('0');
        Num::from_canonical_digits(if trimmed.is_empty() { "0" } else { trimmed })
    }
}

impl Default for Num {
    fn default() -> Num {
        Num::ZERO
    }
}

impl Ord for Num {
    #[inline]
    fn cmp(&self, other: &Num) -> Ordering {
        match (&self.0, &other.0) {
            (Repr::Small(a), Repr::Small(b)) => a.cmp(b),
            (Repr::Small(_), Repr::Big(_)) => Ordering::Less,
            (Repr::Big(_), Repr::Small(_)) => Ordering::Greater,
            (Repr::Big(a), Repr::Big(b)) => a.len().cmp(&b.len()).then_with(|| a.cmp(b)),
        }
    }
}

impl PartialOrd for Num {
    #[inline]
    fn partial_cmp(&self, other: &Num) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<u64> for Num {
    #[inline]
    fn eq(&self, other: &u64) -> bool {
        matches!(self.0, Repr::Small(n) if n == *other)
    }
}

impl PartialOrd<u64> for Num {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        Some(match self.0 {
            Repr::Small(n) => n.cmp(other),
            Repr::Big(_) => Ordering::Greater,
        })
    }
}

macro_rules! from_unsigned {
    ($($t:ty),*) => {$(
        impl From<$t> for Num {
            fn from(n: $t) -> Num {
                Num::new(n as u64)
            }
        }
    )*};
}
from_unsigned!(u8, u16, u32, u64, usize);

impl FromStr for Num {
    type Err = Error;

    /// Decimal digits without a sign or leading zeros (`0`, `42`, or a
    /// number of any length).
    fn from_str(s: &str) -> Result<Num, Error> {
        let b = s.as_bytes();
        match b.first() {
            None => Err(Error::new(ErrorKind::UnexpectedEnd(Part::Major), 0)),
            Some(b'0') if b.len() > 1 => Err(Error::new(ErrorKind::LeadingZero(Part::Major), 0)),
            _ => match b.iter().position(|c| !c.is_ascii_digit()) {
                Some(i) => {
                    let c = s[i..].chars().next().unwrap_or('\0');
                    Err(Error::new(ErrorKind::UnexpectedChar(Part::Major, c), i))
                }
                None => Ok(Num::from_canonical_digits(s)),
            },
        }
    }
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Repr::Small(n) => fmt::Display::fmt(n, f),
            Repr::Big(digits) => f.pad_integral(true, "", digits),
        }
    }
}

impl fmt::Debug for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
