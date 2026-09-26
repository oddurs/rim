//! The stone age's first days, measured (4bd94457): a bot plays a lone
//! naked colonist's opening with core, crafting, primitive and weather on,
//! and reports when each step of the climb lands, on many seeds.
//!
//!   cargo run --release -p rim_sim --example stone_age -- --seeds 20 --days 5
//!
//! The bot acts only through Commands, like a player:
//! - at once: put building first and crafting second; gather and forage
//!   around the start, mark the nearest flint and stones; plan a small
//!   branch hut (3x3 by default, `--hut N`) with a door and a grass pallet, on
//!   ground cleared by hand, with a campfire by it and a crafting spot
//!   outside;
//! - once the spot stands: bills for a hammerstone, a hand axe and a
//!   digging stick;
//! - with an axe: fell the trees around, felling second only to building;
//! - with a digging stick: dig the nearest clay, and plan a row of cob
//!   walls beside the hut.
//!
//! Each morning it marks what has regrown to be gathered again. `--show
//! SEED` prints that run's messages. `--defaults` leaves every priority at
//! its default, to measure what the colonist does unprompted.

use rim_sim::data::{Data, Key};
use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, MadeOf, Thing};
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// When each step first happened, in days from the start.
#[derive(Default)]
struct Report {
    seed: u64,
    campfire: Option<f64>,
    /// A bed in an enclosed room: a shelter to sleep in.
    shelter: Option<f64>,
    flint: Option<f64>,
    edge: Option<f64>,
    felled: Option<f64>,
    clay: Option<f64>,
    cob: Option<f64>,
    died: Option<f64>,
    /// At nightfall on day one: branches gathered so far (lying or built
    /// in), and how many of the hut's plans stand.
    night_branches: u32,
    night_built: usize,
    night_plans: usize,
    messages: Vec<String>,
}

/// The first night, for the targets: 20:00 on day one, 14 hours in. The
/// engine's night starts an hour later, so this is the stricter reading.
const NIGHT: f64 = 14.0 / 24.0;

fn count(s: &Sim, id: &str) -> u32 {
    let Some(d) = s.world.defs.thing_id(id) else { return 0 };
    s.world.ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|t| t.def == d).map(|t| t.count).sum()
}

/// A square of open ground near `c` for the hut, with a margin. Grass and
/// stones can be cleared by hand; a tree or rock can't, so the hut avoids
/// them, as a player would.
fn open_square(s: &Sim, c: IVec, size: i32) -> Option<IVec> {
    let m = &s.world.map;
    let by_hand = |p: IVec| {
        m.fixture_at(p).and_then(|f| s.world.thing(f)).is_none_or(|t| {
            let td = s.world.defs.thing(t.def);
            td.natural && td.harvest.iter().any(|h| h.destroy && h.requires.is_empty())
        })
    };
    let free = |p: IVec| m.passable(p) && m.cost(p) <= 200 && by_hand(p);
    (2..30i32).find_map(|r| {
        (-r..=r)
            .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
            .filter(|&(dx, dy)| dx.abs().max(dy.abs()) == r)
            .map(|(dx, dy)| c.offset(dx, dy))
            .find(|&o| (-1..=size).all(|x| (-1..=size + 2).all(|y| free(o.offset(x, y)))))
    })
}

/// The things of a def nearest `c`, nearest first, by (distance, id).
fn nearest(s: &Sim, id: &str, c: IVec, n: usize) -> Vec<IVec> {
    let Some(d) = s.world.defs.thing_id(id) else { return Vec::new() };
    let mut v: Vec<(i32, u32, IVec)> = s
        .world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .filter(|(_, t)| t.def == d)
        .map(|(e, t)| ((t.pos.x - c.x).abs().max((t.pos.y - c.y).abs()), e.id(), t.pos))
        .collect();
    v.sort();
    v.into_iter().take(n).map(|x| x.2).collect()
}

fn add_bill(s: &mut Sim, site: Entity, recipe: &str) {
    let data = [
        (Key::Str("site".into()), Data::Int(site.to_bits().get() as i64)),
        (Key::Str("recipe".into()), Data::Str(recipe.into())),
    ];
    s.push(Command::ModEvent { name: "crafting:add_bill".into(), data: Some(Data::Table(data.into_iter().collect())) });
}

fn run(mods: &Path, seed: u64, days: u64, hut: i32) -> Report {
    let wanted = ["core", "crafting", "primitive", "weather"];
    let mut s = Sim::with_mods(mods, seed, &|m| wanted.contains(&m)).expect("mods load");
    let mut r = Report { seed, ..Default::default() };
    let defs = s.world.defs.clone();
    let des = |id: &str| defs.lookup("designation", id).expect(id);
    let thing = |id: &str| defs.thing_id(id).expect(id);
    let c = s.world.colony_center().expect("a colony");
    let branches = Some(thing("primitive:branches"));
    let clay = Some(thing("primitive:clay"));

    // Shelter first: at equal priorities the nearest work wins, and some
    // grass is always nearer than the hut's materials.
    // Tools next: bills only run once there's what they need.
    let founder = s.world.colonists().next();
    let priority = |s: &mut Sim, work: &str, level: u8| {
        if let (Some(pawn), Some(work)) = (founder, defs.lookup("work_type", work)) {
            s.push(Command::SetPriority { pawn, work, level });
        }
    };
    let defaults = std::env::args().any(|a| a == "--defaults");
    if !defaults {
        priority(&mut s, "core:build", 1);
        priority(&mut s, "crafting:craft", 2);
    }
    s.push(Command::Designate { designation: des("core:gather"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    s.push(Command::Designate { designation: des("core:harvest"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    for p in nearest(&s, "primitive:flint_nodule", c, 3).into_iter().chain(nearest(&s, "primitive:loose_stones", c, 2))
    {
        s.push(Command::Designate { designation: des("core:gather"), a: p, b: p });
    }
    let Some(o) = open_square(&s, c, hut) else { return r };
    let plans = |s: &Sim| {
        let (mut built, mut all) = (0, 0);
        for y in 0..hut {
            for x in 0..hut {
                if let Some(f) = s.world.map.fixture_at(o.offset(x, y)) {
                    // Grass still waiting under a plan isn't built.
                    let waiting = s.world.ecs.get::<&Blueprint>(f).is_ok()
                        || s.world.ecs.get::<&rim_sim::world::Planned>(f).is_ok();
                    all += 1;
                    built += usize::from(!waiting);
                }
            }
        }
        (built, all)
    };
    let at = |dx: i32, dy: i32| o.offset(dx, dy);
    for y in 0..hut {
        for x in 0..hut {
            let edge = x == 0 || y == 0 || x == hut - 1 || y == hut - 1;
            if edge && (x, y) != (2, hut - 1) {
                s.push(Command::Build { thing: thing("core:wall"), stuff: branches, a: at(x, y), b: at(x, y) });
            }
        }
    }
    s.push(Command::Build { thing: thing("core:door"), stuff: branches, a: at(2, hut - 1), b: at(2, hut - 1) });
    s.push(Command::Build { thing: thing("primitive:pallet"), stuff: None, a: at(1, 1), b: at(1, 1) });
    // The fire inside a hut with room for it, else by the door.
    let fire = if hut >= 5 { at(3, 2) } else { at(2, hut) };
    s.push(Command::Build { thing: thing("core:campfire"), stuff: None, a: fire, b: fire });
    let spot_at = at(2, hut + 1);
    s.push(Command::Build { thing: thing("crafting:spot"), stuff: None, a: spot_at, b: spot_at });

    let (campfire, wall) = (thing("core:campfire"), thing("core:wall"));
    let (billed, chopping, digging) = (&mut false, &mut false, &mut false);
    let now = |s: &Sim| s.world.tick as f64 / TICKS_PER_DAY as f64;
    let every = TICKS_PER_DAY / 48;
    let mut tick = 0;
    let mut marked: Vec<Entity> = Vec::new();
    while tick < days * TICKS_PER_DAY {
        // Each morning, gather again what has grown back, and keep the
        // trees marked to fell once there's an axe (gather would take the
        // mark off an oak: it has both).
        if tick > 0 && tick % TICKS_PER_DAY == 0 {
            s.push(Command::Designate { designation: des("core:gather"), a: c.offset(-20, -20), b: c.offset(20, 20) });
            if *chopping {
                s.push(Command::Designate {
                    designation: des("core:chop"),
                    a: c.offset(-14, -14),
                    b: c.offset(14, 14),
                });
            }
        }
        if tick == (NIGHT * TICKS_PER_DAY as f64) as u64 {
            (r.night_built, r.night_plans) = plans(&s);
            let br = thing("primitive:branches");
            let delivered: u32 = s
                .world
                .ecs
                .query::<&Blueprint>()
                .iter()
                .flat_map(|b| {
                    b.cost.iter().zip(&b.delivered).filter(|(c, _)| c.0 == br).map(|(_, d)| *d).collect::<Vec<_>>()
                })
                .sum();
            let built: u32 = s
                .world
                .ecs
                .query::<(&Thing, &MadeOf)>()
                .without::<&Blueprint>()
                .iter()
                .filter(|(_, m)| m.0 == br)
                .map(|(t, _)| defs.thing(t.def).build.as_ref().and_then(|b| b.stuff.as_ref()).map_or(0, |sc| sc.count))
                .sum();
            r.night_branches = count(&s, "primitive:branches") + delivered + built;
        }
        if tick % every == 0 {
            let w = &s.world;
            let t = now(&s);
            let built = |d| w.ecs.query::<&Thing>().without::<&Blueprint>().iter().any(|x| x.def == d);
            if r.campfire.is_none() && built(campfire) {
                r.campfire = Some(t);
            }
            if r.shelter.is_none() {
                let beds: Vec<IVec> = w
                    .ecs
                    .query::<&Thing>()
                    .without::<&Blueprint>()
                    .iter()
                    .filter(|x| defs.thing(x.def).bed.is_some())
                    .map(|x| x.pos)
                    .collect();
                if beds.iter().any(|&p| w.map.room_at(p).is_some_and(|room| room.enclosed())) {
                    r.shelter = Some(t);
                }
            }
            let any = |id: &str| count(&s, id) > 0;
            let held = |id: &str| {
                let d = thing(id);
                w.ecs.query::<&Thing>().iter().any(|x| x.def == d)
            };
            if r.flint.is_none() && (any("primitive:flint") || held("primitive:hand_axe")) {
                r.flint = Some(t);
            }
            if r.edge.is_none() && (held("primitive:flake") || held("primitive:hand_axe")) {
                r.edge = Some(t);
            }
            // One of the oaks it marked to fell is down. (Not "there's
            // wood": a windfall drops some, and trees spread, so neither
            // wood nor the count of oaks says a tree was felled.)
            if r.felled.is_none() && marked.iter().any(|&e| w.thing(e).is_none()) {
                r.felled = Some(t);
            }
            if r.clay.is_none() && any("primitive:clay") {
                r.clay = Some(t);
            }
            if r.cob.is_none() {
                let cob = w
                    .ecs
                    .query::<(&Thing, &MadeOf)>()
                    .without::<&Blueprint>()
                    .iter()
                    .any(|(x, m)| x.def == wall && Some(m.0) == clay);
                if cob {
                    r.cob = Some(t);
                }
            }
            if r.died.is_none() && w.colonists().next().is_none() {
                r.died = Some(t);
            }
            // The player's next moves.
            let spot = w.map.fixture_at(spot_at).filter(|&e| {
                w.thing(e).is_some_and(|x| x.def == thing("crafting:spot")) && w.ecs.get::<&Blueprint>(e).is_err()
            });
            let axe = held("primitive:hand_axe") || held("primitive:hafted_axe");
            let stick = held("primitive:digging_stick");
            if let (Some(site), false) = (spot, *billed) {
                *billed = true;
                for recipe in ["primitive:hammerstone", "primitive:hand_axe", "primitive:digging_stick"] {
                    add_bill(&mut s, site, recipe);
                }
            }
            if axe && !*chopping {
                *chopping = true;
                if !defaults {
                    priority(&mut s, "core:chop", 2);
                }
                let oak = thing("core:tree_oak");
                marked = s
                    .world
                    .ecs
                    .query::<(Entity, &Thing)>()
                    .iter()
                    .filter(|(_, x)| x.def == oak && (x.pos.x - c.x).abs() <= 14 && (x.pos.y - c.y).abs() <= 14)
                    .map(|(e, _)| e)
                    .collect();
                s.push(Command::Designate {
                    designation: des("core:chop"),
                    a: c.offset(-14, -14),
                    b: c.offset(14, 14),
                });
            }
            if stick && !*digging {
                *digging = true;
                for p in nearest(&s, "primitive:clay_bank", c, 4) {
                    s.push(Command::Designate { designation: des("core:gather"), a: p, b: p });
                }
                // A cob windbreak along the hut's east side.
                s.push(Command::Build { thing: wall, stuff: clay, a: at(hut, 0), b: at(hut, hut - 1) });
            }
        }
        s.step();
        tick += 1;
    }
    r.messages =
        s.world.messages.iter().map(|m| format!("{:>6.2}  {}", m.tick as f64 / TICKS_PER_DAY as f64, m.text)).collect();
    r
}

fn main() {
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let (seeds, days, show) = (arg("--seeds", 20), arg("--days", 5), arg("--show", 0));
    let hut = arg("--hut", 3) as i32;
    let reports: Vec<Report> = std::thread::scope(|sc| {
        let handles: Vec<_> = (1..=seeds)
            .map(|seed| {
                sc.spawn({
                    let mods = mods.clone();
                    move || run(&mods, seed, days, hut)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("a run")).collect()
    });
    let d = |x: Option<f64>| x.map_or("   -  ".to_string(), |v| format!("{v:>6.2}"));
    println!("seed  campfire shelter  flint   edge  felled   clay    cob   died   at nightfall");
    for r in &reports {
        println!(
            "{:>4}  {}  {}  {} {} {}  {} {} {}   {} branches, {}/{} built",
            r.seed,
            d(r.campfire),
            d(r.shelter),
            d(r.flint),
            d(r.edge),
            d(r.felled),
            d(r.clay),
            d(r.cob),
            d(r.died),
            r.night_branches,
            r.night_built,
            r.night_plans
        );
        if r.seed == show {
            for m in &r.messages {
                println!("        {m}");
            }
        }
    }
    // The targets (4bd94457), as shares of seeds.
    let n = reports.len() as f64;
    let share = |f: &dyn Fn(&Report) -> bool| 100.0 * reports.iter().filter(|r| f(r)).count() as f64 / n;
    let night = NIGHT;
    let targets = [
        (
            "campfire and an enclosed bed before the first night",
            share(&|r| r.campfire.is_some_and(|t| t < night) && r.shelter.is_some_and(|t| t < night)),
            90.0,
        ),
        ("a flint tool by the end of day 2", share(&|r| r.edge.is_some_and(|t| t < 2.0)), 80.0),
        ("a felled tree by the end of day 3", share(&|r| r.felled.is_some_and(|t| t < 3.0)), 80.0),
        ("cob walls by the end of day 4", share(&|r| r.cob.is_some_and(|t| t < 4.0)), 60.0),
    ];
    println!();
    for (what, got, want) in targets {
        println!("{} {what}: {got:.0}% (target {want:.0}%)", if got >= want { "ok  " } else { "MISS" });
    }
}
