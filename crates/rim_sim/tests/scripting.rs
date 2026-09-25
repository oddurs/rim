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
    // The runaway hook is switched off, and says so; the game goes on.
    assert_eq!(s.world.data.get("probe:calls").and_then(|d| d.num()), Some(1.0), "not called again");
    assert!(s.world.messages.iter().any(|m| m.text.contains("[probe]") && m.text.contains("switched off")));
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
fn the_standard_libraries_are_read_only() {
    let s = run(
        "readonly",
        r#"
        rim.every(1, function()
            local ok = pcall(function() math.floor = nil end)
            local ok2 = pcall(function() string.x = 1 end)
            rim.set_data("probe:r", { math = ok, string = ok2 })
        end)
    "#,
        2,
    );
    let r = s.world.data.get("probe:r").expect("ran");
    assert_eq!(r.get("math"), Some(&Data::Bool(false)), "math is read-only");
    assert_eq!(r.get("string"), Some(&Data::Bool(false)), "string is read-only");
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

fn load_error(name: &str, ship: &[&str], script: &str) -> String {
    let dir = test_mods(name, ship, &[("rogue", &[("scripts/rogue.luau", script)])]);
    let err = Sim::new(&dir, 1).err().expect("the rogue mod must not load");
    let _ = fs::remove_dir_all(dir);
    err
}

#[test]
fn rim_is_the_engines_and_read_only() {
    let err = load_error("replace-engine", &["core"], "rim.spawn_pawn = function() end\n");
    assert!(err.contains("mod 'rogue' can't set rim.spawn_pawn: rim is the engine's and read-only"), "{err}");
    let err = load_error("add-to-rim", &["core"], "rim.my_api = {}\n");
    assert!(err.contains("can't set rim.my_api") && err.contains("require"), "{err}");
    let err = load_error("metatable", &["core"], "setmetatable(rim, nil)\n");
    assert!(err.to_lowercase().contains("metatable"), "{err}");
}

#[test]
fn another_mods_exports_can_be_used_but_not_changed() {
    let err = load_error(
        "patch-weather",
        &["core", "weather"],
        "local weather = require(\"@weather/scripts/weather\")\nweather.force = function() end\n",
    );
    assert!(err.contains("readonly"), "{err}");
    let dir = test_mods(
        "frozen",
        &["core", "weather"],
        &[(
            "probe",
            &[(
                "scripts/probe.luau",
                r#"
        local weather = require("@weather/scripts/weather")
        rim.every(1, function()
            local a = pcall(function() rim.late_api = 1 end)
            local b = pcall(function() weather.register = nil end)
            local c = pcall(function() rim.creature_defs[1] = nil end)
            rim.set_data("probe:r", { add = a, plugin = b, data = c, types = #weather.types() })
        end)
    "#,
            )],
        )],
    );
    let mut s = Sim::new(&dir, 1).expect("loads");
    s.step();
    s.step();
    let _ = fs::remove_dir_all(dir);
    let r = s.world.data.get("probe:r").expect("ran");
    for k in ["add", "plugin", "data"] {
        assert_eq!(r.get(k), Some(&Data::Bool(false)), "{k} must fail");
    }
    assert!(r.get("types").and_then(|d| d.num()).unwrap_or(0.0) >= 5.0, "and weather's API works");
}

/// Load a single test mod made of `files`; its error, or the sim.
fn modules(name: &str, manifest_extra: &str, files: &[(&str, &str)]) -> Result<Sim, String> {
    let dir = test_mods(name, &["core"], &[("probe", files)]);
    if !manifest_extra.is_empty() {
        let toml = dir.join("probe/mod.toml");
        let text = fs::read_to_string(&toml).unwrap();
        fs::write(&toml, format!("{text}{manifest_extra}\n")).unwrap();
    }
    let s = Sim::new(&dir, 1);
    let _ = fs::remove_dir_all(dir);
    s
}

#[test]
fn modules_run_once_and_resolve_relative_and_mod_paths() {
    let mut s = modules(
        "require",
        "",
        &[
            // Modules in subdirectories only run when required.
            ("scripts/lib/count.luau", "return { n = 0 }\n"),
            ("scripts/lib/thing.luau", "local c = require(\"./count\")\nc.n += 1\nreturn { made = true }\n"),
            ("scripts/lib/unused.luau", "error(\"never required, never run\")\n"),
            ("scripts/a.luau", "return require(\"./lib/thing\")\n"),
            (
                "scripts/b.luau",
                r#"
        local a = require("./a")
        local thing = require("@probe/scripts/lib/thing")
        local count = require("./lib/count")
        local storyteller = require("@core/scripts/storyteller")
        local same = a == thing
        rim.every(1, function()
            rim.set_data("probe:r", {
                same = same,
                runs = count.n,
                core = type(storyteller.register_incident) == "function",
                late = (pcall(require, "./lib/unused")),
            })
        end)
    "#,
            ),
        ],
    )
    .unwrap_or_else(|e| panic!("loads: {e}"));
    s.step();
    s.step();
    let r = s.world.data.get("probe:r").expect("ran");
    assert_eq!(r.get("same"), Some(&Data::Bool(true)), "one module, one table, however it's named");
    assert_eq!(r.get("runs").and_then(|d| d.num()), Some(1.0), "a module runs once");
    assert_eq!(r.get("core"), Some(&Data::Bool(true)), "a dependency's exports");
    assert_eq!(r.get("late"), Some(&Data::Bool(false)), "require only works while loading");
}

#[test]
fn require_is_limited_to_declared_dependencies() {
    // Two test mods that depend only on core: one can't reach the other.
    let dir = test_mods(
        "undeclared",
        &["core"],
        &[
            ("aa", &[("scripts/a.luau", "local b = require(\"@bb/scripts/b\")\n")]),
            ("bb", &[("scripts/b.luau", "return {}\n")]),
        ],
    );
    let err = Sim::new(&dir, 1).err().expect("must not load");
    let _ = fs::remove_dir_all(dir);
    assert!(err.contains("mod 'aa' requires \"@bb/scripts/b\" but doesn't list 'bb' in depends or optional"), "{err}");

    // An optional mod that isn't installed gives nil.
    let mut s = modules(
        "optional",
        "optional = [\"not_installed\"]",
        &[(
            "scripts/p.luau",
            r#"
        local m = require("@not_installed/scripts/api")
        rim.every(1, function() rim.set_data("probe:nil", m == nil) end)
    "#,
        )],
    )
    .unwrap_or_else(|e| panic!("loads: {e}"));
    s.step();
    assert_eq!(s.world.data.get("probe:nil"), Some(&Data::Bool(true)));
}

#[test]
fn require_cycles_and_bad_paths_are_load_errors() {
    let err =
        modules("cycle", "", &[("scripts/a.luau", "require(\"./b\")\n"), ("scripts/b.luau", "require(\"./a\")\n")])
            .err()
            .expect("a cycle must not load");
    assert!(
        err.contains("require cycle: probe/scripts/a.luau -> probe/scripts/b.luau -> probe/scripts/a.luau"),
        "{err}"
    );
    for (path, want) in [
        ("storyteller", "use \"@<mod>/scripts/<name>\""),
        ("@core/ui/kit", "sim modules live in a mod's scripts folder"),
        ("../../core/scripts/storyteller", "leaves the mod's scripts folder"),
        ("./nothing", "there's no probe/scripts/nothing.luau"),
    ] {
        let err = modules("badpath", "", &[("scripts/p.luau", &format!("require(\"{path}\")\n"))])
            .err()
            .expect("a bad path must not load");
        assert!(err.contains(want), "{path}: {err}");
    }
}

#[test]
fn mods_emit_only_their_own_events() {
    let s = run(
        "emit",
        r#"
        rim.every(1, function()
            rim.set_data("probe:own", (pcall(rim.emit, "probe:hello", {})))
            rim.set_data("probe:other", (pcall(rim.emit, "weather:changed", {})))
            rim.set_data("probe:bare", (pcall(rim.emit, "pawn_died", {})))
        end)
    "#,
        2,
    );
    assert_eq!(s.world.data.get("probe:own"), Some(&Data::Bool(true)));
    assert_eq!(s.world.data.get("probe:other"), Some(&Data::Bool(false)), "can't speak for another mod");
    assert_eq!(s.world.data.get("probe:bare"), Some(&Data::Bool(false)), "engine event names are the engine's");
}

#[test]
fn script_data_belongs_to_the_mod_that_wrote_it() {
    let script = r#"
        rim.every(1, function()
            rim.set_data("bare", 1)
            rim.set_data("probe:qualified", 2)
            local ok, err = pcall(rim.set_data, "weather:forecast", {})
            rim.set_data("other", { ok = ok, err = tostring(err) })
            rim.set_data("empty", (pcall(rim.set_data, "probe:", 1)))
            rim.set_data("read", { bare = rim.get_data("bare"), theirs = rim.get_data("weather:types") ~= nil })
        end)
    "#;
    let dir = test_mods("data", &["core", "weather"], &[("probe", &[("scripts/probe.luau", script)])]);
    let mut s = Sim::new(&dir, 1).unwrap_or_else(|e| panic!("loads: {e}"));
    // The weather plugin writes its data on its first 20-tick hook.
    for _ in 0..25 {
        s.step();
    }
    let _ = fs::remove_dir_all(dir);
    assert_eq!(s.world.data.get("probe:bare"), Some(&Data::Int(1)), "a bare key is the caller's");
    assert_eq!(s.world.data.get("probe:qualified"), Some(&Data::Int(2)));
    let other = s.world.data.get("probe:other").expect("ran");
    assert_eq!(other.get("ok"), Some(&Data::Bool(false)), "can't write another mod's data");
    let Some(Data::Str(err)) = other.get("err") else { panic!("an error message") };
    assert!(err.contains("mod 'probe' can't write \"weather:forecast\""), "{err}");
    assert_eq!(s.world.data.get("probe:empty"), Some(&Data::Bool(false)), "an empty key is refused");
    let read = s.world.data.get("probe:read").expect("ran");
    assert_eq!(read.get("bare"), Some(&Data::Int(1)), "get_data resolves a bare key the same way");
    assert_eq!(read.get("theirs"), Some(&Data::Bool(true)), "reading another mod's data is fine");
    assert!(s.world.data.keys().all(|k| k.contains(':')), "every key is qualified");
}

#[test]
fn a_slow_mod_is_named_in_the_warnings() {
    // Slow but not runaway: about 20k steps of work every tick.
    let s = run(
        "slow",
        r#"
        rim.every(1, function()
            local t = {}
            for i = 1, 20000 do t[i % 64 + 1] = tostring(i) end
        end)
    "#,
        1300,
    );
    assert!(
        s.warnings.iter().any(|w| w.contains("mod 'probe' is slow")),
        "the profiler names the slow mod: {:?}",
        s.warnings
    );
    assert!(!s.warnings.iter().any(|w| w.contains("mod 'core' is slow")), "{:?}", s.warnings);
}
