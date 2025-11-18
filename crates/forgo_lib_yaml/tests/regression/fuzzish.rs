use forgo_lib_yaml::{Doc, Elem, Node, Scalar, Seg, normalize_string_list};

/// Deterministic pseudo-random generator (xorshift64*).
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(2685821657736338717)) >> 32) as u32
    }
    fn pick<'a>(&mut self, xs: &'a [&'a str]) -> &'a str {
        let i = (self.next_u32() as usize) % xs.len();
        xs[i]
    }
}

fn synth_doc(mut r: Rng) -> String {
    // Build small randomized doc in supported subset
    let items = [
        "a",
        "b",
        "c",
        "zeta",
        "alpha",
        "forgo",
        "forgo_lib_cli",
        "tooling",
        "x-1",
        "x_2",
    ];
    let mut s = String::new();
    s.push_str("jobs:\n  build:\n    strategy:\n      matrix:\n        project: [");
    let n = 3 + (r.next_u32() % 4);
    for i in 0..n {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(r.pick(&items));
    }
    s.push_str("]\n");
    if r.next_u32() % 2 == 0 {
        s.push_str("list:\n");
        let m = 1 + (r.next_u32() % 4);
        for _ in 0..m {
            s.push_str("  - ");
            s.push_str(r.pick(&items));
            s.push('\n');
        }
    }
    s
}

#[test]
fn fuzzish_roundtrip_many() {
    for i in 0..200 {
        let s = synth_doc(Rng::new(i as u64 + 42));
        let d1 = Doc::from_str(&s).unwrap();
        let s2 = d1.to_string().unwrap();
        let d2 = Doc::from_str(&s2).unwrap();
        assert_eq!(d1, d2, "roundtrip mismatch for:\n{s}\n---\n{s2}");
    }
}

#[test]
fn fuzzish_edit_paths() {
    let mut d = Doc::from_str(&synth_doc(Rng::new(1))).unwrap();
    let path: &[Seg] = &[
        "jobs".into(),
        "build".into(),
        "strategy".into(),
        "matrix".into(),
        "project".into(),
    ];
    d.visit_sequences_mut(path, |seq| {
        let has_omega = seq.iter().any(|e| e.node().as_str() == Some("omega"));
        if !has_omega {
            seq.push(Elem::new(Node::Scalar(Scalar::Str("omega".into()))));
        }
        normalize_string_list(seq)
    });
    let out = d.to_string().unwrap();
    assert!(out.contains("omega"));
}
