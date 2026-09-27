//! An order's preview is what the order does (DESIGN.md §6f): the client
//! draws `designate_preview` and `build_preview`, and `apply` acts on
//! exactly the same lists, so a drag never shows one thing and does another.

mod common;

use rim_sim::command::{apply, build_preview, designate_preview, Blocker, Place, Target};
use rim_sim::hecs::Entity;
use rim_sim::world::{Blueprint, Designated, Planned, Thing, World};
use rim_sim::{Command, IVec, Sim};
use std::collections::BTreeSet;

/// A rectangle near the colony, from a small deterministic generator.
fn rects(seed: u64, n: usize, around: IVec) -> Vec<(IVec, IVec)> {
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut next = move |m: i32| {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((x >> 33) % m as u64) as i32
    };
    (0..n)
        .map(|_| {
            let a = around.offset(next(41) - 20, next(41) - 20);
            (a, a.offset(next(12), next(12)))
        })
        .collect()
}

fn marked(w: &World, d: u16) -> BTreeSet<Entity> {
    w.ecs.query::<(Entity, &Designated)>().iter().filter(|(_, m)| m.0 == d).map(|(e, _)| e).collect()
}

fn blueprints(w: &World) -> BTreeSet<Entity> {
    w.ecs.query::<(Entity, &Blueprint)>().iter().map(|(e, _)| e).collect()
}

fn planned(w: &World) -> BTreeSet<Entity> {
    w.ecs.query::<(Entity, &Planned)>().iter().map(|(e, _)| e).collect()
}

/// Everything the previews could change without it showing in
/// `state_hash`: marks, plans, blueprints and the map's revision.
fn fingerprint(w: &World) -> (u64, usize, usize, usize, usize, u64) {
    let marks = w.ecs.query::<&Designated>().iter().count();
    (w.state_hash(), w.ecs.len() as usize, marks, planned(w).len(), blueprints(w).len(), w.map.revision)
}

fn thing_id(w: &World, id: &str) -> u16 {
    w.defs.thing_id(id).unwrap_or_else(|| panic!("{id} is defined"))
}

#[test]
fn a_preview_changes_nothing() {
    let s = Sim::new(&common::mods(), 3).unwrap();
    let w = &s.world;
    let c = w.colony_center().unwrap();
    let before = fingerprint(w);
    let (a, b) = (c.offset(-30, -30), c.offset(30, 30));
    for d in 0..w.defs.designations.len() {
        designate_preview(w, d as u16, a, b);
    }
    let (wall, wood) = (thing_id(w, "core:wall"), thing_id(w, "core:wood"));
    for facing in 0..4 {
        build_preview(w, wall, Some(wood), a, b, facing);
    }
    assert_eq!(fingerprint(w), before);
}

#[test]
fn designating_marks_exactly_what_the_preview_named() {
    let mut seen = 0;
    for seed in 1..=20 {
        let mut s = Sim::new(&common::mods(), seed).unwrap();
        let c = s.world.colony_center().unwrap();
        for (i, (a, b)) in rects(seed, 6, c).into_iter().enumerate() {
            for d in 0..s.world.defs.designations.len() as u16 {
                let preview = designate_preview(&s.world, d, a, b);
                let before = marked(&s.world, d);
                apply(&mut s.world, Command::Designate { designation: d, a, b });
                let after = marked(&s.world, d);
                let fresh: BTreeSet<Entity> = after.difference(&before).copied().collect();
                let named: BTreeSet<Entity> = preview
                    .iter()
                    .map(|t| match *t {
                        Target::Thing(e) | Target::Creature(e) => e,
                        Target::Rock(p) => s.world.map.fixture_at(p).expect("the rock stood up"),
                    })
                    .collect();
                assert_eq!(named.len(), preview.len(), "seed {seed} rect {i}: a target is named twice");
                assert_eq!(fresh, named, "seed {seed} rect {i} designation {d}");
                seen += named.len();
            }
        }
    }
    assert!(seen > 100, "the rectangles hit {seen} targets; the test should see many");
}

/// Apply a build and check it planned exactly the preview's open and
/// clears cells. Returns how many of each outcome it saw.
fn build_matches(s: &mut Sim, thing: u16, stuff: Option<u16>, a: IVec, b: IVec, facing: u8) -> [usize; 3] {
    let preview = build_preview(&s.world, thing, stuff, a, b, facing);
    let (bp0, pl0) = (blueprints(&s.world), planned(&s.world));
    apply(&mut s.world, Command::Build { thing, stuff, a, b, facing });
    let w = &s.world;
    let anchors: BTreeSet<IVec> =
        blueprints(w).difference(&bp0).map(|&e| w.ecs.get::<&Thing>(e).unwrap().pos).collect();
    let cleared: BTreeSet<Entity> = planned(w).difference(&pl0).copied().collect();
    let open: BTreeSet<IVec> = preview.iter().filter(|(_, p)| *p == Place::Open).map(|(c, _)| *c).collect();
    let mut clears: BTreeSet<Entity> = preview
        .iter()
        .filter_map(|(_, p)| match *p {
            Place::Clears(Target::Thing(e) | Target::Creature(e)) => Some(e),
            Place::Clears(Target::Rock(c)) => Some(w.map.fixture_at(c).expect("the rock stood up")),
            _ => None,
        })
        .collect();
    // A building bigger than a cell names the first thing in its way; the
    // rest of its footprint is marked for it too, anchored at its cell.
    let anchored: BTreeSet<IVec> =
        preview.iter().filter(|(_, p)| matches!(p, Place::Clears(_))).map(|(c, _)| *c).collect();
    clears.extend(
        w.ecs
            .query::<(Entity, &Planned)>()
            .iter()
            .filter(|(_, p)| p.at.is_some_and(|at| anchored.contains(&at)))
            .map(|(e, _)| e),
    );
    assert_eq!(anchors, open, "blueprints went up where the preview said open");
    // A thing already planned for a clearing is planned again: it's in the
    // preview but not new.
    assert!(cleared.is_subset(&clears), "only the preview's things were marked to clear");
    for &e in &clears {
        let p = w.ecs.get::<&Planned>(e).map(|p| (p.thing, p.facing & 3));
        assert_eq!(p.ok(), Some((thing, facing & 3)), "a cleared thing waits for this plan");
    }
    let mut n = [0; 3];
    for (_, p) in &preview {
        n[match p {
            Place::Open => 0,
            Place::Clears(_) => 1,
            Place::Blocked(_) => 2,
        }] += 1;
    }
    n
}

#[test]
fn building_plans_exactly_the_open_and_cleared_cells() {
    let mut n = [0; 3];
    for seed in 1..=20 {
        let mut s = Sim::new(&common::mods(), seed).unwrap();
        let c = s.world.colony_center().unwrap();
        let (wall, wood) = (thing_id(&s.world, "core:wall"), thing_id(&s.world, "core:wood"));
        // Two cells wide, so each facing turns the footprint.
        let pile = thing_id(&s.world, "primitive:woodpile");
        for (i, (a, b)) in rects(seed, 8, c).into_iter().enumerate() {
            let got = if i % 2 == 0 {
                build_matches(&mut s, wall, Some(wood), a, b, 0)
            } else {
                build_matches(&mut s, pile, None, a, b, (i / 2 % 4) as u8)
            };
            for k in 0..3 {
                n[k] += got[k];
            }
        }
    }
    assert!(n.iter().all(|&k| k > 0), "open, clears and blocked all seen: {n:?}");
}

#[test]
fn water_blocks_a_tree_clears_and_a_wall_is_in_the_way() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let (wall, wood) = (thing_id(&s.world, "core:wall"), thing_id(&s.world, "core:wood"));
    let at = |w: &World, p: IVec| build_preview(w, wall, Some(wood), p, p, 0)[0].1;

    let deep = s.world.defs.terrain.iter().position(|t| t.id == "core:deep_water").expect("deep water") as u16;
    let water = (0..s.world.map.w * s.world.map.h)
        .map(|i| s.world.map.pos(i as usize))
        .find(|&p| s.world.map.terrain[s.world.map.idx(p)] == deep && s.world.map.fixture_at(p).is_none())
        .expect("a cell of deep water");
    assert_eq!(at(&s.world, water), Place::Blocked(Blocker::Terrain(water)));

    let oak = thing_id(&s.world, "core:tree_oak");
    let (tree, pos) = s
        .world
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .find(|(_, t)| t.def == oak)
        .map(|(e, t)| (e, t.pos))
        .expect("an oak");
    assert_eq!(at(&s.world, pos), Place::Clears(Target::Thing(tree)));

    let open = (0..s.world.map.w * s.world.map.h)
        .map(|i| s.world.map.pos(i as usize))
        .find(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.solid_at(p).is_none())
        .expect("open ground");
    let built = s.world.spawn_fixture_facing(wall, open, false, Some(wood), 0).expect("a wall");
    assert_eq!(at(&s.world, open), Place::Blocked(Blocker::Occupied(built)));
}

#[test]
fn a_build_without_its_material_plans_nothing() {
    let s = Sim::new(&common::mods(), 3).unwrap();
    let wall = thing_id(&s.world, "core:wall");
    let c = s.world.colony_center().unwrap();
    let p = build_preview(&s.world, wall, None, c, c.offset(2, 0), 0);
    assert_eq!(p.len(), 3);
    assert!(p.iter().all(|(_, place)| *place == Place::Blocked(Blocker::NoMaterial)));
}

/// `apply` for Designate and Build as it was before the previews, kept to
/// show the rewrite changed nothing but what it meant to.
fn old_apply(w: &mut World, c: Command) {
    use rim_sim::defs::Targets;
    use rim_sim::world::{Faction, Owner, Pawn};
    let cells = |w: &World, a: IVec, b: IVec| -> Vec<IVec> {
        let (x0, x1) = (a.x.min(b.x).max(0), a.x.max(b.x).min(w.map.w - 1));
        let (y0, y1) = (a.y.min(b.y).max(0), a.y.max(b.y).min(w.map.h - 1));
        let y1 = if w.map.levels().contains(&a.z) { y1 } else { y0 - 1 };
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| IVec::at(x, y, a.z))).collect()
    };
    let in_rect = |p: IVec, a: IVec, b: IVec| {
        (a.x.min(b.x)..=a.x.max(b.x)).contains(&p.x) && (a.y.min(b.y)..=a.y.max(b.y)).contains(&p.y)
    };
    let defs = w.defs.clone();
    match c {
        Command::Designate { designation, a, b } => match defs.designations[designation as usize].targets {
            Targets::Thing => {
                for p in cells(w, a, b) {
                    let wake = w.map.fixture_at(p).is_none()
                        && w.solid_at(p)
                            .and_then(|s| s.thing_r)
                            .is_some_and(|t| defs.thing(t).harvest_for(designation).is_some());
                    if wake {
                        w.wake_rock(p);
                    }
                    let Some(f) = w.map.fixture_at(p) else { continue };
                    let Some(t) = w.thing(f) else { continue };
                    let clears = defs.thing(t.def).harvest_for(designation).is_some_and(|h| h.destroy);
                    if w.ecs.get::<&Planned>(f).is_ok() && !clears {
                        continue;
                    }
                    if defs.thing(t.def).harvest_for(designation).is_some() {
                        let _ = w.ecs.insert_one(f, Designated(designation));
                    }
                }
            }
            Targets::Built => {
                for p in cells(w, a, b) {
                    for f in [w.map.fixture_at(p), w.map.floor_at(p)].into_iter().flatten() {
                        let ours = w.ecs.get::<&Owner>(f).is_ok_and(|o| o.0 == Faction::Player);
                        let built = w.thing(f).is_some_and(|t| defs.thing(t.def).build.is_some());
                        if ours && built && w.ecs.get::<&Blueprint>(f).is_err() {
                            let _ = w.ecs.insert_one(f, Designated(designation));
                        }
                    }
                }
            }
            Targets::Creature => {
                for e in w.pawns.clone() {
                    let ok = w.ecs.get::<&Pawn>(e).is_ok_and(|p| {
                        p.faction == Faction::Wild && !defs.creature(p.def).butcher_r.is_empty() && in_rect(p.pos, a, b)
                    });
                    if ok {
                        let _ = w.ecs.insert_one(e, Designated(designation));
                    }
                }
            }
        },
        Command::Build { thing, stuff, a, b, facing } => {
            let Some(bd) = defs.thing(thing).build.as_ref() else { return };
            if let Some(sc) = &bd.stuff {
                match stuff {
                    Some(m) if defs.is_material_for(m, &sc.category) => {}
                    _ => return,
                }
            }
            let floor = defs.thing(thing).category == rim_sim::defs::Category::Floor;
            let big = defs.thing(thing).size != [1, 1];
            for p in cells(w, a, b) {
                if big && !floor {
                    w.plan_footprint(thing, stuff, p, facing);
                    continue;
                }
                let ground = w.map.inb(p) && w.map.terrain_cost[w.map.idx(p)] > 0;
                let natural = match w.solid_at(p).is_some() && !floor {
                    true => w.wake_rock(p),
                    false => {
                        w.map.fixture_at(p).filter(|&f| ground && w.thing(f).is_some_and(|t| defs.thing(t.def).natural))
                    }
                };
                match natural {
                    Some(f) if !floor => w.plan_over_facing(f, thing, stuff, facing),
                    _ if w.map.passable(p) => {
                        w.spawn_fixture_facing(thing, p, true, stuff, facing);
                    }
                    _ => {}
                }
            }
        }
        _ => unreachable!("only orders with a preview"),
    }
}

/// Every thing, mark and plan in the world, in a fixed order.
fn summary(w: &World) -> Vec<(u64, u16, i32, i32, u64, u64)> {
    let mut v: Vec<_> = w
        .ecs
        .query::<(Entity, &Thing)>()
        .iter()
        .map(|(e, t)| {
            let mark = w.ecs.get::<&Designated>(e).map_or(0, |d| d.0 as u64 + 1);
            let plan = w.ecs.get::<&Planned>(e).map_or(0, |p| ((p.thing as u64) << 8 | p.facing as u64) + 1);
            (e.to_bits().get(), t.def, t.pos.x, t.pos.y, mark, plan)
        })
        .collect();
    for &e in &w.pawns {
        if let Ok(d) = w.ecs.get::<&Designated>(e) {
            v.push((e.to_bits().get(), d.0, -1, -1, 1, 0));
        }
    }
    v.sort_unstable();
    v
}

#[test]
fn apply_does_what_it_did_before_the_previews() {
    for seed in 1..=10 {
        let mut new = Sim::new(&common::mods(), seed).unwrap();
        let mut old = Sim::new(&common::mods(), seed).unwrap();
        let c = new.world.colony_center().unwrap();
        let (wall, wood) = (thing_id(&new.world, "core:wall"), thing_id(&new.world, "core:wood"));
        let pile = thing_id(&new.world, "primitive:woodpile");
        for (i, (a, b)) in rects(seed + 100, 8, c).into_iter().enumerate() {
            let mut orders: Vec<Command> = (0..new.world.defs.designations.len() as u16)
                .map(|designation| Command::Designate { designation, a, b })
                .collect();
            orders.push(Command::Build { thing: wall, stuff: Some(wood), a, b, facing: 0 });
            orders.push(Command::Build { thing: pile, stuff: None, a, b, facing: (i % 4) as u8 });
            for o in orders {
                apply(&mut new.world, o.clone());
                old_apply(&mut old.world, o.clone());
                assert!(summary(&new.world) == summary(&old.world), "seed {seed} rect {i}: {o:?} differs");
            }
        }
    }
}
