//! The seed corpus, `tests/seeds.toml` (DESIGN.md §7b): every kept map plays
//! a day and round-trips its save, as the nightly sweep plays its seeds.

use rim_sim::seeds;
use std::path::Path;

#[test]
fn every_corpus_seed_plays_a_day_and_round_trips() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let corpus = seeds::corpus(&root.join("tests/seeds.toml")).unwrap();
    assert!(!corpus.is_empty());
    let mods = root.join("../../mods");
    let failed: Vec<String> = corpus
        .iter()
        .filter_map(|s| {
            seeds::soak(&mods, &|_| true, s.seed, 1).err().map(|f| {
                format!("{} (seed {}): tick {}: {}; rerun: {}", s.id, s.seed, f.tick, f.reason, seeds::repro(s.seed, 1))
            })
        })
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
fn a_corpus_seed_says_why_and_ids_are_unique() {
    let dir = std::env::temp_dir().join(format!("rim-corpus-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("seeds.toml");
    std::fs::write(&file, "[[seed]]\nid = \"a\"\nseed = 1\nwhy = \"x\"\n[[seed]]\nid = \"a\"\nseed = 2\nwhy = \"y\"\n")
        .unwrap();
    assert!(seeds::corpus(&file).unwrap_err().contains("two seeds are called a"));
    std::fs::write(&file, "[[seed]]\nid = \"a\"\nseed = 1\n").unwrap();
    assert!(seeds::corpus(&file).unwrap_err().contains("why"));
    let _ = std::fs::remove_dir_all(dir);
}
