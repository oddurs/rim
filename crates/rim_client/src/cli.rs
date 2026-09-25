//! Commands that run without a window: `rim test`, `rim check`.

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

const CHECK_USAGE: &str = "usage: rim check [MOD_DIR ...] [--hours H] [--strict]

Loads each mod with its dependencies, runs a few in-game hours, and reports
load errors, warnings (patch conflicts, skipped patches, determinism hazards)
and script errors. Its sprites must decode and fit the world atlas. Its UI
scripts are checked against the UI API: a ui_api
the engine lacks, or a ui., act. or view. member that does not exist, is an
error naming the file and line. With no MOD_DIR, checks every mod in ./mods.
Exits 1 on an error, or on a warning with --strict.";

/// Each PNG under the mod's `sprites/`, at any depth, must decode and fit
/// a world atlas page, or the game refuses to start with it.
fn sprite_errors(dir: &std::path::Path) -> Vec<String> {
    fn pngs(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for p in rd.flatten().map(|e| e.path()) {
            if p.is_dir() {
                pngs(&p, out);
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    pngs(&dir.join("sprites"), &mut files);
    files.sort();
    files
        .iter()
        .filter_map(|f| match rim_ui::image::decode(f) {
            Err(e) => Some(format!("{}: {e}", f.display())),
            Ok((w, h, _)) if !crate::atlas::fits(w, h) => Some(format!(
                "{}: {w}×{h} doesn't fit a {m}×{m} world atlas page",
                f.display(),
                m = crate::atlas::MAX_PAGE
            )),
            Ok(_) => None,
        })
        .collect()
}

/// `rim check`: returns the process exit code.
pub fn check(args: &[String]) -> i32 {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let (mut hours, mut strict) = (6.0, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--hours" => hours = it.next().and_then(|h| h.parse().ok()).unwrap_or(hours),
            "--strict" => strict = true,
            "-h" | "--help" => {
                println!("{CHECK_USAGE}");
                return 0;
            }
            s if s.starts_with('-') => {
                eprintln!("rim check: unknown option {s}\n\n{CHECK_USAGE}");
                return 2;
            }
            s => dirs.push(PathBuf::from(s)),
        }
    }
    if dirs.is_empty() {
        let mut found: Vec<PathBuf> = std::fs::read_dir("mods")
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.join("mod.toml").is_file()).collect())
            .unwrap_or_default();
        found.sort();
        dirs = found;
    }
    if dirs.is_empty() {
        eprintln!("rim check: no mods in ./mods\n\n{CHECK_USAGE}");
        return 2;
    }
    let mut failed = false;
    for dir in &dirs {
        match modtest::check_mod(dir, hours) {
            Err(e) => {
                println!("FAIL {}\n  {e}", dir.display());
                failed = true;
            }
            Ok(r) => {
                let mut ui = rim_ui::check::check_mod_ui(dir);
                ui.extend(sprite_errors(dir));
                let bad = !r.errors.is_empty() || !ui.is_empty() || (strict && !r.warnings.is_empty());
                let mark = if bad { "FAIL" } else { "ok  " };
                println!(
                    "{mark} {} (with {}): {} warnings, {} script errors, {} UI errors",
                    r.mod_id,
                    r.mods.join(", "),
                    r.warnings.len(),
                    r.errors.len(),
                    ui.len()
                );
                for w in &r.warnings {
                    println!("  warning: {w}");
                }
                for e in &r.errors {
                    println!("  error: {}", e.lines().next().unwrap_or_default());
                }
                for e in &ui {
                    println!("  error: {e}");
                }
                failed |= bad;
            }
        }
    }
    i32::from(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sprite_that_does_not_decode_is_an_error_naming_it() {
        let dir = std::env::temp_dir().join(format!("rim-sprite-check-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sprites/deep")).unwrap();
        std::fs::write(dir.join("sprites/deep/broken.PNG"), b"not a png").unwrap();
        std::fs::copy("../../mods/wildlife_plus/sprites/salt_lick.png", dir.join("sprites/fine.png")).unwrap();
        let errors = sprite_errors(&dir);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("broken.PNG"), "found in a subfolder, any case: {errors:?}");
    }
}
