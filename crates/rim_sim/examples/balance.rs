//! Balance harness: a simple bot plays the opening on many seeds and reports
//! how the first days go. The bot acts only through Commands, like a player.
//!
//!   cargo run --release -p rim_sim --example balance -- --seeds 20 --days 5
//!
//! The bot: designate trees and berry bushes near the start, then build a
//! 5x5 wooden hut (walls, a door, a bed inside). Colonists defend themselves.
//!
//! Flags: `--nohut`, `--no-orders` (core's standing orders switched off),
//! `--priorities auto|flat|tuned` (colonists on Auto, the default; all in
//! Hand at the work types' defaults; or Hand with a hand-written grid),
//! `--colonists N` (N colonists at the start, not one),
//! `--fire` (a campfire in the hut), `--cold-snap DAY`, `--core` (core alone,
//! no weather plugin), `--start-day N` (start on day N of the year, by adding
//! a patch mod to a copy of the mods folder), `--show SEED` (print that run's
//! messages), `--trench DAY` (that day the bot digs a ring of pits five
//! cells round the hut, with a drawbridge before the door, DESIGN.md §6d),
//! `--cellar DAY` (that day the bot digs stairs beside the hut and a 5x5
//! cellar at -1 under it, DESIGN.md §6d), `--tools` (one of every tool for each colonist, laid by the start: the
//! bot crafts none, and digging and chopping wait on them), `--raid DAY` (a
//! raid that day, fired through the storyteller, so runs with
//! and without a trench meet the same threat). Runs over more than one season
//! also report each season. `--bridge-work N` sets the work a bridge takes,
//! which is how long a raider spends on each cell. Seeds run in parallel.

use rim_sim::world::{Blueprint, MsgKind, Pawn, NEED_MAX};
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[derive(Default)]
struct Report {
    seed: u64,
    survived: bool,
    colonists_end: usize,
    deaths: Vec<(f64, String)>,
    min_hp: f64,
    min_food: f64,
    min_warmth: f64,
    /// Lowest warmth after the first night, once there's been time to build.
    min_warmth_later: f64,
    /// Colonist-hours spent at zero warmth (taking hypothermia damage).
    frozen_ticks: u64,
    /// The founder's hours at zero warmth after night one: what a one-bed hut is for.
    founder_frozen_later: u64,
    starving_ticks: u64,
    hut_done_day: Option<f64>,
    threats: Vec<(f64, String)>,
    goods: usize,
    wealth: f64,
    /// Colonist-hours at zero warmth, per season index.
    frozen_by_season: Vec<f64>,
    deaths_by_season: Vec<u32>,
    /// Colonists alive at the end of each season.
    alive_by_season: Vec<usize>,
    /// The founder's hours at zero warmth, per season, and whether they lived.
    founder_frozen_by_season: Vec<f64>,
    founder_alive: bool,
    /// Colonist-samples idle, and all colonist-samples, every 60 ticks.
    idle_samples: u64,
    samples: u64,
    /// Planned levels that changed, over the run.
    plan_changes: u64,
    /// Each death: (cause, the job they were on, cells from the colony's
    /// centre), from what the colonist looked like the sample before.
    causes: Vec<(String, String, i32)>,
    /// The day every cell of the trench was dug, with `--trench`.
    trench_done_day: Option<f64>,
    /// Of the trench's cells: dug by the end, and never diggable (rock,
    /// water, a tree or a building in the way when it was ordered).
    trench_dug: usize,
    trench_blocked: usize,
    /// The day the drawbridge before the door stood.
    drawbridge_day: Option<f64>,
    /// The day the cellar was dug out, with `--cellar`, and colonist-hours
    /// spent below the surface.
    cellar_day: Option<f64>,
    below_ticks: u64,
    /// A raider got inside the trench's ring (or within its radius, with
    /// no trench), with `--raid`.
    raiders_in: bool,
    /// Colonists who died on or after the raid's day.
    raid_deaths: u32,
    /// Bridges standing at the end: what raiders laid, less what was
    /// knocked down.
    bridges: usize,
}

/// Cells from the hut's middle to its trench.
const RING: i32 = 5;

/// What a colonist looked like at the last sample: what would explain a
/// death before the next one.
#[derive(Clone, Default)]
struct Last {
    job: String,
    dist: i32,
    attacker: Option<String>,
    starving: bool,
    freezing: bool,
}

/// How the run sets priorities: `auto` leaves colonists on Auto (the
/// default); `flat` puts them in Hand, every work type at its default;
/// `tuned` puts them in Hand with a grid a player might set for an opening.
fn set_priorities(s: &mut Sim) {
    let mode = std::env::args().skip_while(|a| a != "--priorities").nth(1).unwrap_or_else(|| "auto".into());
    if mode == "auto" {
        return;
    }
    let Some(hand) = s.world.work_roles.iter().position(|r| r.def.as_deref() == Some("core:hand")) else { return };
    let defs = s.world.defs.clone();
    let tuned = [
        ("core:build", 1),
        ("core:harvest", 2),
        ("core:chop", 2),
        ("core:hunt", 3),
        ("core:haul", 3),
        ("core:mine", 4),
    ];
    for pawn in s.world.colonists().collect::<Vec<_>>() {
        let p = s.world.ecs.get::<&Pawn>(pawn).unwrap();
        let (role, pinned) = (p.work_role, !p.priorities.is_empty());
        drop(p);
        if role != Some(hand as u16) {
            s.push(Command::AssignWorkRole { pawn, role: hand as u16 });
        }
        if mode == "tuned" && !pinned {
            for (id, level) in tuned {
                if let Some(work) = defs.lookup("work_type", id) {
                    s.push(Command::SetPriority { pawn, work, level });
                }
            }
        }
    }
}

fn open_square(s: &Sim, c: IVec, size: i32) -> Option<IVec> {
    let m = &s.world.map;
    let free = |p: IVec| m.passable(p) && m.fixture_at(p).is_none() && m.cost(p) <= 100;
    for r in 1..30i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let o = c.offset(dx, dy);
                if (0..size).all(|y| (0..size).all(|x| free(o.offset(x, y)))) {
                    return Some(o);
                }
            }
        }
    }
    None
}

fn play(mods: &Path, seed: u64, days: u64) -> Report {
    let core = std::env::args().any(|a| a == "--core");
    let mut s = Sim::with_mods(mods, seed, &|m| !core || m == "core").expect("mods load");
    // The colony switches off core's standing orders, as a player can.
    if std::env::args().any(|a| a == "--no-orders") {
        for id in ["core:food_low", "core:loose_items", "core:wood_for_winter"] {
            if let Some(rule) = s.world.defs.lookup("priority_rule", id) {
                s.push(Command::SetRuleEnabled { rule, on: false });
            }
        }
    }
    let seasons = s.world.defs.calendar.seasons.len();
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().unwrap();
    // --colonists N: a bigger start, beside the founder.
    let human = defs.creature_id("human").unwrap();
    for i in 1..arg("--colonists", 1) as i32 {
        let at = c.offset(i % 3, i / 3 + 1);
        s.world.spawn_pawn(human, rim_sim::world::Faction::Player, if s.world.map.passable(at) { at } else { c }, None);
    }
    set_priorities(&mut s);
    if std::env::args().any(|a| a == "--tools") {
        let n = s.world.colonists().count() as u32;
        for d in (0..defs.things.len()).filter(|&d| defs.things[d].tool.is_some()) {
            let stuff = defs.things[d]
                .build
                .as_ref()
                .and_then(|b| b.stuff.as_ref())
                .and_then(|sc| defs.materials(&sc.category).first().copied());
            s.world.place_item_of(d as rim_sim::defs::DefId, c, n, stuff);
        }
    }
    let mut plans: std::collections::BTreeMap<u64, Vec<(u16, u8)>> = std::collections::BTreeMap::new();
    let mut last: std::collections::BTreeMap<String, Last> = std::collections::BTreeMap::new();
    let des = |id: &str| defs.lookup("designation", id).unwrap();
    let thing = |id: &str| defs.thing_id(id).unwrap();
    let food = defs.lookup("need", "food").unwrap();
    let warmth = defs.lookup("need", "warmth");

    // Branches, which the stone age's campfire is built of. First, so
    // chop (one designation a thing) keeps the trees for wood.
    s.push(Command::Designate { designation: des("gather"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    s.push(Command::Designate { designation: des("chop"), a: c.offset(-14, -14), b: c.offset(14, 14) });
    s.push(Command::Designate { designation: des("harvest"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    let hut = if std::env::args().any(|a| a == "--nohut") { None } else { open_square(&s, c, 5) };
    if let Some(o) = hut {
        let (w, d, b) = (thing("wall"), thing("door"), thing("bed"));
        let wood = thing("wood");
        let door = o.offset(2, 4);
        for y in 0..5 {
            for x in 0..5 {
                let p = o.offset(x, y);
                let edge = x == 0 || y == 0 || x == 4 || y == 4;
                if edge && p != door {
                    s.push(Command::Build { stuff: Some(wood), thing: w, a: p, b: p, facing: 0 });
                }
            }
        }
        s.push(Command::Build { stuff: Some(wood), thing: d, a: door, b: door, facing: 0 });
        s.push(Command::Build { stuff: Some(wood), thing: b, a: o.offset(2, 2), b: o.offset(2, 2), facing: 0 });
        if std::env::args().any(|a| a == "--fire") {
            let f = thing("campfire");
            s.push(Command::Build { stuff: None, thing: f, a: o.offset(1, 1), b: o.offset(1, 1), facing: 0 });
        }
    }

    let mut r = Report {
        seed,
        min_hp: 1.0,
        min_food: 1.0,
        min_warmth: 1.0,
        min_warmth_later: 1.0,
        frozen_by_season: vec![0.0; seasons],
        deaths_by_season: vec![0; seasons],
        alive_by_season: vec![0; seasons],
        founder_frozen_by_season: vec![0.0; seasons],
        ..Default::default()
    };
    let centre = hut.map_or(c, |o| o.offset(2, 2));
    let ring: Vec<IVec> = (-RING..=RING)
        .flat_map(|y| (-RING..=RING).map(move |x| (x, y)))
        .filter(|(x, y)| x.abs().max(y.abs()) == RING)
        .map(|(x, y)| centre.offset(x, y))
        .collect();
    // The ring's cell before the door: the drawbridge goes over it.
    let gate = centre.offset(0, RING);
    let trench = std::env::args().any(|a| a == "--trench").then(|| arg("--trench", 3) * TICKS_PER_DAY);
    let cellar = std::env::args().any(|a| a == "--cellar").then(|| arg("--cellar", 1) * TICKS_PER_DAY);
    // Stairs by the hut, and the cellar dug round their foot once they stand.
    let stairs_at = (3..12)
        .flat_map(|r| [centre.offset(r, 0), centre.offset(-r, 0), centre.offset(0, r), centre.offset(0, -r)])
        .find(|&p| s.world.can_dig(p) && !ring.contains(&p));
    let mut cellar_marked = false;
    let raid = std::env::args().any(|a| a == "--raid").then(|| arg("--raid", 6) * TICKS_PER_DAY);
    let air = |s: &Sim, p: IVec| s.world.map.inb(p) && s.world.map.is_air(s.world.map.idx(p));
    let mut seen = 0;
    // --cold-snap DAY [--snap-drop C]: a cold snap on that day (C°C, default 8, for
    // two days), pushed directly so every run gets one.
    let snap = std::env::args().any(|a| a == "--cold-snap").then(|| arg("--cold-snap", 5) * TICKS_PER_DAY);
    for _ in 0..days * TICKS_PER_DAY {
        // Like a player, mark what has regrown to be gathered again.
        if s.world.tick > 0 && s.world.tick.is_multiple_of(TICKS_PER_DAY) {
            let c = s.world.colony_center().unwrap_or(IVec::new(0, 0));
            s.push(Command::Designate { designation: des("gather"), a: c.offset(-20, -20), b: c.offset(20, 20) });
        }
        if snap == Some(s.world.tick) {
            let t = defs.lookup("field", "temperature").unwrap() as usize;
            let now = s.world.tick;
            s.world.fields.push_ambient(t, "cold_snap", -(arg("--snap-drop", 8) as f64), now, Some(48.0), 3.0);
        }
        if let (Some(t), Some(top)) = (cellar, stairs_at) {
            if s.world.tick == t {
                s.push(Command::Build { stuff: None, thing: thing("stairs"), a: top, b: top, facing: 0 });
            }
            let down = s.world.map.portals().iter().any(|p| p.top == top);
            if down && !cellar_marked {
                cellar_marked = true;
                let foot = IVec::at(top.x, top.y, -1);
                s.push(Command::Designate {
                    designation: des("core:mine"),
                    a: foot.offset(-2, -2),
                    b: foot.offset(2, 2),
                });
            }
        }
        if trench == Some(s.world.tick) {
            r.trench_blocked = ring.iter().filter(|&&p| !s.world.can_dig(p)).count();
            for &p in &ring {
                s.push(Command::Build { stuff: None, thing: thing("pit"), a: p, b: p, facing: 0 });
            }
        }
        // Once the gate's pit is dug, the drawbridge over it.
        if trench.is_some_and(|t| s.world.tick > t && s.world.tick.is_multiple_of(TICKS_PER_DAY / 24))
            && air(&s, gate)
            && s.world.map.fixture_at(gate).is_none()
        {
            let wood = thing("wood");
            s.push(Command::Build { stuff: Some(wood), thing: thing("drawbridge"), a: gate, b: gate, facing: 0 });
        }
        if raid == Some(s.world.tick) {
            let rim_sim::Sim { scripts, world, .. } = &mut s;
            let fired = scripts.call_export(
                world,
                "@core/scripts/storyteller",
                "fire",
                &[Some(rim_sim::data::Data::Str("raid".into()))],
            );
            if let Err(e) = fired {
                eprintln!("seed {seed}: the raid didn't fire: {e}");
            }
        }
        // Newcomers get the run's priorities too.
        if s.world.tick.is_multiple_of(TICKS_PER_DAY / 24) {
            set_priorities(&mut s);
        }
        s.step();
        let w = &s.world;
        let day = w.tick as f64 / TICKS_PER_DAY as f64;
        if w.tick.is_multiple_of(60) {
            for e in w.colonists() {
                let p = w.ecs.get::<&Pawn>(e).unwrap();
                r.samples += 1;
                r.below_ticks += 60 * (p.pos.z < 0) as u64;
                r.idle_samples += matches!(p.job, rim_sim::world::Job::Idle) as u64;
                let job = format!("{:?}", p.job).split([' ', '{', '(']).next().unwrap_or("").to_string();
                let attacker =
                    p.last_attacker.and_then(|a| w.ecs.get::<&Pawn>(a).ok().map(|ap| defs.creature(ap.def).id.clone()));
                let home = w.colony_center().unwrap_or(p.pos);
                last.insert(
                    p.name.clone(),
                    Last {
                        job,
                        dist: p.pos.octile(home) as i32 / 10,
                        attacker,
                        starving: p.need(food) == Some(0),
                        freezing: warmth.and_then(|n| p.need(n)) == Some(0),
                    },
                );
                let now: Vec<(u16, u8)> = p.plan.iter().map(|x| (x.work, x.level)).collect();
                let before = plans.insert(e.to_bits().get(), now.clone()).unwrap_or_default();
                r.plan_changes += now.iter().filter(|x| before.iter().any(|b| b.0 == x.0 && b.1 != x.1)).count() as u64;
                let max = defs.creature(p.def).max_hp as f64;
                r.min_hp = r.min_hp.min(p.hp as f64 / max);
                if let Some(v) = warmth.and_then(|n| p.need(n)) {
                    r.min_warmth = r.min_warmth.min(v as f64 / NEED_MAX as f64);
                    if day >= 1.4 {
                        r.min_warmth_later = r.min_warmth_later.min(v as f64 / NEED_MAX as f64);
                    }
                    if v == 0 {
                        r.frozen_by_season[w.season_index() as usize] += 60.0 * 24.0 / TICKS_PER_DAY as f64;
                        r.frozen_ticks += 60;
                        if p.founder && day >= 1.4 {
                            r.founder_frozen_later += 60;
                        }
                        if p.founder {
                            r.founder_frozen_by_season[w.season_index() as usize] += 60.0 * 24.0 / TICKS_PER_DAY as f64;
                        }
                    }
                }
                if let Some(f) = p.need(food) {
                    r.min_food = r.min_food.min(f as f64 / NEED_MAX as f64);
                    if f == 0 {
                        r.starving_ticks += 60;
                    }
                }
            }
            if hut.is_some() && r.hut_done_day.is_none() && w.ecs.query::<&Blueprint>().iter().next().is_none() {
                r.hut_done_day = Some(day);
            }
            if let (true, None, Some(top)) = (cellar_marked, r.cellar_day, stairs_at) {
                let foot = IVec::at(top.x, top.y, -1);
                let open = (-2..=2).all(|y| (-2..=2).all(|x| w.solid_at(foot.offset(x, y)).is_none()));
                if open {
                    r.cellar_day = Some(day);
                }
            }
            let standing = w.map.fixture_at(gate).is_some_and(|f| w.ecs.get::<&Blueprint>(f).is_err());
            if trench.is_some() && r.drawbridge_day.is_none() && standing {
                r.drawbridge_day = Some(day);
            }
            if trench.is_some() && r.trench_done_day.is_none() && ring.iter().all(|&p| air(&s, p) || !w.map.inb(p)) {
                r.trench_done_day = Some(day);
            }
            if raid.is_some_and(|t| w.tick >= t) {
                let inside = w.pawns.iter().filter_map(|&e| w.ecs.get::<&Pawn>(e).ok()).any(|p| {
                    p.active
                        && !p.dead
                        && p.faction == rim_sim::world::Faction::Hostile
                        && p.pos.chebyshev(centre) < RING
                });
                r.raiders_in |= inside;
            }
        }
        for m in &w.messages[seen..] {
            if arg("--show", 0) == seed {
                println!("  d{day:.2} [{:?}] {}", m.kind, m.text);
            }
            match m.kind {
                MsgKind::Threat => r.threats.push((day, m.text.clone())),
                MsgKind::Bad if m.text.ends_with("has died.") => {
                    r.deaths.push((day, m.text.clone()));
                    let name = m.text.trim_end_matches(" has died.");
                    let l = last.get(name).cloned().unwrap_or_default();
                    let cause = match (&l.attacker, l.starving, l.freezing) {
                        (Some(a), _, _) => format!("attacked by {a}"),
                        (None, true, _) => "starved".to_string(),
                        (None, false, true) => "froze".to_string(),
                        _ => "other".to_string(),
                    };
                    r.causes.push((cause, l.job.clone(), l.dist));
                    r.deaths_by_season[w.season_index() as usize] += 1;
                    r.raid_deaths += raid.is_some_and(|t| w.tick >= t) as u32;
                }
                MsgKind::Good => r.goods += 1,
                _ => {}
            }
        }
        seen = w.messages.len();
        let si = w.season_index() as usize;
        r.alive_by_season[si] = w.colonists().count();
        if w.colony_lost {
            break;
        }
    }
    let w = &s.world;
    r.colonists_end = w.colonists().count();
    r.founder_alive = w.colonists().any(|e| w.ecs.get::<&Pawn>(e).is_ok_and(|p| p.founder));
    r.survived = r.colonists_end > 0;
    r.wealth = w.wealth;
    r.trench_dug = ring.iter().filter(|&&p| air(&s, p)).count();
    // --show SEED: why each colonist does what they do at the end.
    if arg("--show", 0) == seed {
        let w = &s.world;
        for e in w.colonists() {
            println!("  {}:", w.ecs.get::<&Pawn>(e).map(|p| p.name.clone()).unwrap_or_default());
            for why in rim_sim::ai::explain_work(w, e) {
                let label = &w.defs.work_types[why.work as usize].label;
                println!("    {label} (level {}): {}", why.level, rim_sim::order::why_text(w, &why.why));
            }
        }
    }
    let bridge = defs.thing_id("bridge");
    r.bridges = w.ecs.query::<&rim_sim::world::Thing>().iter().filter(|t| Some(t.def) == bridge).count();
    r
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

fn main() {
    let seeds = arg("--seeds", 20);
    let days = arg("--days", 5);
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    // A later start, or slower bridges, are a mod like any other: patches
    // to core, in a copy of the mods folder.
    let mut patches = Vec::new();
    if std::env::args().any(|a| a == "--start-day") {
        let day = arg("--start-day", 0);
        patches.push(format!("[[patch]]\ntarget = \"calendar/core:core\"\nset = {{ start_day = {day} }}\n"));
    }
    if std::env::args().any(|a| a == "--bridge-work") {
        let work = arg("--bridge-work", 120);
        patches.push(format!("[[patch]]\ntarget = \"thing/core:bridge\"\nset = {{ build = {{ work = {work} }} }}\n"));
    }
    let mods = match patches.is_empty() {
        false => {
            let dir = std::env::temp_dir().join(format!("rim-balance-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            copy_dir(&shipped, &dir);
            let m = dir.join("balance_start");
            std::fs::create_dir_all(m.join("defs")).unwrap();
            std::fs::write(
                m.join("mod.toml"),
                "id = \"balance_start\"\nname = \"Balance start\"\nversion = \"0.1.0\"\napi = \"0.6\"\ndepends = [\"core\"]\n",
            )
            .unwrap();
            std::fs::write(m.join("defs/start.toml"), patches.concat()).unwrap();
            dir
        }
        true => shipped,
    };
    // Seeds in parallel: each run builds its own Sim on its own thread.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()) as u64;
    let mut reports: Vec<Report> = std::thread::scope(|sc| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let mods = &mods;
                sc.spawn(move || {
                    (1..=seeds).filter(|s| s % threads == t).map(|seed| play(mods, seed, days)).collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    reports.sort_by_key(|r| r.seed);

    println!("seed  alive  end  min_hp  min_food  min_warm  starved_h  hut_day  threats  good  wealth  deaths");
    for r in &reports {
        let starved_h = r.starving_ticks as f64 / TICKS_PER_DAY as f64 * 24.0;
        let hut = r.hut_done_day.map_or("-".into(), |d| format!("{d:.2}"));
        let deaths: Vec<String> = r.deaths.iter().map(|(d, t)| format!("d{d:.1} {t}")).collect();
        println!(
            "{:>4}  {:>5}  {:>3}  {:>6.2}  {:>8.2}  {:>8.2}  {:>9.1}  {:>7}  {:>7}  {:>4}  {:>6.0}  {}",
            r.seed,
            if r.survived { "yes" } else { "NO" },
            r.colonists_end,
            r.min_hp,
            r.min_food,
            r.min_warmth,
            starved_h,
            hut,
            r.threats.len(),
            r.goods,
            r.wealth,
            deaths.join("; ")
        );
    }
    let n = reports.len() as f64;
    let lost = reports.iter().filter(|r| !r.survived).count();
    let any_death = reports.iter().filter(|r| !r.deaths.is_empty()).count();
    let near_miss = reports.iter().filter(|r| r.survived && r.min_hp < 0.35).count();
    let hungry = reports.iter().filter(|r| r.min_food < 0.1).count();
    let no_hut = reports.iter().filter(|r| r.hut_done_day.is_none_or(|d| d > 1.0)).count();
    let first_threat: Vec<f64> = reports.iter().filter_map(|r| r.threats.first().map(|t| t.0)).collect();
    println!();
    println!("colonies lost:            {lost}/{n}");
    println!("runs with a death:        {any_death}/{n}");
    // Samples are every 60 ticks: a colonist-day is TICKS_PER_DAY / 60 of them.
    let per_day = TICKS_PER_DAY as f64 / 60.0;
    let samples = reports.iter().map(|r| r.samples).sum::<u64>().max(1) as f64;
    let idle = reports.iter().map(|r| r.idle_samples).sum::<u64>() as f64 / samples * 24.0;
    let churn = reports.iter().map(|r| r.plan_changes).sum::<u64>() as f64 / (samples / per_day);
    println!("idle, hours a colonist-day: {idle:.1}");
    println!("planned changes a colonist-day: {churn:.2}");
    // Deaths by cause, and the jobs and distances they happened at.
    let mut by_cause: std::collections::BTreeMap<&str, (u32, i64)> = std::collections::BTreeMap::new();
    let mut by_job: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
    for (cause, job, dist) in reports.iter().flat_map(|r| r.causes.iter()) {
        let e = by_cause.entry(cause.as_str()).or_default();
        e.0 += 1;
        e.1 += *dist as i64;
        *by_job.entry(job.as_str()).or_default() += 1;
    }
    let causes: Vec<String> =
        by_cause.iter().map(|(c, (n, d))| format!("{c} {n} (mean {} cells out)", d / (*n).max(1) as i64)).collect();
    println!("deaths by cause:          {}", causes.join(", "));
    let jobs: Vec<String> = by_job.iter().map(|(j, n)| format!("{j} {n}")).collect();
    println!("deaths by job:            {}", jobs.join(", "));
    println!("near-misses (hp < 35%):   {near_miss}/{n}");
    println!("food ever below 10%:      {hungry}/{n}");
    let cold = reports.iter().filter(|r| r.min_warmth < 0.1).count();
    println!("warmth ever below 10%:    {cold}/{n}");
    let later = reports.iter().map(|r| r.min_warmth_later).sum::<f64>() / n;
    let frozen = reports.iter().filter(|r| r.frozen_ticks > 0).count();
    let frozen_h = reports.iter().map(|r| r.frozen_ticks).sum::<u64>() as f64 / TICKS_PER_DAY as f64 * 24.0 / n;
    println!("warmth after night one:   mean low {later:.2}");
    println!("froze (warmth hit 0):     {frozen}/{n} runs, {frozen_h:.1} colonist-hours per run");
    let ff = reports.iter().filter(|r| r.founder_frozen_later > 0).count();
    let ff_h = reports.iter().map(|r| r.founder_frozen_later).sum::<u64>() as f64 / TICKS_PER_DAY as f64 * 24.0 / n;
    println!("founder froze after n1:   {ff}/{n} runs, {ff_h:.1} hours per run");
    println!("hut not done by day 1:    {no_hut}/{n}");
    if !first_threat.is_empty() {
        let mean = first_threat.iter().sum::<f64>() / first_threat.len() as f64;
        let min = first_threat.iter().cloned().fold(f64::MAX, f64::min);
        println!("first threat day:         mean {mean:.2}, earliest {min:.2} ({} runs had one)", first_threat.len());
    }
    // Per season, over runs that reached it.
    if days > 15 {
        let names = {
            let core = std::env::args().any(|a| a == "--core");
            let s = Sim::with_mods(&mods, 1, &|m| !core || m == "core").expect("mods load");
            s.world.defs.calendar.seasons.clone()
        };
        println!("\nseason    frozen colonist-h/run  founder frozen h/run  deaths  colonies alive at end");
        for (i, name) in names.iter().enumerate() {
            let frozen = reports.iter().map(|r| r.frozen_by_season[i]).sum::<f64>() / n;
            let founder = reports.iter().map(|r| r.founder_frozen_by_season[i]).sum::<f64>() / n;
            let deaths: u32 = reports.iter().map(|r| r.deaths_by_season[i]).sum();
            let alive = reports.iter().filter(|r| r.alive_by_season[i] > 0).count();
            println!("{name:<9} {frozen:>22.1}  {founder:>20.1}  {deaths:>6}  {alive:>6}/{n}");
        }
        let founders = reports.iter().filter(|r| r.founder_alive).count();
        println!("founder alive at the end: {founders}/{n}");
    }
    if std::env::args().any(|a| a == "--raid") {
        let inside = reports.iter().filter(|r| r.raiders_in).count();
        let deaths: u32 = reports.iter().map(|r| r.raid_deaths).sum();
        let bridges = reports.iter().map(|r| r.bridges).sum::<usize>() as f64 / n;
        println!("raiders got inside:       {inside}/{n}");
        println!("deaths from the raid day: {deaths} over {n} runs");
        println!("bridges standing:         {bridges:.1} a run");
    }
    if std::env::args().any(|a| a == "--cellar") {
        let dug: Vec<f64> = reports.iter().filter_map(|r| r.cellar_day).collect();
        let mean = dug.iter().sum::<f64>() / dug.len().max(1) as f64;
        let below = reports.iter().map(|r| r.below_ticks).sum::<u64>() as f64 / TICKS_PER_DAY as f64 * 24.0 / n;
        println!(
            "cellar dug:               {}/{n} runs, mean day {mean:.2}; {below:.1} colonist-hours below a run",
            dug.len()
        );
    }
    if std::env::args().any(|a| a == "--trench") {
        let dug: Vec<f64> = reports.iter().filter_map(|r| r.trench_done_day).collect();
        let mean = dug.iter().sum::<f64>() / dug.len().max(1) as f64;
        println!("trench dug:               {}/{n} runs, mean day {mean:.2}", dug.len());
        let whole: Vec<&Report> = reports.iter().filter(|r| r.trench_done_day.is_some()).collect();
        let inside = whole.iter().filter(|r| r.raiders_in).count();
        let deaths: u32 = whole.iter().map(|r| r.raid_deaths).sum();
        println!(
            "  with it whole:          raiders inside {inside}/{}, {deaths} deaths from the raid day",
            whole.len()
        );
        let draw = reports.iter().filter(|r| r.drawbridge_day.is_some()).count();
        println!("drawbridge stood:         {draw}/{n} runs");
        let cells = reports.iter().map(|r| r.trench_dug).sum::<usize>() as f64 / n;
        let blocked = reports.iter().map(|r| r.trench_blocked).sum::<usize>() as f64 / n;
        println!("trench cells a run:       {cells:.1} dug, {blocked:.1} not diggable when ordered, of {}", 8 * RING);
    }
    let threat_kinds: Vec<&str> = reports.iter().flat_map(|r| r.threats.iter().map(|t| t.1.as_str())).collect();
    for t in threat_kinds.iter().take(12) {
        println!("  {t}");
    }
}
