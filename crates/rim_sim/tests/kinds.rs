//! Def kinds a mod declares with `[[kind]]`: entries from any mod load,
//! patch and conflict like built-in defs, and scripts read them.

mod common;

use common::test_mods;
use rim_sim::data::{Data, Key};
use rim_sim::Sim;
use std::fs;

const KIND: &str = r#"
[[kind]]
id = "spell"

[kind.fields]
label = "string"
mana = "int"
school = { type = "string", default = "fire" }

[[spell]]
id = "spark"
label = "spark"
mana = 2
"#;

/// Core, a `magic` mod declaring the kind, and mods `aa` and `bb` after it.
fn load(name: &str, magic: &str, aa: &str, bb: &str) -> Result<Sim, String> {
    let (m, a, b) = ([("defs/m.toml", magic)], [("defs/a.toml", aa)], [("defs/b.toml", bb)]);
    let mut extra: Vec<(&str, &[(&str, &str)])> = vec![("magic", &m)];
    if !aa.is_empty() {
        extra.push(("aa", &a));
    }
    if !bb.is_empty() {
        extra.push(("bb", &b));
    }
    let dir = test_mods(name, &["core"], &extra);
    // aa and bb depend on magic too, so they load after it.
    for id in ["aa", "bb"] {
        let toml = dir.join(id).join("mod.toml");
        if let Ok(text) = fs::read_to_string(&toml) {
            fs::write(&toml, text.replace("depends = [\"core\"]", "depends = [\"core\", \"magic\"]")).unwrap();
        }
    }
    let s = Sim::new(&dir, 1);
    let _ = fs::remove_dir_all(dir);
    s
}

fn spells(s: &Sim) -> Vec<(String, i64, String)> {
    s.world.defs.mod_defs["magic:spell"]
        .iter()
        .map(|d| {
            let get = |k: &str| match d {
                Data::Table(t) => t.get(&Key::Str(k.into())).cloned(),
                _ => None,
            };
            let s = |k: &str| match get(k) {
                Some(Data::Str(v)) => v,
                _ => String::new(),
            };
            let mana = match get("mana") {
                Some(Data::Int(i)) => i,
                _ => -1,
            };
            (s("id"), mana, s("school"))
        })
        .collect()
}

#[test]
fn a_mod_kind_loads_from_several_mods_and_patches() {
    let s = load(
        "kinds",
        KIND,
        "[[magic.spell]]\nid = \"frost\"\nlabel = \"frost\"\nmana = 3\nschool = \"ice\"\n",
        "[[patch]]\ntarget = \"magic:spell/magic:spark\"\nset = { mana = 1 }\n",
    )
    .unwrap_or_else(|e| panic!("loads: {e}"));
    assert_eq!(
        spells(&s),
        [("magic:spark".into(), 1, "fire".into()), ("aa:frost".into(), 3, "ice".into())],
        "in load order, with defaults, ids qualified, and bb's patch applied"
    );
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
}

#[test]
fn mod_kind_conflicts_and_schema_errors() {
    let set = |mana: i32| format!("[[patch]]\ntarget = \"magic:spell/magic:spark\"\nset = {{ mana = {mana} }}\n");
    let s = load("kind-conflict", KIND, &set(5), &set(6)).expect("loads");
    assert!(
        s.warnings.iter().any(|w| w.contains("magic:spell/magic:spark.mana set by both 'aa' and 'bb'")),
        "{:?}",
        s.warnings
    );

    let err = load("kind-missing", KIND, "[[magic.spell]]\nid = \"dud\"\nlabel = \"dud\"\n", "").err().expect("fails");
    assert!(err.contains("magic:spell/aa:dud") && err.contains("missing field `mana` (int)"), "{err}");
    let err = load("kind-type", KIND, "[[magic.spell]]\nid = \"dud\"\nlabel = \"dud\"\nmana = \"lots\"\n", "")
        .err()
        .expect("fails");
    assert!(err.contains("`mana` should be int, not string"), "{err}");

    let s = load("kind-extra", KIND, "[[magic.spell]]\nid = \"x\"\nlabel = \"x\"\nmana = 1\ncolour = \"red\"\n", "")
        .expect("loads");
    assert!(s.warnings.iter().any(|w| w.contains("`colour` isn't a field of this kind")), "{:?}", s.warnings);

    // A bare kind name only means the mod's own kind.
    let s = load("kind-bare", KIND, "[[spell]]\nid = \"x\"\nlabel = \"x\"\nmana = 1\n", "").expect("loads");
    assert!(s.warnings.iter().any(|w| w.contains("unknown def kind 'spell' ignored (did you mean [[magic.spell]]?)")));
}

#[test]
fn scripts_read_mod_kinds() {
    let script = r#"
        local mine = rim.defs("spell")
        rim.every(1, function()
            local theirs = rim.defs("magic:spell")
            local ok = pcall(rim.defs, "nothing")
            rim.set_data("magic:r", { n = #mine, first = mine[1].id, school = theirs[1].school, unknown = ok })
        end)
    "#;
    let dir =
        test_mods("kind-script", &["core"], &[("magic", &[("defs/m.toml", KIND), ("scripts/probe.luau", script)])]);
    let mut s = Sim::new(&dir, 1).unwrap_or_else(|e| panic!("loads: {e}"));
    let _ = fs::remove_dir_all(dir);
    s.step();
    s.step();
    let r = s.world.data.get("magic:r").expect("ran");
    assert_eq!(r.get("n").and_then(|d| d.num()), Some(1.0));
    assert_eq!(r.get("first"), Some(&Data::Str("magic:spark".into())));
    assert_eq!(r.get("school"), Some(&Data::Str("fire".into())), "defaults reach scripts");
    assert_eq!(r.get("unknown"), Some(&Data::Bool(false)), "an unknown kind is an error");
}
