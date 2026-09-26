//! `rim test`: a mod's `tests/*.luau`, run against real, seeded, headless
//! worlds (DESIGN.md §7). A test builds a world from a seed and a mod set,
//! drives it with the player's commands or the mods' own exports, advances
//! it, and checks what happened. The engine is deterministic, so a failure
//! reproduces exactly from the seed and tick it prints.
//!
//! ```luau
//! test("a raid brings raiders", function(t)
//!     local w = t.world({ seed = 7 })
//!     w:call("@core/scripts/storyteller", "fire", "raid", { points = 300 })
//!     t.expect(w:count_pawns("hostile")).to_be_at_least(1)
//! end)
//! ```
//!
//! Test code runs in its own Luau VM, apart from the worlds it drives; each
//! world has its own sim VM, exactly as in a game.

use crate::command::Command;
use crate::data::{from_lua, to_lua};
use crate::modloader::{discover, ModManifest};
use crate::world::{Blueprint, Faction, Held, Pawn, Thing};
use crate::{IVec, Sim, TICKS_PER_DAY};
use mlua::{Function, Lua, MultiValue, Table, UserData, UserDataMethods, Value};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

/// One test's outcome.
#[derive(Clone, Debug)]
pub struct TestResult {
    pub mod_id: String,
    /// The test file, relative to the mod: `tests/storyteller.luau`.
    pub file: String,
    pub name: String,
    /// None if it passed; the failure, with where and how to reproduce it.
    pub failure: Option<String>,
    pub seconds: f64,
}

/// Expectations, in Luau so a failure points at the test's own line.
const PRELUDE: &str = r#"
local function show(v)
	if type(v) == "string" then
		return string.format("%q", v)
	end
	return tostring(v)
end

local function fail(msg)
	error(msg .. __rim_where(), 3)
end

function expect(v)
	local e = {}
	function e.to_be(x)
		if v ~= x then fail(`expected {show(x)}, got {show(v)}`) end
	end
	function e.never_to_be(x)
		if v == x then fail(`expected anything but {show(x)}`) end
	end
	function e.to_be_truthy()
		if not v then fail(`expected a true value, got {show(v)}`) end
	end
	function e.to_be_falsy()
		if v then fail(`expected false or nil, got {show(v)}`) end
	end
	function e.to_be_at_least(n)
		if type(v) ~= "number" or v < n then fail(`expected at least {show(n)}, got {show(v)}`) end
	end
	function e.to_be_at_most(n)
		if type(v) ~= "number" or v > n then fail(`expected at most {show(n)}, got {show(v)}`) end
	end
	function e.to_be_near(x, within)
		within = within or 1e-6
		if type(v) ~= "number" or math.abs(v - x) > within then
			fail(`expected {show(x)} ± {show(within)}, got {show(v)}`)
		end
	end
	function e.to_contain(x)
		if type(v) == "string" then
			if not string.find(v, x, 1, true) then fail(`expected {show(v)} to contain {show(x)}`) end
		elseif type(v) == "table" then
			if not table.find(v, x) then fail(`expected the list to contain {show(x)}`) end
		else
			fail(`expected a string or list, got {show(v)}`)
		end
	end
	return e
end
"#;

/// The world a test last built or touched, for "seed 7, tick 1234" in a
/// failure message.
type Last = Rc<RefCell<Option<(u64, Rc<RefCell<Sim>>)>>>;

/// A world handle in test code.
struct World {
    sim: Rc<RefCell<Sim>>,
    seed: u64,
    last: Last,
}

fn rt(msg: impl Into<String>) -> mlua::Error {
    mlua::Error::runtime(msg.into())
}

fn def(sim: &Sim, kind: &'static str, id: &str) -> mlua::Result<crate::defs::DefId> {
    sim.world.defs.lookup(kind, id).ok_or_else(|| rt(format!("unknown {kind} '{id}'")))
}

impl World {
    fn touch(&self) {
        *self.last.borrow_mut() = Some((self.seed, self.sim.clone()));
    }

    fn run(&self, ticks: u64) {
        self.touch();
        let mut s = self.sim.borrow_mut();
        for _ in 0..ticks {
            s.step();
        }
    }
}

impl UserData for World {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("step", |_, w, n: Option<u64>| {
            w.run(n.unwrap_or(1));
            Ok(())
        });
        m.add_method("run_hours", |_, w, h: f64| {
            w.run((h.max(0.0) * TICKS_PER_DAY as f64 / 24.0).round() as u64);
            Ok(())
        });
        m.add_method("run_days", |_, w, d: f64| {
            w.run((d.max(0.0) * TICKS_PER_DAY as f64).round() as u64);
            Ok(())
        });
        m.add_method("tick", |_, w, ()| Ok(w.sim.borrow().world.tick));
        m.add_method("day", |_, w, ()| Ok(w.sim.borrow().world.day()));
        m.add_method("seed", |_, w, ()| Ok(w.seed));
        m.add_method("hash", |_, w, ()| Ok(format!("{:016x}", w.sim.borrow().world.state_hash())));
        m.add_method("colony_center", |_, w, ()| Ok(w.sim.borrow().world.colony_center().map(|c| (c.x, c.y)).unzip()));
        m.add_method("colonists", |lua, w, ()| {
            let s = w.sim.borrow();
            let t = lua.create_table()?;
            for e in s.world.colonists() {
                let Ok(p) = s.world.ecs.get::<&Pawn>(e) else { continue };
                let row = lua.create_table()?;
                row.set("id", e.to_bits().get())?;
                row.set("name", p.name.as_str())?;
                row.set("hp", p.hp)?;
                row.set("x", p.pos.x)?;
                row.set("y", p.pos.y)?;
                t.push(row)?;
            }
            Ok(t)
        });
        // The nearest cell to build on: passable, with nothing standing or
        // lying there. Rings outward in a fixed order, so it's the same
        // cell every run, and it draws on no random numbers.
        m.add_method("open_cell", |_, w, (x, y): (i32, i32)| {
            let s = w.sim.borrow();
            let map = &s.world.map;
            let open =
                |p: IVec| map.inb(p) && map.passable(p) && map.fixture_at(p).is_none() && map.item_at(p).is_none();
            let at = IVec::new(x, y);
            let found = (0..32i32).find_map(|r| {
                (-r..=r)
                    .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
                    .filter(|&(dx, dy)| dx.abs().max(dy.abs()) == r)
                    .map(|(dx, dy)| at.offset(dx, dy))
                    .find(|&p| open(p))
            });
            Ok(found.map(|p| (p.x, p.y)).unzip())
        });
        // Things of one def, built, lying about or in a hand, in entity-id
        // order: to find the station a test built, or the axe it made. A
        // held tool's x and y are where it was picked up.
        m.add_method("things", |lua, w, thing: String| {
            let s = w.sim.borrow();
            let d = def(&s, "thing", &thing)?;
            let mut found: Vec<(hecs::Entity, Thing)> = s
                .world
                .ecs
                .query::<(hecs::Entity, &Thing)>()
                .iter()
                .filter(|(_, t)| t.def == d)
                .map(|(e, t)| (e, t.clone()))
                .collect();
            found.sort_by_key(|(e, _)| e.id());
            let out = lua.create_table()?;
            for (e, t) in found {
                let row = lua.create_table()?;
                row.set("id", e.to_bits().get())?;
                row.set("x", t.pos.x)?;
                row.set("y", t.pos.y)?;
                row.set("count", t.count)?;
                row.set("hp", t.hp)?;
                row.set("blueprint", s.world.ecs.get::<&Blueprint>(e).is_ok())?;
                row.set("held", s.world.ecs.get::<&Held>(e).is_ok())?;
                row.set("made_of", s.world.made_of(e).map(|m| s.world.defs.thing(m).id.clone()))?;
                out.push(row)?;
            }
            Ok(out)
        });
        m.add_method("count_pawns", |_, w, faction: String| {
            let f = Faction::parse(&faction).ok_or_else(|| rt(format!("unknown faction '{faction}'")))?;
            let s = w.sim.borrow();
            let wd = &s.world;
            Ok(wd
                .pawns
                .iter()
                .filter(|&&e| wd.ecs.get::<&Pawn>(e).is_ok_and(|p| p.active && !p.dead && p.faction == f))
                .count())
        });
        m.add_method("spawn_pawn", |_, w, (creature, faction, x, y): (String, String, i32, i32)| {
            w.touch();
            let mut s = w.sim.borrow_mut();
            let d = def(&s, "creature", &creature)?;
            let f = Faction::parse(&faction).ok_or_else(|| rt(format!("unknown faction '{faction}'")))?;
            Ok(s.world.spawn_pawn(d, f, IVec::new(x, y), None).to_bits().get())
        });
        m.add_method("spawn_item", |_, w, (thing, x, y, count): (String, i32, i32, u32)| {
            w.touch();
            let mut s = w.sim.borrow_mut();
            let d = def(&s, "thing", &thing)?;
            Ok(s.world.place_item(d, IVec::new(x, y), count))
        });
        // The player's commands: applied at the next tick, as in a game.
        m.add_method("designate", |_, w, (des, x1, y1, x2, y2): (String, i32, i32, i32, i32)| {
            let mut s = w.sim.borrow_mut();
            let designation = def(&s, "designation", &des)?;
            s.push(Command::Designate { designation, a: IVec::new(x1, y1), b: IVec::new(x2, y2) });
            Ok(())
        });
        m.add_method(
            "build",
            |_, w, (thing, x1, y1, x2, y2, stuff): (String, i32, i32, Option<i32>, Option<i32>, Option<String>)| {
                let mut s = w.sim.borrow_mut();
                let thing = def(&s, "thing", &thing)?;
                let stuff = stuff.map(|id| def(&s, "thing", &id)).transpose()?;
                let (a, b) = (IVec::new(x1, y1), IVec::new(x2.unwrap_or(x1), y2.unwrap_or(y1)));
                s.push(Command::Build { thing, stuff, a, b });
                Ok(())
            },
        );
        m.add_method("send", |_, w, (name, data): (String, Value)| {
            let data = from_lua(&data, &name, 0).map_err(rt)?;
            w.sim.borrow_mut().push(Command::ModEvent { name, data });
            Ok(())
        });
        // Call what a mod's script exports, inside this world.
        m.add_method("call", |lua, w, (path, func, args): (String, String, MultiValue)| {
            w.touch();
            let args: Vec<_> = args
                .iter()
                .enumerate()
                .map(|(i, v)| from_lua(v, &format!("argument {}", i + 1), 0))
                .collect::<Result<_, _>>()
                .map_err(rt)?;
            let mut s = w.sim.borrow_mut();
            let s = &mut *s;
            let r = s.scripts.call_export(&mut s.world, &path, &func, &args).map_err(rt)?;
            match r {
                Some(d) => to_lua(lua, &d),
                None => Ok(Value::Nil),
            }
        });
        m.add_method("data", |lua, w, key: String| match w.sim.borrow().world.data.get(&key) {
            Some(d) => to_lua(lua, d),
            None => Ok(Value::Nil),
        });
        m.add_method("messages", |lua, w, ()| {
            let s = w.sim.borrow();
            let t = lua.create_table()?;
            for msg in &s.world.messages {
                let row = lua.create_table()?;
                row.set("text", msg.text.as_str())?;
                row.set("kind", format!("{:?}", msg.kind).to_lowercase())?;
                row.set("tick", msg.tick)?;
                t.push(row)?;
            }
            Ok(t)
        });
        m.add_method("ambient", |_, w, field: String| {
            let s = w.sim.borrow();
            Ok(s.world.fields.ambient(def(&s, "field", &field)? as usize))
        });
        m.add_method("field", |_, w, (field, x, y): (String, i32, i32)| {
            let s = w.sim.borrow();
            let f = def(&s, "field", &field)? as usize;
            let wd = &s.world;
            Ok(wd.fields.value(&wd.defs, &wd.map, f, IVec::new(x, y)))
        });
        m.add_method("warnings", |lua, w, ()| lua.create_sequence_from(w.sim.borrow().warnings.iter().cloned()));
    }
}

/// Every mod `id` needs, itself included.
fn closure(all: &[ModManifest], id: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut todo = vec![id.to_string()];
    while let Some(m) = todo.pop() {
        if out.insert(m.clone()) {
            if let Some(man) = all.iter().find(|x| x.id == m) {
                todo.extend(man.depends.iter().cloned());
            }
        }
    }
    out
}

/// What `rim check` found in a mod.
#[derive(Debug, Default)]
pub struct CheckReport {
    pub mod_id: String,
    /// The mods it loaded with: itself and everything it depends on.
    pub mods: Vec<String>,
    /// Load warnings: patch conflicts, skipped patches, determinism hazards.
    pub warnings: Vec<String>,
    /// Mods whose scripts ran over their time budget on this machine. Wall
    /// clock, so it says as much about the machine as the mod: reported,
    /// never a failure.
    pub slow: Vec<String>,
    /// Script errors from its first in-game hours.
    pub errors: Vec<String>,
}

/// `rim check`: load the mod at `mod_dir` with its dependencies on a small
/// map and run a few in-game hours, collecting load warnings and script
/// errors. A load failure is the `Err`.
pub fn check_mod(mod_dir: &Path, hours: f64) -> Result<CheckReport, String> {
    let mod_dir = mod_dir.canonicalize().map_err(|e| format!("{}: {e}", mod_dir.display()))?;
    let mods_dir = mod_dir.parent().ok_or("a mod folder has a parent")?.to_path_buf();
    let all = discover(&mods_dir)?;
    let me = all
        .iter()
        .find(|m| m.dir.canonicalize().ok().as_deref() == Some(mod_dir.as_path()))
        .ok_or_else(|| format!("{}: no mod.toml", mod_dir.display()))?;
    let mods = closure(&all, &me.id);
    let mut sim = Sim::build(&mods_dir, 1, &|id| mods.contains(id), 96)?;
    sim.scripts.echo_errors = false;
    let ticks = (hours * TICKS_PER_DAY as f64 / 24.0) as u64;
    // Anything warned while running is the time budget (Sim::step).
    let loaded = sim.warnings.len();
    for _ in 0..ticks {
        sim.step();
    }
    let slow = sim.warnings.split_off(loaded);
    let mut errors: Vec<String> = Vec::new();
    for m in sim.world.messages.iter().filter(|m| m.text.contains("script error")) {
        if !errors.contains(&m.text) {
            errors.push(m.text.clone());
        }
    }
    Ok(CheckReport {
        mod_id: me.id.clone(),
        mods: mods.into_iter().collect(),
        warnings: sim.warnings.clone(),
        slow,
        errors,
    })
}

/// Run every `tests/*.luau` in the mod at `mod_dir`. Worlds load mods from
/// the folder it sits in. `filter` keeps only tests whose name contains it.
pub fn run_mod(mod_dir: &Path, filter: Option<&str>) -> Result<Vec<TestResult>, String> {
    let mod_dir = mod_dir.canonicalize().map_err(|e| format!("{}: {e}", mod_dir.display()))?;
    let mods_dir: PathBuf = mod_dir.parent().ok_or("a mod folder has a parent")?.to_path_buf();
    let all = discover(&mods_dir)?;
    let me = all
        .iter()
        .find(|m| m.dir.canonicalize().ok().as_deref() == Some(mod_dir.as_path()))
        .ok_or_else(|| format!("{}: no mod.toml", mod_dir.display()))?
        .clone();
    let default_mods = closure(&all, &me.id);

    let tests_dir = mod_dir.join("tests");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&tests_dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "luau")).collect())
        .unwrap_or_default();
    files.sort();

    let mut results = Vec::new();
    for file in files {
        let rel = format!("tests/{}", file.file_name().unwrap().to_string_lossy());
        let fail_file = |msg: String| TestResult {
            mod_id: me.id.clone(),
            file: rel.clone(),
            name: "(loading the file)".into(),
            failure: Some(msg),
            seconds: 0.0,
        };
        let source = std::fs::read_to_string(&file).map_err(|e| format!("{}/{rel}: {e}", me.id))?;
        let lua = Lua::new();
        let last: Last = Rc::default();
        let tests: Rc<RefCell<Vec<(String, Function)>>> = Rc::default();
        let setup = || -> mlua::Result<()> {
            let g = lua.globals();
            let l = last.clone();
            g.set(
                "__rim_where",
                lua.create_function(move |_, ()| {
                    Ok(match &*l.borrow() {
                        Some((seed, sim)) => format!(" (world seed {seed}, tick {})", sim.borrow().world.tick),
                        None => String::new(),
                    })
                })?,
            )?;
            lua.load(PRELUDE).set_name("=rim test").exec()?;
            let t = tests.clone();
            g.set(
                "test",
                lua.create_function(move |_, (name, f): (String, Function)| {
                    t.borrow_mut().push((name, f));
                    Ok(())
                })?,
            )?;
            Ok(())
        };
        if let Err(e) = setup() {
            return Err(format!("rim test setup: {e}"));
        }
        if let Err(e) = lua.load(&source).set_name(format!("@{}/{rel}", me.id)).exec() {
            results.push(fail_file(e.to_string()));
            continue;
        }
        let tests = std::mem::take(&mut *tests.borrow_mut());
        for (name, f) in tests {
            if filter.is_some_and(|q| !name.contains(q)) {
                continue;
            }
            *last.borrow_mut() = None;
            let t = Instant::now();
            let r = (|| -> mlua::Result<()> {
                let tt: Table = lua.create_table()?;
                let (mods_dir, default_mods, last) = (mods_dir.clone(), default_mods.clone(), last.clone());
                tt.set(
                    "world",
                    lua.create_function(move |_, opts: Option<Table>| {
                        let seed: u64 = opts.as_ref().and_then(|o| o.get("seed").ok()).unwrap_or(1);
                        let size: i32 = opts.as_ref().and_then(|o| o.get("size").ok()).unwrap_or(crate::sim::MAP_SIZE);
                        let mods: BTreeSet<String> = match opts.as_ref().and_then(|o| o.get::<Vec<String>>("mods").ok())
                        {
                            Some(list) => list.into_iter().collect(),
                            None => default_mods.clone(),
                        };
                        let sim = Sim::build(&mods_dir, seed, &|id| mods.contains(id), size).map_err(rt)?;
                        let sim = Rc::new(RefCell::new(sim));
                        *last.borrow_mut() = Some((seed, sim.clone()));
                        Ok(World { sim, seed, last: last.clone() })
                    })?,
                )?;
                tt.set("expect", lua.globals().get::<Function>("expect")?)?;
                f.call::<()>(tt)
            })();
            results.push(TestResult {
                mod_id: me.id.clone(),
                file: rel.clone(),
                name,
                failure: r.err().map(|e| tidy(&e.to_string())),
                seconds: t.elapsed().as_secs_f64(),
            });
        }
    }
    Ok(results)
}

/// A failure without the runner's own frames: the message, then where in
/// the test files it happened.
fn tidy(err: &str) -> String {
    let err = err.strip_prefix("runtime error: ").unwrap_or(err);
    let Some((msg, trace)) = err.split_once("\nstack traceback:") else { return err.to_string() };
    let frames: Vec<&str> = trace.lines().map(str::trim).filter(|l| l.contains("/tests/")).collect();
    if frames.len() <= 1 {
        return msg.to_string();
    }
    format!("{msg}\n  {}", frames.join("\n  "))
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A JUnit-style report, which CI systems show as a test summary.
pub fn junit(results: &[TestResult]) -> String {
    let failures = results.iter().filter(|r| r.failure.is_some()).count();
    let total: f64 = results.iter().map(|r| r.seconds).sum();
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites tests=\"{}\" failures=\"{failures}\" time=\"{total:.3}\">\n",
        results.len()
    );
    let mut suites: Vec<&str> = results.iter().map(|r| r.mod_id.as_str()).collect();
    suites.dedup();
    for suite in suites {
        let rs: Vec<&TestResult> = results.iter().filter(|r| r.mod_id == suite).collect();
        let f = rs.iter().filter(|r| r.failure.is_some()).count();
        out.push_str(&format!("  <testsuite name=\"{}\" tests=\"{}\" failures=\"{f}\">\n", xml(suite), rs.len()));
        for r in rs {
            let class = format!("{}.{}", r.mod_id, r.file.trim_start_matches("tests/").trim_end_matches(".luau"));
            out.push_str(&format!(
                "    <testcase classname=\"{}\" name=\"{}\" time=\"{:.3}\"",
                xml(&class),
                xml(&r.name),
                r.seconds
            ));
            match &r.failure {
                None => out.push_str("/>\n"),
                Some(msg) => {
                    let first = msg.lines().next().unwrap_or_default();
                    out.push_str(&format!(
                        ">\n      <failure message=\"{}\">{}</failure>\n    </testcase>\n",
                        xml(first),
                        xml(msg)
                    ));
                }
            }
        }
        out.push_str("  </testsuite>\n");
    }
    out.push_str("</testsuites>\n");
    out
}
