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

/// `rim check`: every shipped mod loads with only its dependencies, with no
/// warnings and no script errors in its first hours.
#[test]
fn shipped_mods_check_clean() {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    for m in ["core", "weather", "wildlife_plus"] {
        let r = modtest::check_mod(&mods.join(m), 6.0).unwrap_or_else(|e| panic!("{m}: {e}"));
        assert!(r.warnings.is_empty() && r.errors.is_empty(), "{m}: {:?} {:?}", r.warnings, r.errors);
    }
}

/// A mod slow on this machine is reported apart from its warnings: the
/// time depends on the machine, and a slow CI runner failed a clean mod.
#[test]
fn a_slow_mod_is_a_note_not_a_warning() {
    let dir = std::env::temp_dir().join(format!("rim-check-slow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/core");
    copy(&core, &dir.join("core"));
    let m = dir.join("slowpoke");
    std::fs::create_dir_all(m.join("scripts")).unwrap();
    std::fs::write(
        m.join("mod.toml"),
        format!(
            "id = \"slowpoke\"\nname = \"s\"\nversion = \"0.1.0\"\napi = \"{}.{}\"\ndepends = [\"core\"]\n",
            rim_sim::API_VERSION.0,
            rim_sim::API_VERSION.1
        ),
    )
    .unwrap();
    std::fs::write(
        m.join("scripts/slow.luau"),
        "rim.every(1, function() local t = {} for i = 1, 20000 do t[i % 64 + 1] = tostring(i) end end)\n",
    )
    .unwrap();
    let r = modtest::check_mod(&m, 1.0).unwrap();
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(r.slow.iter().any(|s| s.contains("slowpoke")), "{:?}", r.slow);
    let _ = std::fs::remove_dir_all(dir);
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}
