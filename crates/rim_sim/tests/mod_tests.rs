//! The shipped mods' own tests (`mods/*/tests/*.luau`), run the way
//! `rim test` runs them.

use rim_sim::modtest;
use std::path::Path;

#[test]
fn shipped_mods_pass_their_tests() {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut failed = Vec::new();
    let mut ran = 0;
    for m in ["core", "weather", "wildlife_plus"] {
        let results = modtest::run_mod(&mods.join(m), None).unwrap_or_else(|e| panic!("{m}: {e}"));
        assert!(!results.is_empty(), "{m} ships tests");
        for r in results {
            ran += 1;
            println!(
                "{} {}/{}: {} ({:.2} s)",
                if r.failure.is_none() { "ok  " } else { "FAIL" },
                r.mod_id,
                r.file,
                r.name,
                r.seconds
            );
            if let Some(f) = r.failure {
                failed.push(format!("{}/{}: {}\n{f}", r.mod_id, r.file, r.name));
            }
        }
    }
    assert!(failed.is_empty(), "{} of {ran} mod tests failed:\n\n{}", failed.len(), failed.join("\n\n"));
}
