//! Command lines and crate names.

use forgo_dev::cli::{Command, Kind, parse_args};
use forgo_dev::names::{validate_name, validate_new};

fn parse(args: &[&str]) -> forgo_dev::Result<Command> {
    parse_args(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
}

fn usage_error(args: &[&str]) -> String {
    let e = parse(args).expect_err("a usage error");
    assert!(e.is_usage(), "{e}");
    e.to_string()
}

#[test]
fn new() {
    assert_eq!(
        parse(&["new", "lib", "forgo_lib_x"]).unwrap(),
        Command::New {
            kind: Kind::Lib,
            name: "forgo_lib_x".into(),
            public: false
        }
    );
    assert_eq!(
        parse(&["new", "bin", "forgo_x", "--public"]).unwrap(),
        Command::New {
            kind: Kind::Bin,
            name: "forgo_x".into(),
            public: true
        }
    );
    assert!(usage_error(&["new"]).contains("expected <lib|bin|wasm>"));
    assert!(usage_error(&["new", "dll", "x"]).contains("unknown kind `dll`"));
    assert!(usage_error(&["new", "lib"]).contains("expected <name>"));
    assert!(usage_error(&["new", "lib", "x", "--publc"]).contains("unexpected argument `--publc`"));
    assert!(usage_error(&["new", "lib", "x", "--public", "--public"]).contains("unexpected"));
}

#[test]
fn rm() {
    assert_eq!(
        parse(&["rm", "forgo_x"]).unwrap(),
        Command::Rm {
            kind: None,
            name: "forgo_x".into(),
            yes: false
        }
    );
    assert_eq!(
        parse(&["rm", "lib", "forgo_lib_x", "--yes"]).unwrap(),
        Command::Rm {
            kind: Some(Kind::Lib),
            name: "forgo_lib_x".into(),
            yes: true
        }
    );
    assert_eq!(
        parse(&["rm", "forgo_x", "-y"]).unwrap(),
        Command::Rm {
            kind: None,
            name: "forgo_x".into(),
            yes: true
        }
    );
    assert!(usage_error(&["rm"]).contains("expected <name>"));
    assert!(usage_error(&["rm", "x", "--force"]).contains("unexpected argument `--force`"));
}

#[test]
fn help_and_unknown() {
    for h in ["help", "--help", "-h"] {
        assert_eq!(parse(&[h]).unwrap(), Command::Help);
    }
    assert!(usage_error(&[]).contains("expected a command"));
    assert!(usage_error(&["publish"]).contains("unknown command `publish`"));
}

#[test]
fn names_cannot_be_paths() {
    for bad in [
        "", "..", ".", "../x", "a/b", "a\\b", "/abs", "C:", "x.y", "-x", "_x", "9x", "X", "aB",
        "a b", "é",
    ] {
        assert!(validate_name(bad).is_err(), "{bad:?}");
    }
    for good in ["a", "forgo", "forgo_lib_yaml", "x9", "a_"] {
        assert!(validate_name(good).is_ok(), "{good:?}");
    }
    assert!(validate_name(&"a".repeat(64)).is_ok());
    assert!(validate_name(&"a".repeat(65)).is_err());
}

#[test]
fn names_windows_reserves() {
    for bad in ["con", "nul", "aux", "prn", "com1", "lpt9"] {
        assert!(validate_name(bad).is_err(), "{bad:?}");
    }
    assert!(validate_name("console").is_ok());
}

#[test]
fn names_by_kind() {
    assert!(validate_new(Kind::Lib, "forgo_lib_x").is_ok());
    assert!(validate_new(Kind::Lib, "forgo_x").is_err());
    assert!(validate_new(Kind::Lib, "forgo_lib_x_wasm").is_err());
    assert!(validate_new(Kind::Wasm, "forgo_lib_x_wasm").is_ok());
    assert!(validate_new(Kind::Wasm, "forgo_lib_x").is_err());
    assert!(validate_new(Kind::Wasm, "forgo_x_wasm").is_err());
    assert!(validate_new(Kind::Bin, "forgo_x").is_ok());
    assert!(validate_new(Kind::Bin, "forgo_lib_x").is_err());
    assert!(validate_new(Kind::Bin, "x_wasm").is_err());
    // The kind rule is checked after the path rule.
    let e = validate_new(Kind::Bin, "../x").unwrap_err().to_string();
    assert!(e.contains("invalid crate name"), "{e}");
}
