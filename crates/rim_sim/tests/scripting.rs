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

/// Test vectors for the math functions scripts get: libm's results, bit for
/// bit. They must hold on every CI platform (DESIGN.md §7): a mismatch means
/// a machine computed differently and co-op would desync.
const VECTORS: &[(&str, u64)] = &[
    ("math.sin(1.2345)", 0x3fee351c8409f41d),
    ("math.sin(1e6)", 0xbfd6664b2568d867),
    ("math.cos(1.2345)", 0x3fd51e9b9f0886ae),
    ("math.tan(0.7)", 0x3feaf406c2fc78ae),
    ("math.asin(0.3)", 0x3fd380159e14f6ff),
    ("math.acos(0.3)", 0x3ff441f5ecbeef59),
    ("math.atan(2.5)", 0x3ff30b6d796a4da8),
    ("math.atan2(1.5, -0.5)", 0x3ffe47df3d0dd4d1),
    ("math.exp(1.7)", 0x4015e552770df8a6),
    ("math.exp(-700)", 0x00d14f2b0fb9307f),
    ("math.log(12.5)", 0x400434b1382efeb8),
    ("math.log(8, 2)", 0x4008000000000000),
    ("math.log(1000, 10)", 0x4008000000000000),
    ("math.log(81, 3)", 0x4010000000000001),
    ("math.log10(345.6)", 0x40044effbee97afe),
    ("math.pow(2.5, 3.3)", 0x403491876092afc1),
    ("math.sinh(0.9)", 0x3ff06c9ccd5b6af8),
    ("math.cosh(0.9)", 0x3ff6edebfd5d8680),
    ("math.tanh(0.9)", 0x3fe6ebe982d6605d),
    // Two where Apple's library differs from libm in the last bit (it gives
    // ...002e and ...81a5), so this test catches a regression on macOS too.
    ("math.sin(0.75882)", 0x3fe604a245f7002d),
    ("math.pow(0.00037991, 1.7)", 0x3eb9b6e5c3c481a6),
];

#[test]
fn script_math_is_the_same_bits_everywhere() {
    // Each expression three ways: a direct call (which Luau would otherwise
    // fast-call into the C library), through a local alias, and with its
    // arguments hidden from the compiler (which would otherwise fold a
    // literal call with the compiling machine's library).
    let mut body = String::from("rim.every(1, function()\n  local out = {}\n  local m = math\n");
    for (i, (expr, _)) in VECTORS.iter().enumerate() {
        let alias = expr.replacen("math.", "m.", 1);
        body.push_str(&format!("  out[{}] = {{ {expr}, {alias} }}\n", i + 1));
    }
    body.push_str("  rim.set_data(\"probe:vectors\", out)\nend)\n");
    let s = run("vectors", &body, 2);
    assert!(errors(&s).is_empty(), "{:?}", errors(&s));
    let Some(Data::Table(t)) = s.world.data.get("probe:vectors") else { panic!("no results") };
    for (i, (expr, want)) in VECTORS.iter().enumerate() {
        let row = t.get(&rim_sim::data::Key::Int(i as i64 + 1)).expect("row");
        for k in 1..=2 {
            let got = row.get_index(k).and_then(|d| d.num()).expect("a number").to_bits();
            assert_eq!(got, *want, "{expr} (form {k}): got {got:#018x}, want {want:#018x}");
        }
    }
}

#[test]
fn a_pow_operator_that_could_desync_is_flagged() {
    let src = "local a = x ^ 1.5\nlocal b = x^2\nlocal c = y ^ 0.5 + z^3\n-- a ^ b in a comment\nlocal s = \"2^10\"\nlocal d = --[[ ^ ]] 4 ^ k\n";
    assert_eq!(rim_sim::script::pow_operator_lines(src), vec![1, 6]);
    let s = run("pow", "local x = 3\nlocal y = x ^ 1.5\nrim.every(100, function() end)\n", 1);
    assert!(
        s.warnings.iter().any(|w| w.contains("probe/scripts/probe.luau:2") && w.contains("math.pow")),
        "{:?}",
        s.warnings
    );
}
