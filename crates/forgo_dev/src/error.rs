//! Errors: a usage mistake, a failure, or a check that didn't pass.

use std::fmt;

/// Why a command didn't run or didn't finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The command line is wrong.
    Usage(String),
    /// The command can't be carried out; nothing was changed unless the
    /// message says otherwise.
    Failed(String),
    /// The command ran, and what it checked doesn't hold (an invalid
    /// version, a version that doesn't match): its report, printed as
    /// output, with exit code 1.
    Unmet(String),
}

impl Error {
    /// A usage mistake: the command line itself is wrong.
    pub fn is_usage(&self) -> bool {
        matches!(self, Error::Usage(_))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Usage(m) | Error::Failed(m) | Error::Unmet(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

/// A result with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Shorthand for [`Error::Failed`].
pub(crate) fn failed<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Failed(msg.into()))
}
