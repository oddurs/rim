//! Def ids are namespaced by mod (DESIGN.md §10): `core:wall`. Inside a
//! mod a bare id means its own def; another mod's needs the prefix.

mod common;

use common::test_mods;
use rim_sim::data::Data;
use rim_sim::Sim;
use std::fs;

const IRON: &str = r##"
[[thing]]
id = "iron"
label = "iron"
color = "#8899aa"
category = "item"
"##;

fn load(name: &str, extra: &[(&str, &[(&str, &str)])]) -> Result<Sim, String> {
    let dir = test_mods(name, &["core"], extra);
    let s = Sim::new(&dir, 1);
    let _ = fs::remove_dir_all(dir);
    s
}

#[test]
fn two_mods_can_both_define_iron() {
    let s = load("iron", &[("aa", &[("defs/iron.toml", IRON)]), ("bb", &[("defs/iron.toml", IRON)])])
        .unwrap_or_else(|e| panic!("loads: {e}"));
    let d = &s.world.defs;
    let (a, b) = (d.lookup("thing", "aa:iron").unwrap(), d.lookup("thing", "bb:iron").unwrap());
    assert_ne!(a, b);
    assert_eq!(d.thing(a).id, "aa:iron");
    assert_eq!(d.lookup("thing", "iron"), None, "a bare id two mods define is ambiguous to tools");
    assert_eq!(d.thing(d.lookup("thing", "wood").unwrap()).id, "core:wood", "and unique bare ids still work");
}

#[test]
fn bare_references_are_the_mods_own() {
    // A creature that butchers into its own mod's iron, and core's meat.
    let creature = r##"
[[creature]]
id = "golem"
label = "golem"
color = "#777777"
size = 0.4
speed = 12
max_hp = 50
melee_damage = 5
melee_cooldown = 80
market_value = 10
butcher = [{ thing = "iron", count = 5 }, { thing = "core:raw_meat", count = 1 }]
"##;
    let s = load("own", &[("aa", &[("defs/iron.toml", IRON), ("defs/golem.toml", creature)])])
        .unwrap_or_else(|e| panic!("loads: {e}"));
    let d = &s.world.defs;
    let golem = d.creature(d.lookup("creature", "aa:golem").unwrap());
    let yields: Vec<&str> = golem.butcher_r.iter().map(|&(t, _)| d.thing(t).id.as_str()).collect();
    assert_eq!(yields, ["aa:iron", "core:raw_meat"]);

    // Core's meat without the prefix is this mod's meat, which doesn't exist.
    let bare = creature.replace("core:raw_meat", "raw_meat");
    let err = load("cross", &[("aa", &[("defs/iron.toml", IRON), ("defs/golem.toml", &bare)])])
        .err()
        .expect("a bare cross-mod reference must not load");
    assert!(
        err.contains(
            "creature/aa:golem: unknown thing 'aa:raw_meat': another mod's def needs its prefix, \"core:raw_meat\""
        ),
        "{err}"
    );
}

#[test]
fn a_mod_defines_only_its_own_ids() {
    let err = load("squat", &[("aa", &[("defs/iron.toml", &IRON.replace("\"iron\"", "\"core:iron\""))])])
        .err()
        .expect("defining in core's namespace must not load");
    assert!(err.contains("id 'core:iron' is in mod 'core's namespace"), "{err}");
    // Its own prefix is fine.
    load("own-prefix", &[("aa", &[("defs/iron.toml", &IRON.replace("\"iron\"", "\"aa:iron\""))])])
        .unwrap_or_else(|e| panic!("loads: {e}"));
}

#[test]
fn scripts_resolve_ids_in_their_own_mod() {
    let script = r#"
        rim.every(1, function()
            if rim.get_data("aa:done") then return end
            local x, y = rim.colony_center()
            local mine = rim.spawn_item("iron", x, y, 1)
            local cores = rim.spawn_item("core:wood", x, y, 1)
            local ok, err = pcall(rim.spawn_item, "wood", x, y, 1)
            local ids = {}
            for _, t in rim.thing_defs do
                ids[t.id] = true
            end
            rim.set_data("aa:done", true)
            rim.set_data("aa:r", { mine = mine, cores = cores, bare_core = ok, err = tostring(err),
                qualified = ids["aa:iron"] == true and ids["core:wood"] == true,
                temp = rim.ambient("core:temperature") ~= nil })
        end)
    "#;
    let mut s = load("scripts", &[("aa", &[("defs/iron.toml", IRON), ("scripts/probe.luau", script)])])
        .unwrap_or_else(|e| panic!("loads: {e}"));
    s.step();
    s.step();
    let feed: Vec<&str> = s.world.messages.iter().map(|m| m.text.as_str()).collect();
    let r = s.world.data.get("aa:r").unwrap_or_else(|| panic!("ran: {feed:?}"));
    assert_eq!(r.get("mine").and_then(|d| d.num()), Some(0.0), "a bare id is the mod's own iron");
    assert_eq!(r.get("cores").and_then(|d| d.num()), Some(0.0), "a prefixed id reaches core");
    assert_eq!(r.get("bare_core"), Some(&Data::Bool(false)), "core's wood needs the prefix");
    let Some(Data::Str(err)) = r.get("err") else { panic!("an error message") };
    assert!(err.contains("\"core:wood\""), "the error suggests the prefix: {err}");
    assert_eq!(r.get("qualified"), Some(&Data::Bool(true)), "defs come back with qualified ids");
    assert_eq!(r.get("temp"), Some(&Data::Bool(true)));
}
