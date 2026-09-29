//! The shipped mods' own tests (`mods/*/tests/*.luau`), run the way
//! `rim test` runs them.

use rim_sim::modtest;
use std::path::Path;

#[test]
fn shipped_mods_pass_their_tests() {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut failed = Vec::new();
    let mut ran = 0;
    for m in ["core", "primitive", "weather", "wildlife_plus"] {
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
    for m in ["core", "crafting", "primitive", "weather", "wildlife_plus"] {
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

/// The modding guide's sky example (docs/modding/examples/two_suns) loads
/// clean on its own and beside the weather plugin, and passes its tests
/// there: two suns, a moon with phases, and a storm that dims them.
#[test]
fn the_two_suns_example_loads_and_passes_its_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = std::env::temp_dir().join(format!("rim-two-suns-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for m in ["core", "weather"] {
        copy(&root.join("mods").join(m), &dir.join(m));
    }
    copy(&root.join("docs/modding/examples/two_suns"), &dir.join("two_suns"));
    let r = modtest::check_mod(&dir.join("two_suns"), 6.0).unwrap();
    assert!(r.warnings.is_empty() && r.errors.is_empty(), "{:?} {:?}", r.warnings, r.errors);
    // Beside the weather too, nothing conflicts; and its moon's tint joins
    // core's.
    let sim = rim_sim::Sim::build(&dir, 1, &|_| true, 64).unwrap();
    assert!(sim.warnings.is_empty(), "{:?}", sim.warnings);
    let tints: Vec<&str> = sim.world.defs.sky.tint.keys().map(|k| k.as_str()).collect();
    assert_eq!(tints, ["dawn", "dusk", "green_moon", "overcast"]);
    let results = modtest::run_mod(&dir.join("two_suns"), None).unwrap();
    let failed: Vec<_> =
        results.iter().filter_map(|r| r.failure.as_ref().map(|f| format!("{}: {f}", r.name))).collect();
    assert!(results.len() >= 4 && failed.is_empty(), "{} tests: {failed:#?}", results.len());
    let _ = std::fs::remove_dir_all(dir);
}

/// A world with no seed named gets one from the test's name, not seed 1, and
/// a failure says how the seed was made and how to run the test again with
/// it (DESIGN.md §7b).
#[test]
fn a_world_with_no_seed_gets_one_from_the_test_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = std::env::temp_dir().join(format!("rim-seeded-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(&root.join("mods/core"), &dir.join("core"));
    let m = dir.join("seeded");
    std::fs::create_dir_all(m.join("tests")).unwrap();
    std::fs::write(
        m.join("mod.toml"),
        "id = \"seeded\"\nname = \"seeded\"\nversion = \"0.1.0\"\napi = \"0.8\"\ndepends = [\"core\"]\n",
    )
    .unwrap();
    std::fs::write(
        m.join("tests/seed.luau"),
        r#"
test("which map", function(t)
    local w = t.world({ size = 64 })
    local again = t.world({ size = 64, seed = w:seed() })
    t.expect(again:hash()).to_be(w:hash())
    error("seed " .. string.format("%d", w:seed()))
end)
test("a named map", function(t)
    local w = t.world({ size = 64, seed = 5 })
    error("seed " .. string.format("%d", w:seed()))
end)
"#,
    )
    .unwrap();
    let results = modtest::run_mod(&m, None).unwrap();
    let failure = |name: &str| results.iter().find(|r| r.name == name).and_then(|r| r.failure.clone()).unwrap();
    let want = rim_sim::testseed::seed("seeded/tests/seed.luau/which map");
    assert_ne!(want.seed, 1);
    let derived = failure("which map");
    assert!(
        derived.contains(&format!("seed {}\n", want.seed)) || derived.contains(&format!("seed {} ", want.seed)),
        "{derived}"
    );
    assert!(derived.contains(&format!("{}; rerun: RIM_SEED={} rim test ", want.describe(), want.seed)), "{derived}");
    assert!(derived.ends_with("--filter \"which map\""), "{derived}");
    let named = failure("a named map");
    assert!(named.contains("seed 5") && named.contains("\nrerun: rim test ") && !named.contains("RIM_SEED"), "{named}");
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
