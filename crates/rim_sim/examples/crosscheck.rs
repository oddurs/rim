//! The cross-machine determinism scenario (0157): every platform in CI runs
//! this and must print exactly the same lines.
//!
//!   cargo run --release -p rim_sim --example crosscheck [-- --days N]
//!
//! All shipped mods, a fixed seed, and a player's commands (gather, forage,
//! a campfire, a crafting spot with bills for a hammerstone and a hand axe,
//! then chop and a wooden hut with a door and a bed), so it exercises the
//! engine, the storyteller and its incidents, the weather plugin, the
//! building code and work orders, over one calendar year by default so every season's weather
//! and growth is in the hash. Prints the state hash at the end of each day, so when two
//! platforms disagree the output shows the first day they diverged.
//!
//! A twin of the game is saved and loaded every few days (DESIGN.md §7a),
//! and must match the one that never saved, section for section, every day.

use rim_sim::data::{Data, Key};
use rim_sim::snapshot::Snapshot;
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::path::Path;

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let seed = arg("--seed", 1);
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
    let o = c.offset(3, 3);
    let at = |dx: i32, dy: i32| -> IVec { o.offset(dx, dy) };
    s.push(Command::Build { thing: thing("campfire"), stuff: None, a: at(1, 1), b: at(1, 1) });
    let spot_at = at(2, 6);
    s.push(Command::Build { thing: thing("crafting:spot"), stuff: None, a: spot_at, b: spot_at });
    // With an axe: fell the trees, and raise a 5x5 wooden hut around the
    // fire, with a door on the south side and a bed.
    let hut = |g: &mut Sim| {
        g.push(Command::Designate { designation: des("chop"), a: c.offset(-14, -14), b: c.offset(14, 14) });
        for y in 0..5 {
            for x in 0..5 {
                let p = o.offset(x, y);
                let edge = x == 0 || y == 0 || x == 4 || y == 4;
                if edge && p != o.offset(2, 4) {
                    g.push(Command::Build { thing: thing("wall"), stuff: wood, a: p, b: p });
                }
            }
        }
        g.push(Command::Build { thing: thing("door"), stuff: wood, a: at(2, 4), b: at(2, 4) });
        g.push(Command::Build { thing: thing("bed"), stuff: wood, a: at(2, 2), b: at(2, 2) });
    };

    // The commands apply on the first tick; the twin splits off after it.
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
        let ticks = if day == 1 { TICKS_PER_DAY - 1 } else { TICKS_PER_DAY };
        for _ in 0..ticks {
            s.step();
            twin.step();
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
            "day {day:>3}  hash {:016x}  snapshot {:016x}  pawns {:>3}  messages {:>4}",
            w.state_hash(),
            snap.hash(),
            w.pawns.len(),
            w.messages.len()
        );
    }
}
