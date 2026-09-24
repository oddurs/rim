//! Commands that run without a window: `rim test`.

use rim_sim::modtest;
use std::path::PathBuf;

const TEST_USAGE: &str = "usage: rim test [MOD_DIR ...] [--filter TEXT] [--junit FILE]

Runs each mod's tests/*.luau against seeded headless worlds. With no
MOD_DIR, tests every mod in ./mods. Exits 1 if any test fails.";

/// `rim test`: returns the process exit code.
pub fn test(args: &[String]) -> i32 {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let (mut filter, mut junit) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--filter" => filter = it.next().cloned(),
            "--junit" => junit = it.next().map(PathBuf::from),
            "-h" | "--help" => {
                println!("{TEST_USAGE}");
                return 0;
            }
            s if s.starts_with('-') => {
                eprintln!("rim test: unknown option {s}\n\n{TEST_USAGE}");
                return 2;
            }
            s => dirs.push(PathBuf::from(s)),
        }
    }
    if dirs.is_empty() {
        let mut found: Vec<PathBuf> = std::fs::read_dir("mods")
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.join("tests").is_dir()).collect())
            .unwrap_or_default();
        found.sort();
        if found.is_empty() {
            eprintln!("rim test: no mods with tests in ./mods\n\n{TEST_USAGE}");
            return 2;
        }
        dirs = found;
    }

    let mut results = Vec::new();
    for dir in &dirs {
        match modtest::run_mod(dir, filter.as_deref()) {
            Ok(r) => results.extend(r),
            Err(e) => {
                eprintln!("rim test: {e}");
                return 2;
            }
        }
    }
    for r in &results {
        let mark = if r.failure.is_none() { "ok  " } else { "FAIL" };
        println!("{mark} {}/{}: {} ({:.2} s)", r.mod_id, r.file, r.name, r.seconds);
    }
    let failed: Vec<_> = results.iter().filter(|r| r.failure.is_some()).collect();
    for r in &failed {
        println!("\n{}/{}: {}\n{}", r.mod_id, r.file, r.name, r.failure.as_deref().unwrap_or_default());
    }
    println!("\n{} passed, {} failed", results.len() - failed.len(), failed.len());
    if let Some(path) = junit {
        if let Err(e) = std::fs::write(&path, modtest::junit(&results)) {
            eprintln!("rim test: {}: {e}", path.display());
            return 2;
        }
    }
    i32::from(!failed.is_empty())
}
