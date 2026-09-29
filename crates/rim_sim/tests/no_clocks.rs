//! Tests count work; benches measure time (DESIGN.md §8a). A test that reads
//! the clock either is timed alone, in CI's budget step, and says so by
//! checking `RIM_BUDGETS`, or only prints what it measured and is listed here.

use std::fs;
use std::path::{Path, PathBuf};

/// Measurements that print and assert nothing on the time, and why they read
/// the clock at all.
const PRINTS_ONLY: &[&str] = &[
    // A spike's numbers, kept for the record.
    "crates/rim_sim/tests/boundary_spike.rs",
    // Prints how long the gap search took; asserts the gaps it found.
    "crates/rim_client/src/roomstate.rs",
    // Print the stock pass's cost a tick; they assert the cells it works.
    "crates/rim_sim/tests/ground.rs",
    "crates/rim_sim/tests/stock_fields.rs",
    // Takes Instant::now() as an origin for made-up frame times; measures none.
    "crates/rim_client/src/frames.rs",
    // This file names the clock calls it looks for.
    "crates/rim_sim/tests/no_clocks.rs",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The body of every item marked `#[cfg(test)]`, found by matching braces.
/// A `{` or `}` in a string or char literal would throw the count off; rim's
/// test modules have only balanced ones, in format strings.
fn cfg_test_items(text: &str) -> String {
    let mut out = String::new();
    for (at, _) in text.match_indices("#[cfg(test)]") {
        let rest = &text[at..];
        let Some(open) = rest.find('{') else { continue };
        // `#[cfg(test)] use ...;` ends before any brace.
        if rest[..open].contains(';') {
            continue;
        }
        let mut depth = 0;
        for (i, c) in rest[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                out.push_str(&rest[..open + i + 1]);
                out.push('\n');
                break;
            }
        }
    }
    out
}

#[test]
fn a_test_module_is_found_among_other_code() {
    let src = "fn main() { Instant::now(); }\n#[cfg(test)]\nmod tests {\n    fn t() { let f = format!(\"{x}\"); SystemTime::now(); }\n}\nfn after() { Instant::now(); }\n";
    let tests = cfg_test_items(src);
    assert!(tests.contains("SystemTime::now"), "{tests}");
    assert!(!tests.contains("Instant::now"), "code outside the module isn't test code:\n{tests}");
}

#[test]
fn no_test_reads_the_clock_unless_timed_alone() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for krate in ["rim_sim", "rim_ui", "rim_client"] {
        let mut files = Vec::new();
        rust_files(&root.join("crates").join(krate), &mut files);
        for f in files {
            let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            if PRINTS_ONLY.contains(&rel.as_str()) {
                continue;
            }
            let text = fs::read_to_string(&f).unwrap();
            // Integration tests are all test code; in a source file, only the
            // items under `#[cfg(test)]` are.
            let tests = if rel.contains("/tests/") { text.clone() } else { cfg_test_items(&text) };
            if tests.contains("RIM_BUDGETS") || tests.contains("timing_budgets()") {
                continue;
            }
            for line in tests.lines().filter(|l| l.contains("Instant::now") || l.contains("SystemTime::now")) {
                found.push(format!("{rel}: {}", line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "test code reads the clock: count the work instead, or time it alone behind RIM_BUDGETS \
         (DESIGN.md §8a):\n{}",
        found.join("\n")
    );
}
