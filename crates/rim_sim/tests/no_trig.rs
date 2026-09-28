//! The sim calls no std trig or transcendental function: those go to the
//! platform's libm, which differs in the last bits from machine to machine,
//! and one bit is enough to split lockstep (agree). `rim_sim::sky` has the
//! in-crate ones, built from + − × ÷; `sqrt`, which IEEE 754 requires to be
//! correctly rounded, is allowed.

use std::fs;
use std::path::{Path, PathBuf};

const BANNED: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sin_cos", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh",
    "exp", "exp2", "exp_m1", "ln", "ln_1p", "log2", "log10", "powf", "powi", "cbrt", "hypot",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Calls of a banned name in one line of code, comments dropped: as a
/// method (`x.sin()`) or through the float type (`f64::sin(x)`). `log` is
/// also a method name of rim's own, so `x.log(b)` counts only with a number
/// for its base.
fn calls(line: &str) -> Vec<String> {
    let code = line.split("//").next().unwrap_or("");
    let mut found: Vec<String> = BANNED
        .iter()
        .flat_map(|name| [format!(".{name}("), format!("f64::{name}("), format!("f32::{name}(")])
        .filter(|pat| code.contains(pat.as_str()))
        .collect();
    for (i, _) in code.match_indices(".log(") {
        if code[i + 5..].starts_with(|c: char| c.is_ascii_digit() || c == '-') {
            found.push(".log(".into());
        }
    }
    found
}

#[test]
fn the_sim_calls_no_libm() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.len() > 20, "found the sources: {}", files.len());
    let mut hits = Vec::new();
    for f in &files {
        for (n, line) in fs::read_to_string(f).unwrap().lines().enumerate() {
            for c in calls(line) {
                hits.push(format!("{}:{}: {c}", f.strip_prefix(&src).unwrap().display(), n + 1));
            }
        }
    }
    assert!(hits.is_empty(), "use rim_sim::sky's maths:\n{}", hits.join("\n"));
}

#[test]
fn the_guard_finds_what_it_bans() {
    for bad in ["let y = x.sin();", "a.atan2(b)", "f64::exp(t)", "(v * 2.0).powf(1.5)", "x.log(10.0)", "t.ln()"] {
        assert!(!calls(bad).is_empty(), "{bad}");
    }
    for fine in ["self.log(sim)?;", "sky::sin(x)", "x.sqrt()", "// x.sin() in a comment", "a.cosine(b)"] {
        assert!(calls(fine).is_empty(), "{fine}: {:?}", calls(fine));
    }
}
