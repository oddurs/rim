//! Luau in the sim VM: how fast typical script work runs.
//!
//!   cargo run --release -p rim_sim --example luau_bench
//!
//! Loads core plus a benchmark mod whose hook does script-shaped work (math
//! library calls, table building, pairs/ipairs, string formatting, calls into
//! `rim`) and reports the hook's time. Used to check the VM configuration:
//! safe environments, compiler level, the interrupt budget.

use rim_sim::Sim;
use std::fs;
use std::path::Path;
use std::time::Instant;

const BENCH: &str = r#"
local function work()
    local acc = 0
    local t = {}
    for i = 1, 200000 do
        acc += math.floor(i / 3) + math.abs(-i) % 7 + math.max(i, 5)
        t[i % 512 + 1] = { x = i, y = acc % 97 }
    end
    for _, v in pairs(t) do
        acc += v.x - v.y
    end
    for i, v in ipairs(t) do
        acc += i
    end
    local s = {}
    for i = 1, 5000 do
        s[#s + 1] = string.format("%d:%d", i, acc % 13)
    end
    for i = 1, 20000 do
        acc += rim.ticks_per_day % (i % 11 + 1)
    end
    return acc + #table.concat(s, ",")
end

local done = false
rim.every(1, function()
    if done then return end
    done = true
    rim.set_data("bench:result", work())
end)
"#;

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

fn main() {
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let dir = std::env::temp_dir().join(format!("rim-luau-bench-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy_dir(&shipped.join("core"), &dir.join("core"));
    let m = dir.join("bench");
    fs::create_dir_all(m.join("scripts")).unwrap();
    fs::write(
        m.join("mod.toml"),
        "id = \"bench\"\nname = \"bench\"\nversion = \"0.1.0\"\napi = \"0.1\"\ndepends = [\"core\"]\n",
    )
    .unwrap();
    fs::write(m.join("scripts/bench.luau"), BENCH).unwrap();

    let mut best = f64::MAX;
    for _ in 0..7 {
        let mut s = Sim::new(&dir, 1).expect("loads");
        // The hook runs within the first tick; time the whole step.
        let t = Instant::now();
        s.step();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        assert!(s.world.data.contains_key("bench:result"), "the bench hook ran");
    }
    println!("luau bench: best of 7 = {best:.2} ms");
    let _ = fs::remove_dir_all(dir);
}
