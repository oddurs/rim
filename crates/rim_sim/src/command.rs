//! Player input. Every UI action becomes a `Command` applied at a tick
//! boundary; this is what makes replays and lockstep multiplayer possible.

use crate::ai;
use crate::defs::{DefId, Targets};
use crate::filter::FilterEdit;
use crate::order;
use crate::world::*;
use crate::zone::StoreRef;
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
    /// `pick` names one of the spot's options (`order::options`, by key),
    /// as the orders menu does; without it the order is the first safe one,
    /// never a damaging one.
    Order {
        pawn: Entity,
        cell: IVec,
        on: Option<Entity>,
        #[serde(default)]
        pick: Option<String>,
    },
    /// Take back an order: if the pawn is still on the job an order gave it
    /// (aimed at `target`, or walking to `cell` when there's no target), it
    /// stops and goes back to choosing its own work; `unmark` loses the
    /// designation the order put on it (a deconstruct marks what it takes
    /// down). What the order already did stays done.
    UndoOrder {
        pawn: Entity,
        target: Option<Entity>,
        cell: IVec,
        unmark: Option<Entity>,
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
    /// Change what a store takes: things, categories of them, materials,
    /// condition (DESIGN.md §4f).
    StoreFilter {
        store: StoreRef,
        edit: FilterEdit,
    },
    /// Set how much a colonist wants to do a kind of work: 1 first, 0 never,
    /// up to the priority scale's `levels` (DESIGN.md §4d).
    SetPriority {
        pawn: Entity,
        work: DefId,
        level: u8,
    },
    /// Hand a colonist's work type back: forget their own setting, so they
    /// follow what they'd inherit again (DESIGN.md §4d).
    ClearPriority {
        pawn: Entity,
        work: DefId,
    },
    /// Switch a priority rule off for this colony, or back on: a standing
    /// order the player doesn't want (DESIGN.md §4d).
    SetRuleEnabled {
        rule: DefId,
        on: bool,
    },
    /// Put a colonist in a work role, by its index in the colony's roles.
    /// Their pins stay (DESIGN.md §4d).
    AssignWorkRole {
        pawn: Entity,
        role: u16,
    },
    /// Change the level a colony's work role sets for a work type, or leave
    /// it to the default with `None`. The role is the player's from then on.
    SetRolePriority {
        role: u16,
        work: DefId,
        level: Option<u8>,
    },
    /// A new work role of the player's, starting from another role's levels
    /// or from a colonist's (their role and pins together).
    CreateWorkRole {
        label: String,
        from: RoleSource,
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

/// Where a new work role's levels come from.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RoleSource {
    /// A copy of one of the colony's roles.
    Role(u16),
    /// What a colonist has now before the rules: role and pins.
    Pawn(Entity),
}

/// The longest a work role's name may be, in characters.
pub const ROLE_LABEL_MAX: usize = 40;

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
                    // Rock that can be marked this way is stood up to take
                    // the mark; any other rock stays terrain.
                    let wake = w.map.fixture_at(p).is_none()
                        && w.solid_at(p).is_some_and(|s| defs.thing(s.thing_r).harvest_for(designation).is_some());
                    if wake {
                        w.wake_rock(p);
                    }
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
                // Rock is cleared by mining it, so it stands up to be marked.
                let natural = match w.solid_at(p).is_some() && !floor {
                    true => w.wake_rock(p),
                    false => {
                        w.map.fixture_at(p).filter(|&f| ground && w.thing(f).is_some_and(|t| defs.thing(t.def).natural))
                    }
                };
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
                    // Rock nobody will work goes back to being terrain.
                    w.settle_rock(f);
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
            w.recount_stored();
        }
        Command::Stockpile { a, b, zone: Some(id) } => {
            w.zones.paint(&w.map, a, b, Some(id));
            w.recount_stored();
        }
        Command::ClearZone { a, b } => {
            w.zones.paint(&w.map, a, b, None);
            w.recount_stored();
        }
        Command::ZoneAllow { zone, thing, on } => {
            w.zones.edit(&defs, zone, FilterEdit::Thing { thing, on });
            w.recount_stored();
        }
        Command::StoreFilter { store: StoreRef::Zone(zone), edit } => {
            w.zones.edit(&defs, zone, edit);
            w.recount_stored();
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
        Command::ClearPriority { pawn, work } => {
            if !is_colonist(w, pawn) || work as usize >= defs.work_types.len() {
                return;
            }
            if let Ok(mut p) = w.ecs.get::<&mut Pawn>(pawn) {
                p.clear_priority(work);
            }
        }
        Command::SetRuleEnabled { rule, on } => w.set_rule_enabled(rule, on),
        Command::AssignWorkRole { pawn, role } => {
            if !is_colonist(w, pawn) || role as usize >= w.work_roles.len() {
                return;
            }
            if let Ok(mut p) = w.ecs.get::<&mut Pawn>(pawn) {
                // A plan belongs to the planned role; leaving it drops it.
                if p.work_role != Some(role) {
                    p.plan.clear();
                    p.proposal.clear();
                }
                p.work_role = Some(role);
            }
        }
        Command::SetRolePriority { role, work, level } => {
            if work as usize >= defs.work_types.len() {
                return;
            }
            let levels = defs.priority_scale.levels;
            // A planned role's levels are its planner's to set.
            if let Some(r) = w.work_roles.get_mut(role as usize).filter(|r| r.planner.is_none()) {
                r.set(work, level.map(|l| l.min(levels)));
                r.edited = true;
            }
        }
        Command::CreateWorkRole { label, from } => {
            let label: String = label.trim().chars().take(ROLE_LABEL_MAX).collect();
            if label.is_empty() {
                return;
            }
            let priorities = match from {
                RoleSource::Role(r) => match w.work_roles.get(r as usize) {
                    Some(r) => r.priorities.clone(),
                    None => return,
                },
                // What differs from the defaults, so the new role leaves the
                // rest to them as any role does.
                RoleSource::Pawn(e) => {
                    let Ok(p) = w.ecs.get::<&Pawn>(e) else { return };
                    (0..defs.work_types.len() as DefId)
                        .map(|t| (t, w.base_priority(&p, t)))
                        .filter(|&(t, l)| l != defs.work_types[t as usize].priority.min(defs.priority_scale.levels))
                        .collect()
                }
            };
            let order = w.work_roles.iter().map(|r| r.order).max().unwrap_or(0) + 10;
            w.work_roles.push(crate::rules::WorkRole {
                def: None,
                label,
                order,
                priorities,
                edited: true,
                planner: None,
            });
        }
        Command::SetStance { stance } => {
            if (stance as usize) < defs.stances.len() {
                w.stance = Some(stance);
                w.update_rules();
            }
        }
        Command::UndoOrder { pawn, target, cell, unmark } => {
            if !is_colonist(w, pawn) {
                return;
            }
            let still = w.ecs.get::<&Pawn>(pawn).is_ok_and(|p| match target {
                Some(t) => order::target_of(&p.job) == Some(t),
                None => matches!(p.job, Job::MoveTo { to } if to == cell),
            });
            if still {
                ai::interrupt(w, pawn);
            }
            if let Some(t) = unmark {
                if w.ecs.remove_one::<Designated>(t).is_ok() {
                    w.touch(t);
                }
            }
        }
        Command::Order { pawn, cell, on, pick } => {
            if !is_colonist(w, pawn) {
                return;
            }
            // Rock is listed from its terrain; the order needs the thing.
            let rock = (on.is_none() && w.map.fixture_at(cell).is_none()).then(|| w.wake_rock(cell)).flatten();
            let o = match &pick {
                Some(key) => order::choose(w, pawn, cell, on, key),
                None => order::resolve(w, pawn, cell, on),
            };
            let Some(o) = o else {
                if let Some(r) = rock {
                    w.settle_rock(r);
                }
                return;
            };
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
