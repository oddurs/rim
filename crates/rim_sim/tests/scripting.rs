//! The sim's Luau VM: sandboxing, limits and determinism guards.

mod common;

use common::test_mods;
use rim_sim::data::Data;
use rim_sim::Sim;
use std::fs;

fn run(name: &str, script: &str, ticks: u32) -> Sim {
    let dir = test_mods(name, &["core"], &[("probe", &[("scripts/probe.luau", script)])]);
    let mut s = Sim::new(&dir, 1).unwrap_or_else(|e| panic!("loads: {e}"));
    for _ in 0..ticks {
        s.step();
    }
    let _ = fs::remove_dir_all(dir);
    s
}

fn errors(s: &Sim) -> Vec<String> {
    s.world.messages.iter().filter(|m| m.text.contains("script error")).map(|m| m.text.clone()).collect()
}

#[test]
fn an_endless_loop_is_stopped_and_the_game_goes_on() {
    let s = run(
        "endless",
        r#"
        local n = 0
        rim.every(5, function()
            n += 1
            rim.set_data("probe:calls", n)
            if n == 1 then
                while true do end
            end
        end)
    "#,
        20,
    );
    let e = errors(&s);
    assert!(e.iter().any(|m| m.contains("endless loop")), "{e:?}");
    // The hook still runs on later ticks.
    assert!(s.world.data.get("probe:calls").and_then(|d| d.num()).unwrap_or(0.0) >= 3.0);
}

#[test]
fn running_out_of_memory_fails_the_script_not_the_game() {
    let s = run(
        "memory",
        r#"
        rim.every(5, function()
            local t = {}
            for i = 1, 1e9 do
                t[i] = string.rep("x", 1024) .. i
            end
        end)
    "#,
        6,
    );
    let e = errors(&s);
    assert!(e.iter().any(|m| m.to_lowercase().contains("memory")), "{e:?}");
}

#[test]
fn nondeterministic_and_escape_hatches_are_gone() {
    let s = run(
        "globals",
        r#"
        rim.every(1, function()
            local seen = {}
            for _, name in { "os", "io", "debug", "collectgarbage", "gcinfo", "loadstring", "getfenv", "setfenv", "newproxy" } do
                if _G[name] ~= nil then
                    table.insert(seen, name)
                end
            end
            if math.random ~= nil then table.insert(seen, "math.random") end
            rim.set_data("probe:seen", table.concat(seen, ","))
        end)
    "#,
        2,
    );
    assert_eq!(s.world.data.get("probe:seen"), Some(&Data::Str(String::new())), "{:?}", errors(&s));
}

#[test]
fn libraries_are_read_only_but_rim_is_open_to_plugins() {
    let s = run(
        "readonly",
        r#"
        rim.my_plugin_api = function() return 7 end
        rim.every(1, function()
            local ok = pcall(function() math.floor = nil end)
            local ok2 = pcall(function() string.x = 1 end)
            rim.set_data("probe:r", { math = ok, string = ok2, api = rim.my_plugin_api() })
        end)
    "#,
        2,
    );
    let r = s.world.data.get("probe:r").expect("ran");
    assert_eq!(r.get("math"), Some(&Data::Bool(false)), "math is read-only");
    assert_eq!(r.get("string"), Some(&Data::Bool(false)), "string is read-only");
    assert_eq!(r.get("api").and_then(|d| d.num()), Some(7.0), "rim accepts plugin APIs");
}
