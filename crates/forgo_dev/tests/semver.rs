//! `cargo x semver`.

use forgo_dev::Error;
use std::path::Path;

fn run(args: &[&str]) -> Result<String, Error> {
    let mut all = vec!["semver".to_string()];
    all.extend(args.iter().map(|a| a.to_string()));
    forgo_dev::run(&all, Path::new("."))
}

#[track_caller]
fn ok(args: &[&str]) -> String {
    run(args).unwrap_or_else(|e| panic!("{args:?}: {e:?}"))
}

#[track_caller]
fn unmet(args: &[&str]) -> String {
    match run(args) {
        Err(Error::Unmet(report)) => report,
        other => panic!("{args:?}: expected a failed check, got {other:?}"),
    }
}

#[track_caller]
fn usage(args: &[&str]) -> String {
    match run(args) {
        Err(e @ Error::Usage(_)) => e.to_string(),
        other => panic!("{args:?}: expected a usage error, got {other:?}"),
    }
}

#[test]
fn validate() {
    assert_eq!(
        ok(&["validate", "1.2.3", "18446744073709551616.0.0"]),
        "1.2.3: valid\n18446744073709551616.0.0: valid\n"
    );
    let report = unmet(&["validate", "1.2.3", "v1.2.3"]);
    assert!(
        report.contains("v1.2.3: invalid: unexpected character 'v'"),
        "{report}"
    );
    assert!(report.contains("--lenient"), "{report}");
    assert_eq!(ok(&["validate", "--lenient", "v1.2.3"]), "v1.2.3: valid\n");
    assert_eq!(
        unmet(&["validate", "--json", "1.2"]),
        "[{\"input\":\"1.2\",\"valid\":false,\"error\":\"missing the patch version at byte 3\"}]\n"
    );
}

#[test]
fn explain() {
    let v = ok(&["explain", "1.0.0-rc.1+build.5"]);
    assert!(
        v.contains("pre-release  rc.1 (rc: text, 1: number)\n"),
        "{v}"
    );
    assert!(v.contains("note         a pre-release"), "{v}");
    assert_eq!(
        ok(&["explain", "--json", "1.2.3-a.7"]),
        "{\"version\":\"1.2.3-a.7\",\"major\":1,\"minor\":2,\"patch\":3,\"pre\":[{\"value\":\"a\",\"numeric\":false},{\"value\":\"7\",\"numeric\":true}],\"build\":[]}\n"
    );
    assert_eq!(
        ok(&["explain", ">= 1.2, <1.8"]),
        "requirement  >= 1.2, <1.8 (Cargo)\ncanonical    >=1.2, <1.8\nlowest       1.2.0\n"
    );
    assert!(
        ok(&["explain", "--npm", "^1.2.3 || 2.x"])
            .contains("canonical    >=1.2.3 <2.0.0-0||>=2.0.0 <3.0.0-0\n")
    );
    assert!(ok(&["explain", "--npm", ">2 <1"]).contains("lowest       none"));
}

#[test]
fn compare() {
    assert_eq!(ok(&["compare", "1.2.3", "1.10.0"]), "1.2.3 < 1.10.0\n");
    assert_eq!(
        ok(&["compare", "1.0.0", "1.0.0-rc.1"]),
        "1.0.0 > 1.0.0-rc.1\n"
    );
    assert_eq!(
        ok(&["compare", "1.0.0+a", "1.0.0+b"]),
        "1.0.0+a = 1.0.0+b (same precedence; the build metadata differs)\n"
    );
    assert_eq!(
        ok(&["compare", "--json", "1.0.0+a", "1.0.0"]),
        "{\"a\":\"1.0.0+a\",\"b\":\"1.0.0\",\"precedence\":\"equal\",\"identical\":false}\n"
    );
}

#[test]
fn sort() {
    assert_eq!(
        ok(&[
            "sort",
            "1.0.0+b",
            "2.0.0",
            "1.0.0-rc.1",
            "1.0.0+a",
            "1.10.0",
            "1.9.0"
        ]),
        "1.0.0-rc.1\n1.0.0+a\n1.0.0+b  (same precedence as the line above)\n1.9.0\n1.10.0\n2.0.0\n"
    );
    assert_eq!(
        ok(&["sort", "--reverse", "1.0.0", "2.0.0"]),
        "2.0.0\n1.0.0\n"
    );
    assert_eq!(
        ok(&["sort", "--json", "1.0.0+b", "1.0.0+a", "0.1.0"]),
        "[[\"0.1.0\"],[\"1.0.0+a\",\"1.0.0+b\"]]\n"
    );
    assert!(matches!(
        run(&["sort", "1.0.0", "nope"]),
        Err(Error::Failed(_))
    ));
}

#[test]
fn bump() {
    for (release, version, id, expected) in [
        ("major", "1.2.3", None, "2.0.0"),
        ("minor", "1.2.3", None, "1.3.0"),
        ("patch", "1.2.3", None, "1.2.4"),
        ("premajor", "1.2.3", Some("rc"), "2.0.0-rc.0"),
        ("preminor", "1.2.3", None, "1.3.0-0"),
        ("prepatch", "1.2.3", Some("alpha"), "1.2.4-alpha.0"),
        (
            "prerelease",
            "1.2.4-alpha.0",
            Some("alpha"),
            "1.2.4-alpha.1",
        ),
        ("release", "2.0.0-rc.1", None, "2.0.0"),
        ("major", "2.0.0-rc.1", None, "2.0.0"),
        (
            "patch",
            "18446744073709551615.0.18446744073709551615",
            None,
            "18446744073709551615.0.18446744073709551616",
        ),
    ] {
        let mut args = vec!["bump", release, version];
        args.extend(id);
        assert_eq!(ok(&args), format!("{expected}\n"), "{args:?}");
    }
    assert!(matches!(
        run(&["bump", "release", "1.2.3"]),
        Err(Error::Failed(_))
    ));
    assert!(usage(&["bump", "major", "1.2.3", "rc"]).contains("takes no pre-release id"));
    assert!(usage(&["bump", "huge", "1.2.3"]).contains("unknown release"));
}

#[test]
fn matches() {
    assert_eq!(
        ok(&["matches", "^1.2", "1.4.0", "1.2.0"]),
        "1.4.0: matches\n1.2.0: matches\n"
    );
    assert_eq!(
        unmet(&["matches", "^1.2", "1.4.0", "2.0.0"]),
        "1.4.0: matches\n2.0.0: doesn't match\n"
    );
    assert_eq!(
        ok(&["matches", "--npm", "1.x || >=2.5", "2.6.0"]),
        "2.6.0: matches\n"
    );
    assert!(matches!(
        run(&["matches", "1.x || 2", "1.0.0"]),
        Err(Error::Failed(_))
    ));
}

#[test]
fn max_and_min() {
    assert_eq!(
        ok(&["max", "^1", "1.0.0", "1.9.3", "2.0.0", "1.9.3-rc.1"]),
        "1.9.3\n"
    );
    assert_eq!(
        ok(&["min", "--npm", ">=1.5", "1.0.0", "1.9.3", "1.5.0"]),
        "1.5.0\n"
    );
    assert_eq!(unmet(&["max", "^3", "1.0.0"]), "no version meets ^3\n");
    assert_eq!(
        unmet(&["max", "--json", "^3", "1.0.0"]),
        "{\"version\":null}\n"
    );
    assert_eq!(
        ok(&["max", "--json", "^1", "1.0.0"]),
        "{\"version\":\"1.0.0\"}\n"
    );
}

#[test]
fn usage_errors() {
    assert!(usage(&["frobnicate"]).contains("unknown command `frobnicate`"));
    assert!(usage(&["validate", "--fast", "1.2.3"]).contains("unknown option `--fast`"));
    assert!(usage(&["compare", "--npm", "1.0.0", "2.0.0"]).contains("`--npm` doesn't apply here"));
    assert!(usage(&["compare", "1.0.0"]).contains("expected two versions"));
    assert!(ok(&["help"]).contains("Exit codes: 0 yes, 1 no, 2 usage mistake."));
}
