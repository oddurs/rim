//! Patches edit lists without owning them: append, remove, and edit
//! elements matched by a key, with conflicts reported like field sets.

mod common;

use common::test_mods;
use rim_sim::Sim;
use std::fs;

/// Core plus test mods `aa` and `bb` (loaded in that order), each with one
/// defs file.
fn load(name: &str, aa: &str, bb: &str) -> Result<Sim, String> {
    let mut extra: Vec<(&str, &[(&str, &str)])> = Vec::new();
    let a = [("defs/p.toml", aa)];
    let b = [("defs/p.toml", bb)];
    if !aa.is_empty() {
        extra.push(("aa", &a));
    }
    if !bb.is_empty() {
        extra.push(("bb", &b));
    }
    let dir = test_mods(name, &["core"], &extra);
    let s = Sim::new(&dir, 1);
    let _ = fs::remove_dir_all(dir);
    s
}

fn conflicts(s: &Sim) -> Vec<&String> {
    s.warnings.iter().filter(|w| w.contains("conflict")).collect()
}

#[test]
fn append_and_remove_edit_lists_in_place() {
    let s = load(
        "append",
        r#"
[[patch]]
target = "creature/deer"
append = { butcher = [{ thing = "wood", count = 2 }], spawn = { terrain = ["dirt"] } }

[[patch]]
target = "creature/human"
remove = { needs = ["warmth"] }
"#,
        r#"
[[patch]]
target = "creature/deer"
append = { butcher = [{ thing = "stone", count = 1 }] }
remove = { spawn = { terrain = ["grass"] } }
"#,
    )
    .unwrap_or_else(|e| panic!("loads: {e}"));
    let d = &s.world.defs;
    let deer = d.creature(d.creature_id("deer").unwrap());
    let yields: Vec<(&str, u32)> = deer.butcher.iter().map(|b| (b.thing.as_str(), b.count)).collect();
    assert_eq!(yields, [("raw_meat", 35), ("wood", 2), ("stone", 1)], "both mods' appends, in load order");
    assert_eq!(deer.spawn.as_ref().unwrap().terrain, ["rich_soil", "dirt"], "nested lists too");
    let human = d.creature(d.creature_id("human").unwrap());
    assert_eq!(human.needs, ["food", "rest"]);
    assert!(conflicts(&s).is_empty(), "appending to the same list isn't a conflict: {:?}", s.warnings);
}

#[test]
fn edit_changes_the_matched_element_only() {
    let s = load(
        "edit",
        r#"
[[patch]]
target = "thing/window"

[[patch.edit]]
list = "boundary"
match = { field = "temperature" }
set = { leak = 2.0 }

[[patch]]
target = "thing/wall"
remove = { boundary = [{ field = "temperature" }] }
"#,
        "",
    )
    .unwrap_or_else(|e| panic!("loads: {e}"));
    let d = &s.world.defs;
    let window = d.thing(d.thing_id("window").unwrap());
    let b: Vec<(&str, f64, f64)> = window.boundary.iter().map(|b| (b.field.as_str(), b.leak, b.pass)).collect();
    assert_eq!(b[0], ("light", 1.0, 0.35), "the light entry is untouched");
    assert_eq!(b[1].0, "temperature");
    assert_eq!(b[1].1, 2.0, "the matched entry is edited");
    let wall = d.thing(d.thing_id("wall").unwrap());
    assert!(wall.boundary.is_empty(), "a table pattern removes the element it matches");
}

#[test]
fn list_conflicts_are_reported() {
    // Two mods setting the same matched field conflict, like any field.
    let edit = r#"
[[patch]]
target = "thing/window"
[[patch.edit]]
list = "boundary"
match = { field = "temperature" }
set = { leak = LEAK }
"#;
    let s = load("edit-conflict", &edit.replace("LEAK", "2.0"), &edit.replace("LEAK", "3.0")).expect("loads");
    let c = conflicts(&s);
    assert!(
        c.iter().any(|w| w.contains("thing/window.boundary[field=\"temperature\"].leak set by both 'aa' and 'bb'")),
        "{c:?}"
    );

    // Replacing a whole list another mod appended to loses their edit.
    let s = load(
        "replace-conflict",
        "[[patch]]\ntarget = \"creature/deer\"\nappend = { butcher = [{ thing = \"wood\", count = 2 }] }\n",
        "[[patch]]\ntarget = \"creature/deer\"\nset = { butcher = [] }\n",
    )
    .expect("loads");
    let c = conflicts(&s);
    assert!(c.iter().any(|w| w.contains("'bb' sets creature/deer.butcher, replacing the list 'aa' edited")), "{c:?}");
}

#[test]
fn list_patch_mistakes_are_named() {
    let err = load("typo", "[[patch]]\ntarget = \"creature/deer\"\nsett = { hp = 1 }\n", "").err().expect("fails");
    assert!(err.contains("aa/defs/p.toml: patch on creature/deer has an unknown key 'sett'"), "{err}");
    let err = load("not-a-list", "[[patch]]\ntarget = \"creature/deer\"\nappend = { label = [\"x\"] }\n", "")
        .err()
        .expect("fails");
    assert!(err.contains("can't append to creature/deer.label: it isn't a list"), "{err}");
    let err = load(
        "bad-append",
        "[[patch]]\ntarget = \"creature/deer\"\nappend = { butcher = [{ thing = \"wood\" }] }\n",
        "",
    )
    .err()
    .expect("fails");
    assert!(
        err.contains("creature/deer") && err.contains("patched by 'aa'"),
        "a bad appended value names the mod: {err}"
    );

    let s = load(
        "no-match",
        r#"
[[patch]]
target = "creature/human"
remove = { needs = ["joy"] }
[[patch]]
target = "thing/wall"
[[patch.edit]]
list = "boundary"
match = { field = "light" }
set = { pass = 0.5 }
"#,
        "",
    )
    .expect("loads");
    assert!(
        s.warnings.iter().any(|w| w.contains("remove from creature/human.needs: nothing matched \"joy\"")),
        "{:?}",
        s.warnings
    );
    assert!(s.warnings.iter().any(|w| w.contains("no element matched")), "{:?}", s.warnings);
}

/// Every TOML sample in docs/modding/patches.md loads, in one mod beside core.
#[test]
fn guide_samples_load() {
    let guide = fs::read_to_string(common::mods().join("../docs/modding/patches.md")).unwrap().replace("\r\n", "\n");
    let samples: Vec<&str> = guide.split("```toml\n").skip(1).map(|b| b.split("```").next().unwrap()).collect();
    assert!(samples.len() >= 3, "found {} samples", samples.len());
    let s = load("guide", &samples.join("\n"), "").unwrap_or_else(|e| panic!("the guide's samples don't load: {e}"));
    assert!(conflicts(&s).is_empty(), "{:?}", s.warnings);
    let d = &s.world.defs;
    let bush = d.thing(d.thing_id("berry_bush").unwrap());
    assert_eq!(bush.harvest.as_ref().unwrap().regrow_days, 1.5);
}
