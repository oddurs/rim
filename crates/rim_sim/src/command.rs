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
        /// Quarter turns clockwise (DESIGN.md §6c). Old logs have none.
        #[serde(default)]
        facing: u8,
    },
    /// Place a house plan (DESIGN.md §6c) with its corner at `at`, turned
    /// `facing` quarter turns clockwise: every piece becomes a blueprint,
    /// as a Build would, and any that can't go up is left out. `stuff`
    /// replaces the plan's material wherever it will do.
    PlacePlan {
        plan: DefId,
        at: IVec,
        facing: u8,
        stuff: Option<DefId>,
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
    /// Put a store at a level of the store priority scale: stacks move only
    /// to a higher one (DESIGN.md §4f).
    StoreLevel {
        store: StoreRef,
        level: u8,
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
    /// Mark a job urgent, or clear the mark (DESIGN.md §4d): a blueprint, a
    /// thing or creature marked for work, or an order's site.
    MarkUrgent {
        target: Entity,
        on: bool,
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

/// The cells of the rectangle from `a` to `b`, on `a`'s level; none on a
/// level the map doesn't have.
fn cells(w: &World, a: IVec, b: IVec) -> impl Iterator<Item = IVec> {
    let (x0, x1) = (a.x.min(b.x).max(0), a.x.max(b.x).min(w.map.w - 1));
    let (y0, y1) = (a.y.min(b.y).max(0), a.y.max(b.y).min(w.map.h - 1));
    let y1 = if w.map.levels().contains(&a.z) { y1 } else { y0 - 1 };
    (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| IVec::at(x, y, a.z)))
}

fn in_rect(p: IVec, a: IVec, b: IVec) -> bool {
    (a.x.min(b.x)..=a.x.max(b.x)).contains(&p.x) && (a.y.min(b.y)..=a.y.max(b.y)).contains(&p.y)
}

/// Something an order would act on: a thing, rock still asleep in its cell
/// (stood up when the order lands), or a creature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Thing(Entity),
    Rock(IVec),
    Creature(Entity),
}

/// What a Build order would do in one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The blueprint goes up now, anchored here. Anything natural that
    /// makes way at once (grass) is replaced.
    Open,
    /// The target is marked to be cleared, and the blueprint goes up once
    /// it's gone.
    Clears(Target),
    /// Nothing is planned here.
    Blocked(Blocker),
}

/// Why a cell can't take a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocker {
    /// Terrain nothing stands on: the cell, or a cell of the footprint.
    Terrain(IVec),
    /// Solid rock that nothing clears.
    Solid(IVec),
    /// A thing already there, in the cell or the footprint.
    Occupied(Entity),
    /// The footprint runs off the map.
    OutOfBounds,
    /// An earlier cell of the same order already put a plan over it.
    Overlap,
    /// The order has no material, or one of the wrong kind.
    NoMaterial,
    /// The thing isn't a buildable at all.
    NotBuildable,
}

/// Everything `Command::Designate` would newly mark, in the order it would
/// mark them. Nothing is changed: `apply` acts on exactly this list.
pub fn designate_preview(w: &World, designation: DefId, a: IVec, b: IVec) -> Vec<Target> {
    let defs = &w.defs;
    let marked = |e: Entity| w.ecs.get::<&Designated>(e).is_ok_and(|d| d.0 == designation);
    // A thing bigger than a cell is in the rectangle once per cell.
    let mut seen = std::collections::BTreeSet::new();
    let mut out: Vec<Target> = Vec::new();
    let mut push = |t: Target| {
        if seen.insert(t) {
            out.push(t);
        }
    };
    match defs.designations[designation as usize].targets {
        Targets::Thing => {
            for p in cells(w, a, b) {
                let Some(f) = w.map.fixture_at(p) else {
                    let rock = w.solid_at(p).and_then(|s| s.thing_r);
                    if rock.is_some_and(|t| defs.thing(t).harvest_for(designation).is_some()) {
                        push(Target::Rock(p));
                    }
                    continue;
                };
                let Some(t) = w.thing(f) else { continue };
                let td = defs.thing(t.def);
                // A thing cleared for a building keeps the mark that
                // clears it: gathering an oak planned over would leave
                // the oak, and the wall waiting on it, forever.
                let clears = td.harvest_for(designation).is_some_and(|h| h.destroy);
                if w.ecs.get::<&Planned>(f).is_ok() && !clears {
                    continue;
                }
                if td.harvest_for(designation).is_some() && !marked(f) {
                    push(Target::Thing(f));
                }
            }
        }
        Targets::Built => {
            for p in cells(w, a, b) {
                for f in [w.map.fixture_at(p), w.map.floor_at(p)].into_iter().flatten() {
                    let ours = w.ecs.get::<&Owner>(f).is_ok_and(|o| o.0 == Faction::Player);
                    let built = w.thing(f).is_some_and(|t| defs.thing(t.def).build.is_some());
                    // A blueprint is cancelled, not deconstructed.
                    if ours && built && w.ecs.get::<&Blueprint>(f).is_err() && !marked(f) {
                        push(Target::Thing(f));
                    }
                }
            }
        }
        Targets::Creature => {
            for &e in &w.pawns {
                let ok = w.ecs.get::<&Pawn>(e).is_ok_and(|p| {
                    p.faction == Faction::Wild && !defs.creature(p.def).butcher_r.is_empty() && in_rect(p.pos, a, b)
                });
                if ok && !marked(e) {
                    push(Target::Creature(e));
                }
            }
        }
    }
    out
}

fn is_floor(w: &World, thing: DefId) -> bool {
    w.defs.thing(thing).category == crate::defs::Category::Floor
}

/// What `Command::Build` would do in each cell of the rectangle, row by
/// row. Nothing is changed: `apply` acts on exactly this list. A plan
/// placed earlier in the order covers its footprint for the cells after.
pub fn build_preview(
    w: &World,
    thing: DefId,
    stuff: Option<DefId>,
    a: IVec,
    b: IVec,
    facing: u8,
) -> Vec<(IVec, Place)> {
    let defs = &w.defs;
    let td = defs.thing(thing);
    let all = |why: Blocker| cells(w, a, b).map(|p| (p, Place::Blocked(why))).collect();
    let Some(bd) = td.build.as_ref() else { return all(Blocker::NotBuildable) };
    // A buildable that takes a material needs one, of the right kind.
    if let Some(sc) = &bd.stuff {
        if !stuff.is_some_and(|m| defs.is_material_for(m, &sc.category)) {
            return all(Blocker::NoMaterial);
        }
    }
    let floor = is_floor(w, thing);
    let facing = facing & 3;
    // Cells this order has already put a plan on, fixture layer or floor.
    let mut taken = std::collections::BTreeSet::new();
    // Whether a new plan fits with its footprint's top-left at `p`, `over`
    // being a natural thing at `p` that makes way for it.
    let fits = |p: IVec, over: Option<Entity>, taken: &std::collections::BTreeSet<usize>| -> Result<(), Blocker> {
        if floor {
            if taken.contains(&w.map.idx(p)) {
                return Err(Blocker::Overlap);
            }
            return w.map.floor_at(p).map_or(Ok(()), |f| Err(Blocker::Occupied(f)));
        }
        for c in td.footprint(p, facing) {
            if !w.map.inb(c) {
                return Err(Blocker::OutOfBounds);
            }
            if taken.contains(&w.map.idx(c)) {
                return Err(Blocker::Overlap);
            }
            match w.map.fixture_at(c) {
                Some(f) if Some(f) != over => return Err(Blocker::Occupied(f)),
                _ => {}
            }
            // The anchor is passable or holds what makes way; the rest of a
            // bigger thing's footprint must be open ground.
            if c != p && !w.map.passable(c) {
                return Err(Blocker::Terrain(c));
            }
        }
        Ok(())
    };
    // A building bigger than one cell (`World::plan_footprint`): every cell
    // of its footprint must be on the map and open, or hold a natural thing
    // it can clear that no other plan has claimed. What must be cut or
    // mined first makes it `Clears`; the cells it claims come back too.
    let big = !floor && td.size != [1, 1];
    let footprint = |p: IVec, taken: &std::collections::BTreeSet<usize>| -> (Place, Vec<usize>) {
        let clearable = |f: Entity| {
            w.ecs.get::<&Planned>(f).is_err()
                && w.ecs.get::<&Blueprint>(f).is_err()
                && w.thing(f).is_some_and(|t| {
                    let nd = defs.thing(t.def);
                    nd.natural && (!nd.blocks || nd.harvest.iter().any(|h| h.destroy))
                })
        };
        let (mut clears, mut marked, mut all) = (None, Vec::new(), Vec::new());
        for c in td.footprint(p, facing) {
            if !w.map.inb(c) {
                return (Place::Blocked(Blocker::OutOfBounds), Vec::new());
            }
            let i = w.map.idx(c);
            if taken.contains(&i) {
                return (Place::Blocked(Blocker::Overlap), Vec::new());
            }
            all.push(i);
            let ground = w.map.terrain_cost[i] > 0;
            match (w.map.fixture_at(c), w.solid_at(c).is_some()) {
                (Some(f), rock) => {
                    if !((ground || rock) && clearable(f)) {
                        return (Place::Blocked(Blocker::Occupied(f)), Vec::new());
                    }
                    // Grass makes way at once; a tree or rock is marked.
                    if w.thing(f).is_some_and(|t| defs.thing(t.def).harvest.iter().any(|h| h.destroy)) {
                        clears.get_or_insert(Target::Thing(f));
                        marked.push(i);
                    }
                }
                (None, true) => {
                    if w.solid_at(c).and_then(|s| s.thing_r).is_some() {
                        clears.get_or_insert(Target::Rock(c));
                        marked.push(i);
                    }
                }
                (None, false) if !w.map.passable(c) => return (Place::Blocked(Blocker::Terrain(c)), Vec::new()),
                (None, false) => {}
            }
        }
        // Open: the blueprint stands on the whole footprint now. Clears:
        // only the marked cells are claimed until they're cleared.
        match clears {
            None => (Place::Open, all),
            Some(t) => (Place::Clears(t), marked),
        }
    };
    let mut out = Vec::new();
    for p in cells(w, a, b) {
        if big {
            let (place, claimed) = footprint(p, &taken);
            taken.extend(claimed);
            out.push((p, place));
            continue;
        }
        // Grass, a tree or rock in the way is cleared first, not silently
        // left out: a wall with a gap is no wall. (On ground that can be
        // built on: nothing is planned over water.)
        let ground = w.map.terrain_cost[w.map.idx(p)] > 0;
        let natural: Option<Target> = if !floor && w.solid_at(p).is_some() {
            // Rock is cleared by mining it, so it stands up to be marked.
            // Rock with no thing to stand up is terrain like any other.
            match w.map.fixture_at(p) {
                Some(f) => Some(Target::Thing(f)),
                None => w.solid_at(p).and_then(|s| s.thing_r).map(|_| Target::Rock(p)),
            }
        } else if !floor {
            w.map
                .fixture_at(p)
                .filter(|&f| ground && w.thing(f).is_some_and(|t| defs.thing(t.def).natural))
                .map(Target::Thing)
        } else {
            None
        };
        let place = match natural {
            Some(target) => {
                // What stands there: a fixture, or rock still asleep.
                let (over, def) = match target {
                    Target::Rock(q) => (None, w.solid_at(q).and_then(|s| s.thing_r)),
                    Target::Thing(f) | Target::Creature(f) => (Some(f), w.thing(f).map(|t| t.def)),
                };
                match def.map(|d| defs.thing(d)) {
                    None => Place::Blocked(Blocker::Solid(p)),
                    Some(nd) if nd.harvest.iter().any(|h| h.destroy) => Place::Clears(target),
                    Some(nd) if !nd.blocks => match fits(p, over, &taken) {
                        Ok(()) => Place::Open,
                        Err(why) => Place::Blocked(why),
                    },
                    Some(_) => Place::Blocked(over.map_or(Blocker::Solid(p), Blocker::Occupied)),
                }
            }
            None if w.map.passable(p) => match fits(p, None, &taken) {
                Ok(()) => Place::Open,
                Err(why) => Place::Blocked(why),
            },
            None => Place::Blocked(match w.map.fixture_at(p) {
                Some(f) if ground => Blocker::Occupied(f),
                _ => Blocker::Terrain(p),
            }),
        };
        if place == Place::Open {
            if floor {
                taken.insert(w.map.idx(p));
            } else {
                taken.extend(td.footprint(p, facing).map(|c| w.map.idx(c)));
            }
        }
        out.push((p, place));
    }
    out
}

pub fn apply(w: &mut World, c: Command) {
    let defs = w.defs.clone();
    match c {
        Command::Designate { designation, a, b } => {
            for target in designate_preview(w, designation, a, b) {
                let e = match target {
                    // Rock that can be marked this way is stood up to take
                    // the mark; any other rock stays terrain.
                    Target::Rock(p) => match w.wake_rock(p) {
                        Some(e) => e,
                        None => continue,
                    },
                    Target::Thing(e) | Target::Creature(e) => e,
                };
                let _ = w.ecs.insert_one(e, Designated(designation));
                if let Some(t) = w.thing(e) {
                    for p in defs.thing(t.def).footprint(t.pos, t.facing) {
                        w.map.touch(p);
                    }
                }
            }
        }
        Command::Build { thing, stuff, a, b, facing } => {
            for (p, place) in build_preview(w, thing, stuff, a, b, facing) {
                realize(w, thing, stuff, p, facing, place);
            }
        }
        Command::PlacePlan { plan, at, facing, stuff } => {
            let Some(pd) = defs.plans.get(plan as usize) else { return };
            for piece in pd.placed(&defs, at, facing) {
                let stuff =
                    stuff.filter(|_| piece.stuff.is_some()).filter(|&m| stuff_fits(&defs, piece.thing, Some(m)));
                let (p, stuff) = (IVec::at(piece.at.0, piece.at.1, at.z), stuff.or(piece.stuff));
                for (q, place) in build_preview(w, piece.thing, stuff, p, p, piece.facing) {
                    realize(w, piece.thing, stuff, q, piece.facing, place);
                }
            }
        }
        Command::Cancel { a, b } => {
            let targets: Vec<Entity> =
                cells(w, a, b).flat_map(|p| [w.map.fixture_at(p), w.map.floor_at(p)]).flatten().collect();
            for f in targets {
                // Cancelling a plan over grass or a tree leaves it be.
                if !w.unplan(f) && w.ecs.remove_one::<Designated>(f).is_ok() {
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
            w.zones_changed();
        }
        Command::Stockpile { a, b, zone: Some(id) } => {
            w.zones.paint(&w.map, a, b, Some(id));
            w.zones_changed();
        }
        Command::ClearZone { a, b } => {
            w.zones.paint(&w.map, a, b, None);
            w.zones_changed();
        }
        Command::ZoneAllow { zone, thing, on } => {
            w.zones.edit(&defs, zone, FilterEdit::Thing { thing, on });
            w.zones_changed();
        }
        Command::StoreFilter { store, edit } => w.edit_store(store, edit),
        Command::StoreLevel { store, level } => w.set_store_level(store, level),
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
        Command::MarkUrgent { target, on } => {
            let changed = if on && w.is_markable(target) {
                w.ecs.insert_one(target, crate::world::Urgent).is_ok()
            } else {
                !on && w.ecs.remove_one::<crate::world::Urgent>(target).is_ok()
            };
            // The mark is drawn on the thing, in its chunk's mesh.
            if changed {
                w.touch(target);
            }
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

/// Whether `stuff` is what `thing` is built of: a material of the right
/// kind when it takes one, and none when it doesn't need one.
fn stuff_fits(defs: &crate::defs::DefDb, thing: DefId, stuff: Option<DefId>) -> bool {
    let Some(bd) = defs.thing(thing).build.as_ref() else { return false };
    match (&bd.stuff, stuff) {
        (Some(sc), Some(m)) => defs.is_material_for(m, &sc.category),
        (Some(_), None) => false,
        (None, _) => true,
    }
}

/// Carry out what `build_preview` said about `thing` at `p`: put the plan
/// up, or mark what's in its way to be cleared first. Every build goes
/// through here, so a preview and the order can't disagree.
fn realize(w: &mut World, thing: DefId, stuff: Option<DefId>, p: IVec, facing: u8, place: Place) {
    let floor = is_floor(w, thing);
    // A building bigger than a cell marks its whole footprint.
    if !floor && w.defs.thing(thing).size != [1, 1] {
        if matches!(place, Place::Open | Place::Clears(_)) {
            w.plan_footprint(thing, stuff, p, facing);
        }
        return;
    }
    match place {
        Place::Open => {
            // Grass and the like make way at once; so does rock nothing has
            // to dig, stood up to be replaced.
            let over = match (floor, w.solid_at(p).is_some()) {
                (true, _) => None,
                (false, true) => w.wake_rock(p),
                (false, false) => w.map.fixture_at(p),
            };
            match over {
                Some(f) => w.plan_over_facing(f, thing, stuff, facing),
                None => {
                    w.spawn_fixture_facing(thing, p, true, stuff, facing);
                }
            }
        }
        Place::Clears(Target::Rock(q)) => {
            if let Some(f) = w.wake_rock(q) {
                w.plan_over_facing(f, thing, stuff, facing);
            }
        }
        Place::Clears(Target::Thing(f) | Target::Creature(f)) => w.plan_over_facing(f, thing, stuff, facing),
        Place::Blocked(_) => {}
    }
}
