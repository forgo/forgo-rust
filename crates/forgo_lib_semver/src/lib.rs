//! Semantic Versioning 2.0.0 for Rust.
//!
//! - [`Version`]: parse exactly as the specification defines (numbers of
//!   any size), compare by precedence, bump, diff.
//! - [`Req`]: version requirements in Cargo's syntax or npm's, with the
//!   matching rules of each, and exact set operations between them.
//! - [`npm`]: node-semver's functions, with node-semver's results.
//!
//! ```
//! use forgo_lib_semver::{Req, Version};
//!
//! let req = Req::cargo("^1.2")?;
//! assert!(req.matches(&Version::parse("1.9.0")?));
//! assert!(req.is_equivalent(&Req::npm(">=1.2.0 <2.0.0-0")?));
//! # Ok::<(), forgo_lib_semver::Error>(())
//! ```
//!
//! No dependencies, no `unsafe`, `no_std` (with `alloc`). Compliance and
//! comparisons with other crates are in the
//! [README](https://github.com/forgo/forgo-rust/tree/HEAD/crates/forgo_lib_semver#readme).

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

/// The README's examples, compiled and run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

mod cargo;
mod error;
mod ident;
pub mod npm;
mod num;
mod req;
mod set;
mod version;

pub use error::{Error, ErrorKind, Part};
pub use ident::{BuildMetadata, Identifier, Identifiers, Prerelease};
pub use num::Num;
pub use req::{Dialect, Req};
pub use version::{Bump, Version};

/// A kind of release: what [`Version::diff`] finds between two versions,
/// and the `release` argument of [`npm::inc`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Release {
    /// `major`: incompatible changes.
    Major,
    /// `premajor`: a pre-release of the next major version.
    Premajor,
    /// `minor`: compatible additions.
    Minor,
    /// `preminor`: a pre-release of the next minor version.
    Preminor,
    /// `patch`: compatible fixes.
    Patch,
    /// `prepatch`: a pre-release of the next patch version.
    Prepatch,
    /// `prerelease`: a change in the pre-release only.
    Prerelease,
}

impl Release {
    /// The lower-case name (`"major"`), as npm writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Release::Major => "major",
            Release::Premajor => "premajor",
            Release::Minor => "minor",
            Release::Preminor => "preminor",
            Release::Patch => "patch",
            Release::Prepatch => "prepatch",
            Release::Prerelease => "prerelease",
        }
    }
}
