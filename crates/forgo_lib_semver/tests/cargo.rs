//! Cargo's requirement syntax.
//!
//! Most cases are the `semver` crate's own (tests/test_version_req.rs at
//! 1.0.28, MIT OR Apache-2.0, by David Tolnay), which define the syntax
//! Cargo uses. Matching and canonical text are checked as there; for
//! errors only that parsing fails, since messages differ. The one
//! deliberate difference: no limit on the number of comparators (the
//! `semver` crate rejects more than 32), nor on the size of numbers.

use forgo_lib_semver::{ErrorKind, Part, Req, Version};

#[track_caller]
fn req(text: &str) -> Req {
    Req::cargo(text).unwrap_or_else(|e| panic!("{text:?}: {e}"))
}

#[track_caller]
fn req_err(text: &str) -> forgo_lib_semver::Error {
    match Req::cargo(text) {
        Ok(r) => panic!("{text:?} parsed as {r}"),
        Err(e) => e,
    }
}

#[track_caller]
fn assert_to_string(r: &Req, expected: &str) {
    assert_eq!(r.to_string(), expected);
}

#[track_caller]
fn assert_match_all(r: &Req, versions: &[&str]) {
    for v in versions {
        assert!(
            r.matches(&Version::parse(v).unwrap()),
            "{r} did not match {v}"
        );
    }
}

#[track_caller]
fn assert_match_none(r: &Req, versions: &[&str]) {
    for v in versions {
        assert!(!r.matches(&Version::parse(v).unwrap()), "{r} matched {v}");
    }
}

#[test]
fn basic() {
    let r = &req("1.0.0");
    assert_to_string(r, "^1.0.0");
    assert_match_all(r, &["1.0.0", "1.1.0", "1.0.1"]);
    assert_match_none(r, &["0.9.9", "0.10.0", "0.1.0", "1.0.0-pre", "1.0.1-pre"]);
}

#[test]
fn exact() {
    let r = &req("=1.0.0");
    assert_to_string(r, "=1.0.0");
    assert_match_all(r, &["1.0.0"]);
    assert_match_none(r, &["1.0.1", "0.9.9", "0.10.0", "0.1.0", "1.0.0-pre"]);

    let r = &req("=0.9.0");
    assert_to_string(r, "=0.9.0");
    assert_match_all(r, &["0.9.0"]);
    assert_match_none(r, &["0.9.1", "1.9.0", "0.0.9", "0.9.0-pre"]);

    let r = &req("=0.0.2");
    assert_to_string(r, "=0.0.2");
    assert_match_all(r, &["0.0.2"]);
    assert_match_none(r, &["0.0.1", "0.0.3", "0.0.2-pre"]);

    let r = &req("=0.1.0-beta2.a");
    assert_to_string(r, "=0.1.0-beta2.a");
    assert_match_all(r, &["0.1.0-beta2.a"]);
    assert_match_none(r, &["0.9.1", "0.1.0", "0.1.1-beta2.a", "0.1.0-beta2"]);

    let r = &req("=0.1.0+meta");
    assert_to_string(r, "=0.1.0");
    assert_match_all(r, &["0.1.0", "0.1.0+meta", "0.1.0+any"]);
}

#[test]
fn greater_than() {
    let r = &req(">= 1.0.0");
    assert_to_string(r, ">=1.0.0");
    assert_match_all(r, &["1.0.0", "2.0.0"]);
    assert_match_none(r, &["0.1.0", "0.0.1", "1.0.0-pre", "2.0.0-pre"]);

    let r = &req(">= 2.1.0-alpha2");
    assert_to_string(r, ">=2.1.0-alpha2");
    assert_match_all(r, &["2.1.0-alpha2", "2.1.0-alpha3", "2.1.0", "3.0.0"]);
    assert_match_none(
        r,
        &["2.0.0", "2.1.0-alpha1", "2.0.0-alpha2", "3.0.0-alpha2"],
    );
}

#[test]
fn less_than() {
    let r = &req("< 1.0.0");
    assert_to_string(r, "<1.0.0");
    assert_match_all(r, &["0.1.0", "0.0.1"]);
    assert_match_none(r, &["1.0.0", "1.0.0-beta", "1.0.1", "0.9.9-alpha"]);

    let r = &req("<= 2.1.0-alpha2");
    assert_match_all(r, &["2.1.0-alpha2", "2.1.0-alpha1", "2.0.0", "1.0.0"]);
    assert_match_none(
        r,
        &["2.1.0", "2.2.0-alpha1", "2.0.0-alpha2", "1.0.0-alpha2"],
    );

    assert_match_all(&req(">1.0.0-alpha, <1.0.0"), &["1.0.0-beta"]);
    assert_match_none(&req(">1.0.0-alpha, <1.0"), &["1.0.0-beta"]);
    assert_match_none(&req(">1.0.0-alpha, <1"), &["1.0.0-beta"]);
}

#[test]
fn multiple() {
    let r = &req("> 0.0.9, <= 2.5.3");
    assert_to_string(r, ">0.0.9, <=2.5.3");
    assert_match_all(r, &["0.0.10", "1.0.0", "2.5.3"]);
    assert_match_none(r, &["0.0.8", "2.5.4"]);

    let r = &req("0.3.0, 0.4.0");
    assert_to_string(r, "^0.3.0, ^0.4.0");
    assert_match_none(r, &["0.0.8", "0.3.0", "0.4.0"]);

    let r = &req("<= 0.2.0, >= 0.5.0");
    assert_to_string(r, "<=0.2.0, >=0.5.0");
    assert_match_none(r, &["0.0.8", "0.3.0", "0.5.1"]);

    let r = &req("0.1.0, 0.1.4, 0.1.6");
    assert_to_string(r, "^0.1.0, ^0.1.4, ^0.1.6");
    assert_match_all(r, &["0.1.6", "0.1.9"]);
    assert_match_none(r, &["0.1.0", "0.1.4", "0.2.0"]);

    assert_eq!(
        req_err("> 0.1.0,").kind(),
        ErrorKind::UnexpectedEnd(Part::Major)
    );
    assert_eq!(
        req_err("> 0.3.0, ,").kind(),
        ErrorKind::UnexpectedChar(Part::Major, ',')
    );

    let r = &req(">=0.5.1-alpha3, <0.6");
    assert_to_string(r, ">=0.5.1-alpha3, <0.6");
    assert_match_all(
        r,
        &[
            "0.5.1-alpha3",
            "0.5.1-alpha4",
            "0.5.1-beta",
            "0.5.1",
            "0.5.5",
        ],
    );
    assert_match_none(
        r,
        &["0.5.1-alpha1", "0.5.2-alpha3", "0.5.5-pre", "0.5.0-pre"],
    );
    assert_match_none(r, &["0.6.0", "0.6.0-pre"]);

    assert_eq!(
        req_err("1.2.3 - 2.3.4").kind(),
        ErrorKind::ExpectedComma('-')
    );
}

#[test]
fn any_number_of_comparators() {
    // The `semver` crate stops at 32 ("excessive number of version
    // comparators").
    let text: Vec<String> = (1..=100).map(|i| format!(">{i}")).collect();
    let r = &req(&text.join(", "));
    assert_match_all(r, &["101.0.0", "200.3.4"]);
    assert_match_none(r, &["100.9.9", "1.0.0"]);
}

#[test]
fn numbers_of_any_size() {
    let r = &req(">=18446744073709551616.0.0");
    assert_match_all(
        r,
        &["18446744073709551616.0.0", "99999999999999999999999.0.0"],
    );
    assert_match_none(r, &["18446744073709551615.99.99"]);
    let r = &req("^18446744073709551615");
    assert_match_all(r, &["18446744073709551615.7.0"]);
    assert_match_none(r, &["18446744073709551616.0.0"]);
    assert_to_string(r, "^18446744073709551615");
}

#[test]
fn whitespace_delimited_comparator_sets() {
    assert_eq!(
        req_err("> 0.0.9 <= 2.5.3").kind(),
        ErrorKind::ExpectedComma('<')
    );
}

#[test]
fn tilde() {
    let r = &req("~1");
    assert_match_all(r, &["1.0.0", "1.0.1", "1.1.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "0.0.9"]);

    let r = &req("~1.2");
    assert_match_all(r, &["1.2.0", "1.2.1"]);
    assert_match_none(r, &["1.1.1", "1.3.0", "0.0.9"]);

    let r = &req("~1.2.2");
    assert_match_all(r, &["1.2.2", "1.2.4"]);
    assert_match_none(r, &["1.2.1", "1.9.0", "1.0.9", "2.0.1", "0.1.3"]);

    let r = &req("~1.2.3-beta.2");
    assert_match_all(r, &["1.2.3", "1.2.4", "1.2.3-beta.2", "1.2.3-beta.4"]);
    assert_match_none(r, &["1.3.3", "1.1.4", "1.2.3-beta.1", "1.2.4-beta.2"]);
}

#[test]
fn caret() {
    let r = &req("^1");
    assert_match_all(r, &["1.1.2", "1.1.0", "1.2.1", "1.0.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "0.1.4"]);
    assert_match_none(r, &["1.0.0-beta1", "0.1.0-alpha", "1.0.1-pre"]);

    let r = &req("^1.1");
    assert_match_all(r, &["1.1.2", "1.1.0", "1.2.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.0.1", "0.1.4"]);

    let r = &req("^1.1.2");
    assert_match_all(r, &["1.1.2", "1.1.4", "1.2.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1"]);
    assert_match_none(r, &["1.1.2-alpha1", "1.1.3-alpha1", "2.9.0-alpha1"]);

    let r = &req("^0.1.2");
    assert_match_all(r, &["0.1.2", "0.1.4"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1"]);
    assert_match_none(r, &["0.1.2-beta", "0.1.3-alpha", "0.2.0-pre"]);

    let r = &req("^0.5.1-alpha3");
    assert_match_all(
        r,
        &[
            "0.5.1-alpha3",
            "0.5.1-alpha4",
            "0.5.1-beta",
            "0.5.1",
            "0.5.5",
        ],
    );
    assert_match_none(
        r,
        &[
            "0.5.1-alpha1",
            "0.5.2-alpha3",
            "0.5.5-pre",
            "0.5.0-pre",
            "0.6.0",
        ],
    );

    let r = &req("^0.0.2");
    assert_match_all(r, &["0.0.2"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1", "0.1.4"]);

    let r = &req("^0.0");
    assert_match_all(r, &["0.0.2", "0.0.0"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.1.4"]);

    let r = &req("^0");
    assert_match_all(r, &["0.9.1", "0.0.2", "0.0.0"]);
    assert_match_none(r, &["2.9.0", "1.1.1"]);

    let r = &req("^1.4.2-beta.5");
    assert_match_all(
        r,
        &["1.4.2", "1.4.3", "1.4.2-beta.5", "1.4.2-beta.6", "1.4.2-c"],
    );
    assert_match_none(
        r,
        &[
            "0.9.9",
            "2.0.0",
            "1.4.2-alpha",
            "1.4.2-beta.4",
            "1.4.3-beta.5",
        ],
    );
}

#[test]
fn wildcard() {
    assert_eq!(req_err("").kind(), ErrorKind::UnexpectedEnd(Part::Major));

    let r = &req("*");
    assert_match_all(r, &["0.9.1", "2.9.0", "0.0.9", "1.0.1", "1.1.1"]);
    assert_match_none(r, &["1.0.0-pre"]);
    for s in ["x", "X"] {
        assert_eq!(*r, req(s));
    }

    let r = &req("1.*");
    assert_match_all(r, &["1.2.0", "1.2.1", "1.1.1", "1.3.0"]);
    assert_match_none(r, &["0.0.9", "1.2.0-pre"]);
    for s in ["1.x", "1.X", "1.*.*"] {
        assert_eq!(*r, req(s));
    }

    let r = &req("1.2.*");
    assert_match_all(r, &["1.2.0", "1.2.2", "1.2.4"]);
    assert_match_none(r, &["1.9.0", "1.0.9", "2.0.1", "0.1.3", "1.2.2-pre"]);
    for s in ["1.2.x", "1.2.X"] {
        assert_eq!(*r, req(s));
    }
}

#[test]
fn logical_or_is_npm_syntax() {
    assert_eq!(
        req_err("=1.2.3 || =2.3.4").kind(),
        ErrorKind::ExpectedComma('|')
    );
    assert_eq!(
        req_err("1.1 || =1.2.3").kind(),
        ErrorKind::ExpectedComma('|')
    );
    assert_eq!(
        req_err("6.* || 8.* || >= 10.*").kind(),
        ErrorKind::ExpectedComma('|')
    );
}

#[test]
fn pre() {
    assert_match_all(&req("=2.1.1-really.0"), &["2.1.1-really.0"]);
}

#[test]
fn parse_errors() {
    assert_eq!(
        req_err("\0").kind(),
        ErrorKind::UnexpectedChar(Part::Major, '\0')
    );
    assert_eq!(
        req_err(">= >= 0.0.2").kind(),
        ErrorKind::UnexpectedChar(Part::Major, '>')
    );
    assert_eq!(
        req_err(">== 0.0.2").kind(),
        ErrorKind::UnexpectedChar(Part::Major, '=')
    );
    assert_eq!(
        req_err("a.0.0").kind(),
        ErrorKind::UnexpectedChar(Part::Major, 'a')
    );
    assert_eq!(
        req_err("1.0.0-").kind(),
        ErrorKind::EmptyIdentifier(Part::Pre)
    );
    assert_eq!(req_err(">=").kind(), ErrorKind::UnexpectedEnd(Part::Major));
}

#[test]
fn comparators() {
    assert_to_string(&req("1.2.3-alpha"), "^1.2.3-alpha");
    assert_to_string(&req("2.X"), "2.*");
    assert_to_string(&req("2"), "^2");
    assert_to_string(&req("2.x.x"), "2.*");
    assert_eq!(
        req_err("1.2.3-01").kind(),
        ErrorKind::LeadingZero(Part::Pre)
    );
    assert_eq!(
        req_err("1.2.3+4.").kind(),
        ErrorKind::EmptyIdentifier(Part::Build)
    );
    assert_eq!(req_err(">").kind(), ErrorKind::UnexpectedEnd(Part::Major));
    assert_eq!(req_err("1.").kind(), ErrorKind::UnexpectedEnd(Part::Minor));
    assert_eq!(req_err("1.*.").kind(), ErrorKind::UnexpectedAfterWildcard);
    assert!(Req::cargo("1.2.3+4ÿ").is_err());
}

#[test]
fn cargo3202() {
    let r = &req("0.*.*");
    assert_to_string(r, "0.*");
    assert_match_all(r, &["0.5.0"]);
    assert_to_string(&req("0.0.*"), "0.0.*");
}

#[test]
fn digit_after_wildcard() {
    for text in ["*.1", "1.*.1", ">=1.*.1"] {
        assert_eq!(
            req_err(text).kind(),
            ErrorKind::UnexpectedAfterWildcard,
            "{text}"
        );
    }
}

#[test]
fn eq_and_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    fn hash(value: &Req) -> u64 {
        let mut h = DefaultHasher::new();
        value.hash(&mut h);
        h.finish()
    }
    assert!(req("^1") == req("^1"));
    assert_eq!(hash(&req("^1")), hash(&req("^1")));
    assert!(req("^1") != req("^2"));
    assert_eq!(req(">= 1.0"), req(">=1.0"));
    assert_eq!(hash(&req(">= 1.0")), hash(&req(">=1.0")));
}

#[test]
fn leading_digit_in_pre_and_build() {
    for op in ["=", ">", ">=", "<", "<=", "~", "^"] {
        for v in [
            "1.2.3-1a",
            "1.2.3+1a",
            "1.2.3-01a",
            "1.2.3+01",
            "1.2.3-1+1",
            "1.2.3-1-1+1-1-1",
            "1.2.3-1a+1a",
            "1.2.3-1a-1a+1a-1a-1a",
        ] {
            req(&format!("{op} {v}"));
        }
    }
}

#[test]
fn wildcard_and_another() {
    for text in ["*, 0.20.0-any", "0.20.0-any, *", "0.20.0-any, *, 1.0"] {
        assert_eq!(
            req_err(text).kind(),
            ErrorKind::WildcardNotTheOnlyComparator('*'),
            "{text}"
        );
    }
}
