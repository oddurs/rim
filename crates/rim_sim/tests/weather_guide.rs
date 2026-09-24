//! Every sample in docs/modding/weather.md runs: TOML blocks load as defs and
//! Luau blocks as scripts, in one mod beside core and weather. Blocks after
//! `<!-- not a sample -->` are API listings and are skipped.

mod common;

use common::test_mods;
use rim_sim::world::MsgKind;
use rim_sim::{Sim, TICKS_PER_DAY};
use std::fs;
use std::path::Path;

#[test]
fn guide_samples_run() {
    let guide =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/modding/weather.md")).unwrap();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut lines = guide.lines().peekable();
    let mut prev = "";
    while let Some(line) = lines.next() {
        let lang = line.strip_prefix("```").filter(|l| *l == "toml" || *l == "lua");
        if let Some(lang) = lang {
            let mut body = String::new();
            for l in lines.by_ref() {
                if l.starts_with("```") {
                    break;
                }
                body.push_str(l);
                body.push('\n');
            }
            if prev.trim() != "<!-- not a sample -->" {
                let n = files.len();
                let path = if lang == "toml" {
                    format!("defs/sample_{n:02}.toml")
                } else {
                    format!("scripts/sample_{n:02}.luau")
                };
                files.push((path, body));
            }
        }
        if !line.trim().is_empty() {
            prev = line;
        }
    }
    assert!(files.len() >= 6, "found {} samples", files.len());
    let refs: Vec<(&str, &str)> = files.iter().map(|(p, b)| (p.as_str(), b.as_str())).collect();
    let dir = test_mods("guide", &["core", "weather"], &[("guide", &refs)]);
    let mut s = Sim::new(&dir, 3).unwrap_or_else(|e| panic!("the guide's samples don't load: {e}"));
    for _ in 0..TICKS_PER_DAY / 2 {
        s.step();
    }
    let errors: Vec<&str> = s
        .world
        .messages
        .iter()
        .filter(|m| m.kind == MsgKind::Bad && m.text.contains("error"))
        .map(|m| m.text.as_str())
        .collect();
    assert!(errors.is_empty(), "sample script errors: {errors:?}");
    // The drizzle sample registered, and the two-suns sample took effect.
    let daylight = s.world.defs.lookup("field", "daylight").unwrap() as usize;
    let parts = s.world.fields.explain_ambient(&s.world.defs, daylight);
    assert!(parts.iter().any(|p| p.0 == "white_sun"), "{parts:?}");
    assert!(s.world.defs.sky.tint.contains_key("green_moon"));
    let _ = fs::remove_dir_all(dir);
}
