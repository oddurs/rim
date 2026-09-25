//! The system font list cache: same fonts, much faster. Its own test binary,
//! because it points RIM_CACHE_DIR at a scratch folder for the process.

use rim_ui::fontcache::{system_fonts, FontsFrom};
use std::time::Instant;

fn faces(db: &cosmic_text::fontdb::Database) -> Vec<(String, u32, u16, String)> {
    let mut v: Vec<_> = db
        .faces()
        .filter_map(|f| match &f.source {
            cosmic_text::fontdb::Source::File(p) => Some((
                p.display().to_string(),
                f.index,
                f.weight.0,
                f.families.iter().map(|x| x.0.as_str()).collect::<Vec<_>>().join("|"),
            )),
            _ => None,
        })
        .collect();
    v.sort();
    v
}

#[test]
fn the_font_cache_gives_the_same_fonts_faster() {
    let dir = std::env::temp_dir().join(format!("rim-fontcache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: this test binary has one test and sets the variable before any
    // other thread reads it.
    unsafe { std::env::set_var("RIM_CACHE_DIR", &dir) };

    let t = Instant::now();
    let (scanned, from) = system_fonts();
    let scan_ms = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(from, FontsFrom::Scan, "no cache yet: a scan");
    assert!(dir.join("fonts-v1.txt").is_file(), "and the cache was written");

    // The best of a few loads: a wall-clock ratio from one sample is
    // noise when other tests share the machine (nextest runs them side by
    // side).
    let (mut cached, mut cache_ms) = (None, f64::MAX);
    for _ in 0..5 {
        let t = Instant::now();
        let (db, from) = system_fonts();
        cache_ms = cache_ms.min(t.elapsed().as_secs_f64() * 1e3);
        assert_eq!(from, FontsFrom::Cache, "a load after the first reads the cache");
        cached = Some(db);
    }
    let cached = cached.expect("loaded five times");
    assert_eq!(faces(&scanned), faces(&cached), "the same faces, families and weights");
    println!("system fonts: scan {scan_ms:.1} ms, cache {cache_ms:.1} ms, {} faces", scanned.len());
    if scanned.len() > 50 {
        assert!(cache_ms * 2.0 < scan_ms, "the cache is much faster ({cache_ms:.1} vs {scan_ms:.1} ms)");
    }

    // A changed font directory makes the cache stale.
    let text = std::fs::read_to_string(dir.join("fonts-v1.txt")).unwrap();
    let broken = text.replacen("\nD\t", "\nD\t/nonexistent/fonts", 1);
    std::fs::write(dir.join("fonts-v1.txt"), broken).unwrap();
    assert_eq!(system_fonts().1, FontsFrom::Scan, "a stale cache is rebuilt");
    let _ = std::fs::remove_dir_all(&dir);
}
