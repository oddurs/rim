//! Player input. Every UI action becomes a `Command` applied at a tick
//! boundary; this is what makes replays and lockstep multiplayer possible.

use crate::ai;
use crate::defs::{Category, DefId, Targets};
use crate::order;
use crate::world::*;
use crate::IVec;
use hecs::Entity;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Command {
    /// Mark things (or creatures) in a rectangle with a designation.
    Designate {
        designation: DefId,
        a: IVec,
        b: IVec,
    },
    /// Place blueprints of a buildable thing over a rectangle. `stuff` is
    /// the material chosen for it, and rides the command so a replay builds
    /// the same wall out of the same thing.
    Build {
        thing: DefId,
        stuff: Option<DefId>,
        a: IVec,
        b: IVec,
    },
    /// Remove designations and blueprints in a rectangle.
    Cancel {
        a: IVec,
        b: IVec,
    },
    Draft {
        pawn: Entity,
        on: bool,
    },
    /// Right-click: give one pawn the job that fits what it was clicked
    /// on, ahead of whatever it picked for itself. `on` is the creature
    /// under the cursor, which wins over the cell it is standing in.
    Order {
        pawn: Entity,
        cell: IVec,
        on: Option<Entity>,
    },
    /// Paint cells into a stockpile: into `zone`, or a new zone taking every
    /// item when it's `None`.
    Stockpile {
        a: IVec,
        b: IVec,
        zone: Option<u32>,
    },
    /// Take cells out of whatever zone they're in.
    ClearZone {
        a: IVec,
        b: IVec,
    },
    /// Let a zone take an item, or stop it.
    ZoneAllow {
        zone: u32,
        thing: DefId,
        on: bool,
    },
    /// Set how much a colonist wants to do a kind of work: 1 first, 0 never,
    /// up to the priority scale's `levels` (DESIGN.md §4d).
    SetPriority {
        pawn: Entity,
        work: DefId,
        level: u8,
    },
    /// Put the colony in a stance: its priority rules hold until another.
    SetStance {
        stance: DefId,
    },
    /// A mod's interface asks its own sim scripts to do something: delivered
    /// as the script event `name` (namespaced by the mod, "weather:force")
    /// at the tick boundary, like any other input, so it replays and stays
    /// in lockstep.
    ModEvent {
        name: String,
        data: Option<crate::data::Data>,
    },
}

fn cells(w: &World, a: IVec, b: IVec) -> impl Iterator<Item = IVec> {
    let (x0, x1) = (a.x.min(b.x).max(0), a.x.max(b.x).min(w.map.w - 1));
    let (y0, y1) = (a.y.min(b.y).max(0), a.y.max(b.y).min(w.map.h - 1));
    (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| IVec::new(x, y)))
}

fn in_rect(p: IVec, a: IVec, b: IVec) -> bool {
    (a.x.min(b.x)..=a.x.max(b.x)).contains(&p.x) && (a.y.min(b.y)..=a.y.max(b.y)).contains(&p.y)
}

pub fn apply(w: &mut World, c: Command) {
    let defs = w.defs.clone();
    match c {
        Command::Designate { designation, a, b } => match defs.designations[designation as usize].targets {
            Targets::Thing => {
                for p in cells(w, a, b).collect::<Vec<_>>() {
                    let Some(f) = w.map.fixture_at(p) else { continue };
                    let Some(t) = w.thing(f) else { continue };
                    // A thing cleared for a building keeps the mark that
                    // clears it: gathering an oak planned over would leave
                    // the oak, and the wall waiting on it, forever.
                    let clears = defs.thing(t.def).harvest_for(designation).is_some_and(|h| h.destroy);
                    if w.ecs.get::<&Planned>(f).is_ok() && !clears {
                        continue;
                    }
                    if defs.thing(t.def).harvest_for(designation).is_some() {
                        let _ = w.ecs.insert_one(f, Designated(designation));
                        w.map.touch(p);
                    }
                }
            }
            Targets::Built => {
                for p in cells(w, a, b).collect::<Vec<_>>() {
                    for f in [w.map.fixture_at(p), w.map.floor_at(p)].into_iter().flatten() {
                        let ours = w.ecs.get::<&Owner>(f).is_ok_and(|o| o.0 == Faction::Player);
                        let built = w.thing(f).is_some_and(|t| defs.thing(t.def).build.is_some());
                        // A blueprint is cancelled, not deconstructed.
                        if ours && built && w.ecs.get::<&Blueprint>(f).is_err() {
                            let _ = w.ecs.insert_one(f, Designated(designation));
                            w.map.touch(p);
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
        Command::Build { thing, stuff, a, b } => {
            let Some(bd) = defs.thing(thing).build.as_ref() else { return };
            // A buildable that takes a material needs one, of the right kind.
            if let Some(sc) = &bd.stuff {
                match stuff {
                    Some(m) if defs.is_material_for(m, &sc.category) => {}
                    _ => return,
                }
            }
            let floor = defs.thing(thing).category == crate::defs::Category::Floor;
            for p in cells(w, a, b).collect::<Vec<_>>() {
                // Grass, a tree or rock in the way is cleared first, not
                // silently left out: a wall with a gap is no wall.
                // (On ground that can be built on: nothing is planned over water.)
                let ground = w.map.inb(p) && w.map.terrain_cost[w.map.idx(p)] > 0;
                let natural =
                    w.map.fixture_at(p).filter(|&f| ground && w.thing(f).is_some_and(|t| defs.thing(t.def).natural));
                match natural {
                    Some(f) if !floor => w.plan_over(f, thing, stuff),
                    _ if w.map.passable(p) => {
                        w.spawn_fixture_of(thing, p, true, stuff);
                    }
                    _ => {}
                }
            }
        }
        Command::Cancel { a, b } => {
            let targets: Vec<Entity> =
                cells(w, a, b).flat_map(|p| [w.map.fixture_at(p), w.map.floor_at(p)]).flatten().collect();
            for f in targets {
                // Cancelling a plan over grass or a tree leaves it be.
                let planned = w.ecs.remove_one::<Planned>(f).is_ok();
                if w.ecs.remove_one::<Designated>(f).is_ok() || planned {
                    w.touch(f);
                }
                // Refund what was actually delivered, of whatever it was
                // made of -- not what the def says it costs.
                let refund = w.ecs.get::<&Blueprint>(f).ok().map(|bp| (bp.cost.clone(), bp.delivered.clone()));
                if let Some((cost, delivered)) = refund {
                    let p = w.thing(f).map(|t| t.pos).unwrap_or(a);
                    w.despawn_thing(f);
                    for (c, n) in cost.iter().zip(delivered) {
                        if n > 0 {
                            w.place_item(c.0, p, n);
                        }
                    }
                }
            }
            for e in w.pawns.clone() {
                if w.pawn_pos(e).is_some_and(|p| in_rect(p, a, b)) {
                    let _ = w.ecs.remove_one::<Designated>(e);
                }
            }
        }
        Command::Draft { pawn, on } => {
            if !is_colonist(w, pawn) {
                return;
            }
            ai::interrupt(w, pawn);
            if let Ok(mut p) = w.ecs.get::<&mut Pawn>(pawn) {
                p.drafted = on;
            }
        }
        Command::ModEvent { name, data } => w.events.push(GameEvent::Script { name, data }),
        Command::Stockpile { a, b, zone: None } => {
            w.zones.create(&defs, &w.map, a, b);
        }
        Command::Stockpile { a, b, zone: Some(id) } => w.zones.paint(&w.map, a, b, Some(id)),
        Command::ClearZone { a, b } => w.zones.paint(&w.map, a, b, None),
        Command::ZoneAllow { zone, thing, on } => {
            if (thing as usize) < defs.things.len() && defs.thing(thing).category == Category::Item {
                w.zones.allow(zone, thing, on);
            }
        }
        Command::SetPriority { pawn, work, level } => {
            if !is_colonist(w, pawn) || work as usize >= defs.work_types.len() {
                return;
            }
            let level = level.min(defs.priority_scale.levels);
            if let Ok(mut p) = w.ecs.get::<&mut Pawn>(pawn) {
                p.set_priority(work, level);
            }
        }
        Command::SetStance { stance } => {
            if (stance as usize) < defs.stances.len() {
                w.stance = Some(stance);
                w.update_rules();
            }
        }
        Command::Order { pawn, cell, on } => {
            if !is_colonist(w, pawn) {
                return;
            }
            let Some(o) = order::resolve(w, pawn, cell, on) else { return };
            // The player outranks whoever was already on this work.
            let held: Vec<Entity> =
                o.reserve.iter().filter_map(|t| w.reservations.get(t).copied()).filter(|&h| h != pawn).collect();
            for holder in held {
                ai::interrupt(w, holder);
            }
            // A deconstruct order is a one-thing designation: the job checks
            // for the mark so a later Cancel can still stop it.
            if let Job::Deconstruct { target, .. } = o.job {
                if let Some(d) = defs.designations.iter().position(|d| d.targets == Targets::Built) {
                    let _ = w.ecs.insert_one(target, Designated(d as DefId));
                    w.touch(target);
                }
            }
            // set_job drops the pawn's own claims, so take the new ones after.
            ai::set_job(w, pawn, o.job);
            for t in o.reserve {
                w.reserve(t, pawn);
            }
        }
    }
}

fn is_colonist(w: &World, e: Entity) -> bool {
    w.ecs.get::<&Pawn>(e).is_ok_and(|p| p.active && !p.dead && p.faction == Faction::Player)
}
