//! Player input. Every UI action becomes a `Command` applied at a tick
//! boundary; this is what makes replays and lockstep multiplayer possible.

use crate::ai;
use crate::defs::{DefId, Targets};
use crate::order;
use crate::world::*;
use crate::IVec;
use hecs::Entity;

#[derive(Clone, Debug)]
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
                    if defs.thing(t.def).harvest.as_ref().is_some_and(|h| h.desig_r == designation) {
                        let _ = w.ecs.insert_one(f, Designated(designation));
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
            for p in cells(w, a, b).collect::<Vec<_>>() {
                if w.map.passable(p) {
                    w.spawn_fixture_of(thing, p, true, stuff);
                }
            }
        }
        Command::Cancel { a, b } => {
            for p in cells(w, a, b).collect::<Vec<_>>() {
                let Some(f) = w.map.fixture_at(p) else { continue };
                let _ = w.ecs.remove_one::<Designated>(f);
                // Refund what was actually delivered, of whatever it was
                // made of -- not what the def says it costs.
                let refund = w.ecs.get::<&Blueprint>(f).ok().map(|bp| (bp.cost.clone(), bp.delivered.clone()));
                if let Some((cost, delivered)) = refund {
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
