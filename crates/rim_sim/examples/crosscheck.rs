//! The cross-machine determinism scenario (0157): every platform in CI runs
//! this and must print exactly the same lines.
//!
//!   cargo run --release -p rim_sim --example crosscheck [-- --days N]
//!
//! All shipped mods, a fixed seed, and a player's commands (gather, forage,
//! a campfire, a crafting spot with bills for a hammerstone and a hand axe,
//! then chop and a wooden hut with a door and a bed), over one calendar year
//! by default. The colony lives the whole year, so the hash covers its work
//! orders, building, needs and newcomers through every season's weather and
//! growth, and fails if it doesn't. The storyteller rolls its dice all year,
//! with threats off: an undefended colony falls to a stampede or a pack
//! within weeks (224a5488), and a dead colony leaves only wildlife and
//! weather to hash. One raid on day RAID_DAY and one boar stampede on day
//! STAMPEDE_DAY keep combat in it: the stampede once killed this colony, all
//! nine, in two hours (6ca1ea2e), and must leave it standing. Prints the
//! state hash at the end of each day, so when two platforms disagree the
//! output shows the first day they diverged.
//!
//! A twin of the game is saved and loaded every few days (DESIGN.md §7a),
//! and must match the one that never saved, section for section, every day.
//!
//! `--trace-day D` also prints every section's hash after each tick of day D
//! (`tick T section=hash ...`): when platforms disagree on a day, CI runs it
//! for that day on each and names the first tick and sections that differ.

use rim_sim::data::{Data, Key};
use rim_sim::snapshot::Snapshot;
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

const STORYTELLER: &str = "@core/scripts/storyteller";
/// The day the one raid comes, at dawn: past the stone-age opening, with the
/// hut up and a few colonists to meet it.
const RAID_DAY: u64 = 10;
/// The day wildlife_plus's boars stampede, at dawn: when, with threats on,
/// they once wiped the colony out.
const STAMPEDE_DAY: u64 = 18;

/// Call core's storyteller, as the harness's hand on the dice.
fn storyteller(s: &mut Sim, export: &str, args: &[Option<Data>]) {
    let Sim { scripts, world, .. } = s;
    if let Err(e) = scripts.call_export(world, STORYTELLER, export, args) {
        eprintln!("the storyteller's {export} failed: {e}");
        std::process::exit(1);
    }
}

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    // Seed 4: its colony makes its tools and hut by day 5 and lives the
    // year. The seed follows the map, and a change to worldgen can move it:
    // the checks below say so rather than hash less than they claim.
    let seed = arg("--seed", 4);
    let trace_day = arg("--trace-day", 0);
    let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let mut s = Sim::new(&mods, seed).expect("mods load");
    let days = arg("--days", s.world.defs.calendar.year_days as u64);
    let defs = s.world.defs.clone();
    let des = |id: &str| defs.lookup("designation", id).expect(id);
    let thing = |id: &str| defs.thing_id(id).expect(id);
    let wood = Some(thing("wood"));
    let c = s.world.colony_center().expect("a colony");

    // The stone age, played: gather branches, stones and flint by hand, a
    // campfire of branches, and a crafting spot for the first tools. The
    // rest waits for them (below).
    s.push(Command::Designate { designation: des("gather"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    s.push(Command::Designate { designation: des("harvest"), a: c.offset(-20, -20), b: c.offset(20, 20) });
    // Flint is scarce and rarely close: like a player who spots it, mark
    // the nearest few nodules wherever they are.
    let nodule = thing("primitive:flint_nodule");
    let mut flint: Vec<(i32, u32, IVec)> = s
        .world
        .ecs
        .query::<(rim_sim::hecs::Entity, &rim_sim::world::Thing)>()
        .iter()
        .filter(|(_, t)| t.def == nodule)
        .map(|(e, t)| ((t.pos.x - c.x).abs().max((t.pos.y - c.y).abs()), e.id(), t.pos))
        .collect();
    flint.sort();
    for &(_, _, p) in flint.iter().take(3) {
        s.push(Command::Designate { designation: des("gather"), a: p, b: p });
    }
    let o = c.offset(3, 3);
    let at = |dx: i32, dy: i32| -> IVec { o.offset(dx, dy) };
    s.push(Command::Build { thing: thing("campfire"), stuff: None, a: at(1, 1), b: at(1, 1), facing: 0 });
    // Outside the door, on the nearest cell with nothing growing on it and
    // clear of the hut to come.
    let hut_cell = |p: IVec| (0..5).contains(&(p.x - o.x)) && (0..5).contains(&(p.y - o.y));
    let open = |p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && !hut_cell(p);
    let spot_at = (0..16)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy))).map(|(dx, dy)| at(2 + dx, 6 + dy)))
        .find(|&p| open(p))
        .expect("open ground for a crafting spot");
    s.push(Command::Build { thing: thing("crafting:spot"), stuff: None, a: spot_at, b: spot_at, facing: 0 });
    // With an axe: fell the trees, and raise a 5x5 wooden hut around the
    // fire, with a door on the south side and a bed.
    let hut = |g: &mut Sim| {
        g.push(Command::Designate { designation: des("chop"), a: c.offset(-14, -14), b: c.offset(14, 14) });
        for y in 0..5 {
            for x in 0..5 {
                let p = o.offset(x, y);
                let edge = x == 0 || y == 0 || x == 4 || y == 4;
                if edge && p != o.offset(2, 4) {
                    g.push(Command::Build { thing: thing("wall"), stuff: wood, a: p, b: p, facing: 0 });
                }
            }
        }
        g.push(Command::Build { thing: thing("door"), stuff: wood, a: at(2, 4), b: at(2, 4), facing: 0 });
        g.push(Command::Build { thing: thing("bed"), stuff: wood, a: at(2, 2), b: at(2, 2), facing: 0 });
    };

    storyteller(&mut s, "set_threats", &[Some(Data::Bool(false))]);
    // The commands apply on the first tick; the twin splits off after it,
    // and the storyteller's memory, threats off, goes with it.
    s.step();
    let mut twin = Snapshot::capture(&s).restore(&mods, &|_| true).expect("the twin loads");

    println!(
        "crosscheck seed {seed}, {days} days, mods: {}",
        s.mods.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(", ")
    );
    let campfire = thing("campfire");
    let axe = thing("primitive:hand_axe");
    let spot_def = thing("crafting:spot");
    let (mut billed, mut chopping) = (false, false);
    for day in 1..=days {
        // Like a player, each morning: mark what has regrown to be gathered
        // again; once the spot stands, ask it for tools; once there's an
        // axe, fell trees and plan the hut.
        let spot = s.world.map.fixture_at(spot_at).filter(|&e| {
            s.world.thing(e).is_some_and(|t| t.def == spot_def)
                && s.world.ecs.get::<&rim_sim::world::Blueprint>(e).is_err()
        });
        let bills = spot.filter(|_| !billed);
        let armed = !chopping && s.world.ecs.query::<&rim_sim::world::Thing>().iter().any(|t| t.def == axe);
        if day > 1 {
            for g in [&mut s, &mut twin] {
                g.push(Command::Designate { designation: des("gather"), a: c.offset(-20, -20), b: c.offset(20, 20) });
                if let Some(site) = bills {
                    for recipe in ["primitive:hammerstone", "primitive:hand_axe"] {
                        let data = [
                            (Key::Str("site".into()), Data::Int(site.to_bits().get() as i64)),
                            (Key::Str("recipe".into()), Data::Str(recipe.into())),
                        ];
                        let data = Some(Data::Table(data.into_iter().collect()));
                        g.push(Command::ModEvent { name: "crafting:add_bill".into(), data });
                    }
                }
                if armed {
                    hut(g);
                }
            }
            billed |= bills.is_some();
            chopping |= armed;
        }
        let threat = match day {
            RAID_DAY => Some("raid"),
            STAMPEDE_DAY => Some("boar_stampede"),
            _ => None,
        };
        if let Some(id) = threat {
            for g in [&mut s, &mut twin] {
                storyteller(g, "fire", &[Some(Data::Str(id.into()))]);
            }
        }
        let ticks = if day == 1 { TICKS_PER_DAY - 1 } else { TICKS_PER_DAY };
        for _ in 0..ticks {
            s.step();
            twin.step();
            if day == trace_day {
                println!("{}", rim_sim::bisect::trace_line(&s));
            }
        }
        let snap = Snapshot::capture(&s);
        if snap != Snapshot::capture(&twin) {
            eprintln!("day {day}: the game that was saved and loaded no longer matches the one that wasn't");
            std::process::exit(1);
        }
        if day % 5 == 0 {
            let bytes = Snapshot::capture(&twin).to_bytes();
            twin = Snapshot::from_bytes(&bytes).and_then(|b| b.restore(&mods, &|_| true)).expect("the twin reloads");
        }
        let w = &s.world;
        // The rest of the year hashes wildlife and weather alone.
        let colonists = w.colonists().count();
        if colonists == 0 {
            eprintln!("day {day}: the colony is lost, before the year's last day ({days})");
            std::process::exit(1);
        }
        // The scenario means to build its hut: a blueprint still waiting by
        // day 5 is a lost material or a lost job, not a determinism result.
        if day == 5 {
            use rim_sim::world::{Blueprint, Thing};
            let unbuilt = w.ecs.query::<&Thing>().with::<&Blueprint>().iter().count();
            let fires = w.ecs.query::<&Thing>().without::<&Blueprint>().iter().filter(|t| t.def == campfire).count();
            // Without an axe the scenario never reached its work orders or
            // its hut: say so rather than pass on less than it claims.
            if !chopping {
                eprintln!("day 5: no hand axe was made, so nothing was chopped or built (no flint in reach?)");
                std::process::exit(1);
            }
            if unbuilt > 0 || fires == 0 {
                eprintln!("day 5: {unbuilt} blueprints still waiting, or no campfire built");
                for t in w.ecs.query::<&Thing>().with::<&Blueprint>().iter() {
                    eprintln!("  waiting: {} at ({}, {})", w.defs.thing(t.def).id, t.pos.x, t.pos.y);
                }
                std::process::exit(1);
            }
        }
        println!(
            "day {day:>3}  hash {:016x}  snapshot {:016x}  pawns {:>3}  colonists {colonists:>2}  messages {:>4}",
            w.state_hash(),
            snap.hash(),
            w.pawns.len(),
            w.messages.len()
        );
    }
}
