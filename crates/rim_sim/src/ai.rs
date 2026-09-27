//! Pawn AI: think (pick a job when idle, staggered), run the job as a small
//! state machine, then advance movement.
//!
//! While a pawn is being processed its component is swapped out for an
//! inactive placeholder, so the job code has `&mut World` freely.

use crate::defs::*;
use crate::map::CHUNK;
use crate::path::Goal;
use crate::store::StoreKey;
use crate::world::*;
use crate::IVec;
use hecs::Entity;
use std::collections::{BTreeMap, BTreeSet};

pub fn tick_pawns(w: &mut World) {
    let mut i = 0;
    while i < w.pawns.len() {
        let e = w.pawns[i];
        i += 1;
        let Some(mut p) = take(w, e) else { continue };
        if p.active && !p.dead {
            tick_pawn(w, e, &mut p);
        }
        put(w, e, p);
    }
}

fn take(w: &mut World, e: Entity) -> Option<Pawn> {
    w.ecs.get::<&mut Pawn>(e).ok().map(|mut r| std::mem::take(&mut *r))
}
fn put(w: &mut World, e: Entity, p: Pawn) {
    if let Ok(mut r) = w.ecs.get::<&mut Pawn>(e) {
        *r = p;
    }
}

/// Stop whatever the pawn is doing (used by commands like draft).
pub fn interrupt(w: &mut World, e: Entity) {
    if let Some(mut p) = take(w, e) {
        end_job(w, e, &mut p, 0);
        put(w, e, p);
    }
}

pub fn set_job(w: &mut World, e: Entity, job: Job) {
    if let Some(mut p) = take(w, e) {
        end_job(w, e, &mut p, 0);
        p.job = job;
        put(w, e, p);
    }
}

fn tick_pawn(w: &mut World, e: Entity, p: &mut Pawn) {
    if p.cooldown > 0 {
        p.cooldown -= 1;
    }
    if matches!(p.job, Job::Idle) && w.tick >= p.next_think {
        let job = match p.faction {
            Faction::Player => think_colonist(w, e, p),
            Faction::Hostile => think_hostile(w, e, p),
            Faction::Wild => think_animal(w, e, p),
        };
        match job {
            Some(j) => p.job = j,
            None => p.next_think = w.tick + 30 + (e.id() % 13) as u64,
        }
    }
    run_job(w, e, p);
    advance_movement(w, p);
}

fn end_job(w: &mut World, e: Entity, p: &mut Pawn, delay: u64) {
    w.release_all(e);
    if let Some(lot) = p.carry.take() {
        w.place_lot(lot, p.pos);
    }
    p.job = Job::Idle;
    p.path.clear();
    p.path_goal = None;
    p.asleep = false;
    p.next_think = w.tick + delay + (e.id() % 7) as u64;
}

// ================================================================ movement

enum Go {
    Arrived,
    Moving,
    Failed,
}

fn go_to(w: &mut World, p: &mut Pawn, goal: Goal) -> Go {
    if p.next.is_none() && goal.satisfied(p.pos) {
        p.path.clear();
        p.path_goal = None;
        return Go::Arrived;
    }
    if p.path_goal == Some(goal) && p.moving() {
        return Go::Moving;
    }
    if p.next.is_some() {
        // Finish the current step, then replan.
        p.path.clear();
        p.path_goal = None;
        return Go::Moving;
    }
    w.map.ensure_regions();
    if !w.map.can_reach_for(p.pos, goal, p.faction) {
        return Go::Failed;
    }
    // The regions say a path exists, so the search is sure to find one; a
    // cap below the whole map would only give up on a long way round and
    // have the pawn ask again, and again.
    let cap = (w.map.w * w.map.h) as u32;
    match w.pf.find(&w.map, p.pos, goal, cap, p.faction) {
        Some(path) => {
            p.path = path;
            p.path_goal = Some(goal);
            Go::Moving
        }
        None => Go::Failed,
    }
}

fn advance_movement(w: &mut World, p: &mut Pawn) {
    if let Some(n) = p.next {
        p.progress += 1;
        if p.progress < p.step_ticks {
            return;
        }
        p.pos = n;
        p.next = None;
        p.progress = 0;
    }
    if let Some(&n) = p.path.last() {
        if !w.map.passable(n) {
            p.path.clear();
            p.path_goal = None;
            return;
        }
        p.path.pop();
        let diag = n.x != p.pos.x && n.y != p.pos.y;
        let speed = w.defs.creature(p.def).speed;
        p.step_ticks = (speed * w.map.cost(n) / 100 * if diag { 14 } else { 10 } / 10).max(1);
        p.next = Some(n);
        p.progress = 0;
    }
}

// ================================================================ thinking

/// How far colonists will go to fight a hostile. Raiders pick off anyone who
/// fights alone, so this is deliberately wider than sight range.
const DEFEND_RADIUS: i32 = 20;

fn think_colonist(w: &mut World, e: Entity, p: &mut Pawn) -> Option<Job> {
    let defs = w.defs.clone();
    if p.drafted {
        // Drafted pawns hold position but defend themselves.
        let (t, _) = nearest_pawn(w, e, p.pos, 1, |o| o.faction == Faction::Hostile)?;
        return Some(Job::Attack { target: t, until: w.tick + 600 });
    }
    if wounded(&defs, p) {
        // Badly hurt: get away from danger, then eat and sleep to heal.
        let attacker = p.last_attacker.take().and_then(|a| w.pawn_pos(a));
        let threat = attacker.or_else(|| nearest_pawn(w, e, p.pos, 10, |o| o.faction == Faction::Hostile).map(|t| t.1));
        if let Some(tp) = threat {
            return flee(w, p, tp);
        }
    } else {
        if let Some(a) = p.last_attacker.take() {
            if w.pawn_pos(a).is_some_and(|ap| ap.chebyshev(p.pos) <= 12) {
                return Some(Job::Attack { target: a, until: w.tick + 900 });
            }
        }
        // Colonists rally: anyone within DEFEND_RADIUS of a hostile joins the fight.
        if let Some((t, tp)) =
            nearest_pawn(w, e, p.pos, DEFEND_RADIUS, |o| o.faction == Faction::Hostile && !out_of_fight(&defs, o))
        {
            if reachable(w, p.pos, Goal::Touch(tp)) {
                return Some(Job::Attack { target: t, until: w.tick + 900 });
            }
        }
    }
    for i in 0..p.needs.len() {
        let (nid, v) = p.needs[i];
        let nd = defs.need(nid);
        let seek = (nd.seek_below * NEED_MAX as f64) as i32;
        match nd.satisfier {
            Satisfier::Food if v < seek => {
                if let Some(j) = find_food(w, e, p) {
                    return Some(j);
                }
            }
            Satisfier::Rest if v < seek || (w.is_night() && v < NEED_MAX * 6 / 10) => {
                return Some(find_bed(w, e, p));
            }
            Satisfier::Field if v < seek => {
                if let Some(to) = comfortable_spot(w, p) {
                    return Some(Job::Comfort { to, need: nid, until: w.tick + 2400 });
                }
            }
            _ => {}
        }
    }
    if let Some(j) = find_work(w, e, p) {
        return Some(j);
    }
    // Nothing to do: wait somewhere comfortable rather than out in the cold.
    let chilled = p.needs.iter().any(|&(nid, v)| defs.need(nid).satisfier == Satisfier::Field && v < NEED_MAX * 9 / 10);
    if chilled {
        if let Some(to) = comfortable_spot(w, p) {
            let need = p.needs.iter().find(|n| defs.need(n.0).satisfier == Satisfier::Field).unwrap().0;
            return Some(Job::Comfort { to, need, until: w.tick + 1200 });
        }
    }
    wander(w, p, 4)
}

fn think_hostile(w: &mut World, e: Entity, p: &mut Pawn) -> Option<Job> {
    let defs = w.defs.clone();
    if wounded(&defs, p) || p.leave_at.is_some_and(|t| w.tick >= t) {
        return leave(w, p);
    }
    if let Some((t, tp)) = nearest_pawn(w, e, p.pos, 1000, |o| o.faction == Faction::Player && !out_of_fight(&defs, o))
    {
        w.map.ensure_regions();
        if w.map.can_reach_for(p.pos, Goal::Touch(tp), p.faction) {
            return Some(Job::Attack { target: t, until: w.tick + 3000 });
        }
        // Walled out. A door is the thin part of the wall, so break that.
        if let Some(target) = nearest_breach(w, p.pos, p.faction, tp) {
            return Some(Job::Breach { target });
        }
    }
    wander(w, p, 8)
}

/// The weakest owned piece standing between `from`'s side and `to`'s side,
/// as the faction shut out sees it: the window before the door, the door
/// before the wall. A piece in the way is one whose neighbours include
/// both sides. Nothing natural qualifies -- you cannot dig through a
/// mountain -- and nothing unowned does, since it was never shut against
/// anyone. Ties go to the nearest. Call `ensure_regions` first.
fn nearest_breach(w: &World, from: IVec, who: Faction, to: IVec) -> Option<Entity> {
    let (mine, theirs) = (w.map.region_at_for(from, who), w.map.region_at_for(to, who));
    if mine == 0 || theirs == 0 || mine == theirs {
        return None;
    }
    let touches = |p: IVec, r: u32| {
        crate::map::NEIGHBORS8.iter().any(|(dx, dy)| w.map.region_at_for(p.offset(*dx, *dy), who) == r)
    };
    let mut best: Option<((i32, u32, u32), Entity)> = None;
    for (de, t, owner) in w.ecs.query::<(Entity, &Thing, &Owner)>().without::<&Blueprint>().iter() {
        if owner.0 == who || !w.map.blocks_fields(w.map.idx(t.pos)) {
            continue;
        }
        let key = (t.hp, t.pos.octile(from), de.id());
        if best.is_some_and(|b| b.0 <= key) {
            continue;
        }
        if touches(t.pos, mine) && touches(t.pos, theirs) && w.map.can_reach_for(from, w.reach_goal(t), who) {
            best = Some((key, de));
        }
    }
    best.map(|b| b.1)
}

fn think_animal(w: &mut World, e: Entity, p: &mut Pawn) -> Option<Job> {
    let defs = w.defs.clone();
    let cd = defs.creature(p.def);
    if let Some(a) = p.last_attacker.take() {
        if let Some(ap) = w.pawn_pos(a) {
            if cd.flees || wounded(&defs, p) {
                return flee(w, p, ap);
            }
            return Some(Job::Attack { target: a, until: w.tick + 900 });
        }
    }
    if cd.aggressive {
        if let Some((t, tp)) =
            nearest_pawn(w, e, p.pos, 8, |o| defs.creature(o.def).intelligent && !out_of_fight(&defs, o))
        {
            if reachable(w, p.pos, Goal::Touch(tp)) {
                return Some(Job::Attack { target: t, until: w.tick + 600 });
            }
        }
    }
    wander(w, p, 6)
}

/// Running from a fight. Nobody picks a retreating creature as a target or
/// chases one down; retreat has to work the same for both sides.
fn retreating(p: &Pawn) -> bool {
    matches!(p.job, Job::Flee { .. } | Job::Leave { .. })
}

/// Out of the fight: retreating, or hurt badly enough to retreat. Never
/// chosen as a new target, though anyone adjacent can still land a blow.
fn out_of_fight(defs: &DefDb, p: &Pawn) -> bool {
    retreating(p) || wounded(defs, p)
}

/// Below the creature's `retreat_below` fraction of max hp.
fn wounded(defs: &DefDb, p: &Pawn) -> bool {
    let cd = defs.creature(p.def);
    cd.retreat_below > 0.0 && (p.hp as f64) < cd.max_hp as f64 * cd.retreat_below
}

/// Head for the nearest reachable map edge.
fn leave(w: &mut World, p: &mut Pawn) -> Option<Job> {
    w.map.ensure_regions();
    let (mw, mh) = (w.map.w, w.map.h);
    let mut edges =
        [IVec::new(0, p.pos.y), IVec::new(mw - 1, p.pos.y), IVec::new(p.pos.x, 0), IVec::new(p.pos.x, mh - 1)];
    edges.sort_by_key(|q| q.octile(p.pos));
    for q in edges {
        // Slide along the edge until we find somewhere we can actually reach.
        for k in 0..mw.max(mh) {
            for s in [k, -k] {
                let c = if q.x == 0 || q.x == mw - 1 { q.offset(0, s) } else { q.offset(s, 0) };
                if w.map.passable(c) && w.map.can_reach(p.pos, Goal::Cell(c)) {
                    return Some(Job::Leave { to: c });
                }
            }
        }
    }
    None
}

fn reachable(w: &mut World, from: IVec, goal: Goal) -> bool {
    w.map.ensure_regions();
    w.map.can_reach(from, goal)
}

fn nearest_pawn(
    w: &World,
    me: Entity,
    from: IVec,
    radius: i32,
    filter: impl Fn(&Pawn) -> bool,
) -> Option<(Entity, IVec)> {
    let mut best: Option<(u32, Entity, IVec)> = None;
    for &o in &w.pawns {
        if o == me {
            continue;
        }
        let Ok(op) = w.ecs.get::<&Pawn>(o) else { continue };
        if !op.active || op.dead || op.pos.chebyshev(from) > radius || !filter(&op) {
            continue;
        }
        let d = op.pos.octile(from);
        if best.is_none_or(|b| d < b.0) {
            best = Some((d, o, op.pos));
        }
    }
    best.map(|b| (b.1, b.2))
}

fn wander(w: &mut World, p: &mut Pawn, r: i32) -> Option<Job> {
    for _ in 0..8 {
        let to = p.pos.offset(w.rng.range(-r, r), w.rng.range(-r, r));
        if w.map.passable(to) && reachable(w, p.pos, Goal::Cell(to)) {
            return Some(Job::Wander { to, until: w.tick + 600 });
        }
    }
    None
}

fn flee(w: &mut World, p: &mut Pawn, from: IVec) -> Option<Job> {
    let dx = (p.pos.x - from.x).signum();
    let dy = (p.pos.y - from.y).signum();
    for _ in 0..10 {
        let to = p.pos.offset(dx * 10 + w.rng.range(-4, 4), dy * 10 + w.rng.range(-4, 4));
        if w.map.passable(to) && reachable(w, p.pos, Goal::Cell(to)) {
            return Some(Job::Flee { to, until: w.tick + 400 });
        }
    }
    wander(w, p, 8)
}

// ================================================================ finding work

/// Why a designated thing's work isn't being done, in the player's words,
/// or `None` when nothing stands in the way. The inspector shows it.
/// Reachability reads the regions as of the last tick: a wall finished
/// this tick counts from the next.
pub fn work_blocked(w: &World, e: Entity) -> Option<String> {
    let t = w.thing(e)?;
    // A plan waits only on a tool here; its materials say for themselves.
    let d = match w.ecs.get::<&Designated>(e).ok().map(|d| d.0) {
        Some(d) => Some(d),
        None if w.ecs.get::<&Blueprint>(e).is_ok() => None,
        None => return None,
    };
    if let Some(d) = d.filter(|&d| w.defs.designations[d as usize].targets == Targets::Thing) {
        let h = w.defs.thing(t.def).harvest_for(d)?;
        if !w.harvest_ready(e, h.key()) {
            return Some("Growing back.".into());
        }
    }
    let need = match d {
        Some(d) => w.defs.thing(t.def).harvest_for(d).map_or(0, |h| h.requires_r),
        None => w.defs.thing(t.def).build.as_ref().map_or(0, |b| b.requires_r),
    };
    if d.is_none() && need == 0 {
        return None;
    }
    // Drafted colonists take no work.
    let free: Vec<Entity> =
        w.colonists().filter(|&c| w.ecs.get::<&Pawn>(c).is_ok_and(|p| !p.drafted && p.active)).collect();
    if free.is_empty() && w.colonists().next().is_some() {
        return Some("Everyone is drafted.".into());
    }
    let reachable = free.iter().filter_map(|&c| w.pawn_pos(c)).any(|p| w.map.can_reach(p, w.reach_goal(&t)));
    if !reachable {
        return Some("No colonist can reach it.".into());
    }
    if need == 0 {
        return None;
    }
    let tags = |m: ToolMask| w.defs.tool_tag_names(m).join(" and ");
    let missing = need & !w.colony_tools();
    if missing != 0 {
        return Some(format!("Needs a {} tool.", tags(missing)));
    }
    // The colony has one, but it's in other hands, claimed or out of reach.
    let can = free.iter().any(|&c| {
        let Ok(p) = w.ecs.get::<&Pawn>(c) else { return false };
        w.hand_covers(&p, need) || w.nearest_tool(c, p.pos, need).is_some()
    });
    (!can).then(|| format!("Needs a free {} tool.", tags(need)))
}

/// What pawn `e` must do to hold a tool covering `need`, given the tags
/// the colony's tools cover (`have`): nothing (`Some((0, None))`), walk
/// this far to fetch one (`Some((d, Some(tool)))`), or it can't (`None`).
/// A tag nobody has is refused with a mask test, before looking for tools.
pub(crate) fn tool_for(
    w: &World,
    e: Entity,
    p: &Pawn,
    need: ToolMask,
    have: ToolMask,
) -> Option<(u32, Option<Entity>)> {
    if w.hand_covers(p, need) {
        return Some((0, None));
    }
    if have & need != need {
        return None;
    }
    w.nearest_tool(e, p.pos, need).map(|(d, t)| (d, Some(t)))
}

fn find_food(w: &mut World, e: Entity, p: &Pawn) -> Option<Job> {
    let defs = w.defs.clone();
    w.map.ensure_regions();
    // (distance, id), the thing, and the harvest that yields food when it
    // isn't food itself: Some(key).
    let mut best: Option<((u32, u32), Entity, Option<HarvestKey>)> = None;
    for (te, t) in w.ecs.query::<(Entity, &Thing)>().without::<&Blueprint>().iter() {
        let td = defs.thing(t.def);
        let is_item = td.category == Category::Item && td.food.is_some();
        let forage = (!is_item)
            .then(|| {
                td.harvest.iter().find(|h| {
                    h.yields_r.iter().any(|y| defs.thing(y.0).food.is_some())
                        && w.harvest_ready(te, h.key())
                        && w.hand_covers(p, h.requires_r)
                })
            })
            .flatten();
        let is_plant = forage.is_some();
        if !is_item && !is_plant {
            continue;
        }
        // Prefer ready food over foraging.
        let d = (t.pos.octile(p.pos) + if is_plant { 150 } else { 0 }, te.id());
        if best.is_some_and(|b| b.0 <= d) || w.reserved_by_other(te, e) {
            continue;
        }
        let goal = if is_item { w.stack_goal(te, t.pos) } else { w.reach_goal(t) };
        if !w.map.can_reach(p.pos, goal) {
            continue;
        }
        best = Some((d, te, forage.map(|h| h.key())));
    }
    let (_, t, forage) = best?;
    w.reserve(t, e);
    if let Some(harvest) = forage {
        return Some(Job::Harvest { target: t, forced: true, harvest, tool: None });
    }
    // Somewhere to sit and eat it, if the colony has such a thing.
    let food_at = w.thing(t).map_or(p.pos, |f| f.pos);
    let seat = nearest_spot(w, e, food_at, |td| td.food.is_none() && td.bed.is_none());
    if let Some((se, _)) = seat {
        w.reserve(se, e);
    }
    Some(Job::Eat { src: t, t: 0, seat, stage: 0 })
}

/// The cells around `thing` that a pawn may use it from, as its def lays
/// them out, keeping only those whose `beside` requirement is met.
fn spots_of(w: &World, thing: Entity) -> Vec<IVec> {
    let Some(t) = w.thing(thing) else { return Vec::new() };
    let td = w.defs.thing(t.def);
    td.spots
        .iter()
        .map(|s| (t.pos.offset(s.dx, s.dy), s))
        .filter(|(cell, s)| s.beside.is_empty() || beside(w, *cell, &s.beside))
        .map(|(cell, _)| cell)
        .collect()
}

/// Is there a thing tagged `tag` in one of the eight cells around `cell`?
fn beside(w: &World, cell: IVec, tag: &str) -> bool {
    crate::map::NEIGHBORS8.iter().any(|(dx, dy)| {
        w.map
            .fixture_at(cell.offset(*dx, *dy))
            .and_then(|f| w.thing(f))
            .is_some_and(|t| w.defs.thing(t.def).tags.iter().any(|g| g == tag))
    })
}

/// The nearest free, reachable spot on a thing `pick` accepts, from `from`.
/// A thing is one reservation: two pawns never share it.
fn nearest_spot(w: &World, e: Entity, from: IVec, pick: impl Fn(&ThingDef) -> bool) -> Option<(Entity, IVec)> {
    let mut best: Option<((u32, u32), Entity, IVec)> = None;
    for (te, t) in w.ecs.query::<(Entity, &Thing)>().without::<&Blueprint>().iter() {
        let td = w.defs.thing(t.def);
        if td.spots.is_empty() || !pick(td) || w.reserved_by_other(te, e) {
            continue;
        }
        for cell in spots_of(w, te) {
            let d = (cell.octile(from), te.id());
            if best.is_some_and(|b| b.0 <= d) {
                continue;
            }
            if w.map.passable(cell) && w.map.can_reach(from, Goal::Cell(cell)) {
                best = Some((d, te, cell));
            }
        }
    }
    best.map(|b| (b.1, b.2))
}

fn find_bed(w: &mut World, e: Entity, p: &mut Pawn) -> Job {
    let defs = w.defs.clone();
    w.map.ensure_regions();
    let _ = &defs;
    let found = nearest_spot(w, e, p.pos, |td| td.bed.is_some());
    if let Some((b, _)) = found {
        w.reserve(b, e);
    }
    // No bed: sleep somewhere comfortable if the ground here isn't.
    let (bed, spot) = match found {
        Some((b, cell)) => (Some(b), cell),
        None => (None, comfortable_spot(w, p).unwrap_or(p.pos)),
    };
    Job::Sleep { bed, spot, stage: 0 }
}

/// How far a pawn will look for somewhere comfortable, in cells explored.
const COMFORT_SEARCH: usize = 4000;

/// How long a pawn that found nowhere better waits before looking again:
/// an hour. The land doesn't warm up by the minute, and the look is the
/// widest search a pawn makes.
const COMFORT_RETRY: u64 = crate::TICKS_PER_DAY / 24;

/// How much better a spot must be than where the pawn stands (in field units
/// outside comfort) before it's worth walking to.
const COMFORT_GAIN: f64 = 2.0;

/// Nearest cell, by walking, where every field need of this pawn is inside
/// its comfort range; failing that, the least uncomfortable cell within
/// reach, if it's clearly better than here (an unheated hut beats the night
/// outside). `None` if the pawn is already comfortable or nothing nearby is
/// better.
pub fn comfortable_spot(w: &mut World, p: &mut Pawn) -> Option<IVec> {
    if w.tick < p.comfort_after {
        return None;
    }
    let defs = &w.defs;
    let needs: Vec<&NeedDef> =
        p.needs.iter().map(|n| defs.need(n.0)).filter(|nd| nd.satisfier == Satisfier::Field).collect();
    if needs.is_empty() {
        return None;
    }
    // How far outside comfort a cell is, summed over the pawn's field needs.
    // Aim one unit inside the range so the pawn isn't standing on the edge.
    let off = |c: IVec| -> f64 {
        needs
            .iter()
            .map(|nd| {
                let v = w.fields.value(defs, &w.map, nd.field_r as usize, c);
                (nd.comfort[0] + 1.0 - v).max(v - (nd.comfort[1] - 1.0)).max(0.0)
            })
            .sum()
    };
    let here = off(p.pos);
    if here == 0.0 {
        return None;
    }
    let mut best = (here, p.pos);
    let comfortable = w.pf.flood(&w.map, p.pos, COMFORT_SEARCH, |c| {
        let o = off(c);
        if o < best.0 {
            best = (o, c);
        }
        (o == 0.0).then_some(c)
    });
    let spot = comfortable.or((best.0 + COMFORT_GAIN < here).then_some(best.1));
    if spot.is_none() {
        p.comfort_after = w.tick + COMFORT_RETRY;
    }
    spot
}

/// The work a colonist should do next (DESIGN.md §4d): of the work types
/// not at 0 once the priority rules have had their say, those at its lowest priority level that have reachable
/// work, and of those the nearest job; `order` breaks a tie. Work comes from
/// blueprints (the work type that covers "build"), designated things and
/// designated creatures, each designation naming its work type, except that
/// clearing a thing a building is planned over is building.
fn find_work(w: &mut World, e: Entity, p: &Pawn) -> Option<Job> {
    w.map.ensure_regions();
    let (_, job, res) = choose_work(w, e, p, None)?;
    w.reserve(res, e);
    if let Job::Deliver { src, .. }
    | Job::Supply { src, .. }
    | Job::Harvest { tool: Some(src), .. }
    | Job::Craft { tool: Some(src), .. } = job
    {
        w.reserve(src, e);
    }
    Some(job)
}

/// Added to the distance of work that isn't urgent, so any urgent job at a
/// level comes first; no walk on a map comes near it.
const CALM: u32 = 1 << 24;

/// Why a kind of work was or wasn't taken: the why panel (DESIGN.md §4d).
#[derive(Clone, Debug)]
pub enum Why {
    /// This is the work it picked.
    Picked(Job),
    /// Set to never, by the player or a rule.
    Never,
    /// None of it is waiting.
    Nothing,
    /// The nearest is someone else's.
    Reserved,
    /// None of it can be reached.
    Unreachable,
    /// A job needs something brought and none can be found: what.
    NoMaterials(Option<DefId>),
    /// A job needs a tool with these tags, and none is free to hold.
    NeedsTool(ToolMask),
    /// Work of this type is there, but this won: a better level, or the
    /// same level and nearer.
    Beaten(DefId),
}

/// One work type's outcome for one colonist, with its effective level
/// and, where there was a job, how far.
#[derive(Clone, Debug)]
pub struct WorkWhy {
    pub work: DefId,
    pub level: u8,
    pub why: Why,
    pub dist: Option<u32>,
}

/// The nearest refusal of each work type while choosing: (distance, why).
struct Refusals(Vec<Option<(u32, Why)>>);

impl Refusals {
    fn note(&mut self, t: DefId, d: u32, why: Why) {
        let slot = &mut self.0[t as usize];
        if slot.as_ref().is_none_or(|s| d < s.0) {
            *slot = Some((d, why));
        }
    }
}

/// Why pawn `e` would take the work it would, and passes over the rest,
/// work type by work type in tie-break order. The same choice find_work
/// makes, with the refusals kept: it runs only for someone inspecting.
pub fn explain_work(w: &World, e: Entity) -> Vec<WorkWhy> {
    let Ok(p) = w.ecs.get::<&Pawn>(e).map(|p| (*p).clone()) else { return Vec::new() };
    let defs = &w.defs;
    let mut refused = Refusals(vec![None; defs.work_types.len()]);
    let chosen = choose_work(w, e, &p, Some(&mut refused));
    defs.work_order
        .iter()
        .map(|&t| {
            let level = crate::rules::effective(w, &p, t);
            let (why, dist) = match (&chosen, refused.0[t as usize].take()) {
                _ if level == 0 => (Why::Never, None),
                (Some((c, job, _)), _) if *c == t => (Why::Picked(job.clone()), None),
                // Hauling unsearched because a better level had work: it
                // lost to whatever won.
                (_, Some((u32::MAX, Why::Beaten(_)))) => (Why::Beaten(chosen.as_ref().map_or(t, |c| c.0)), None),
                (_, Some((d, why))) => (why, Some(d)),
                _ => (Why::Nothing, None),
            };
            WorkWhy { work: t, level, why, dist }
        })
        .collect()
}

/// Who would take the job on `target` next, soonest first, and in about
/// how many ticks they'd be there: the colonists free to choose (idle, or
/// wandering until their wander is up) whose own choice it is, in the
/// order they'd next think (spawn order within a tick). Busy colonists
/// aren't asked: when they'll be free is anyone's guess, and asking costs
/// a search each. A job someone already holds has nobody next: ask
/// `World::reservations` who's on it. Needs come first for a colonist,
/// and this doesn't ask them.
pub fn who_takes(w: &World, target: Entity) -> Vec<(Entity, u64)> {
    if w.reservations.contains_key(&target) {
        return Vec::new();
    }
    let mut line: Vec<(u64, usize, Entity, u64)> = Vec::new();
    for (i, &c) in w.pawns.iter().enumerate() {
        let Ok(p) = w.ecs.get::<&Pawn>(c).map(|p| (*p).clone()) else { continue };
        if p.faction != Faction::Player || p.drafted || !p.active || p.dead {
            continue;
        }
        let think = match p.job {
            Job::Idle => p.next_think,
            Job::Wander { until, .. } => until.max(p.next_think),
            _ => continue,
        };
        let Some((_, _, res)) = choose_work(w, c, &p, None) else { continue };
        if res != target {
            continue;
        }
        let there = w.thing(target).map(|t| t.pos).or_else(|| w.pawn_pos(target)).unwrap_or(p.pos);
        let think = think.max(w.tick);
        // A cell takes about the creature's speed in ticks to cross.
        let walk = there.octile(p.pos) as u64 * w.defs.creature(p.def).speed as u64 / 10;
        line.push((think, i, c, think - w.tick + walk));
    }
    line.sort_unstable_by_key(|x| (x.0, x.1));
    line.into_iter().map(|x| (x.2, x.3)).collect()
}

/// The work pawn `e` would take next, and of which type: the choice,
/// without reserving anything. With `why`, each refusal is kept.
fn choose_work(w: &World, e: Entity, p: &Pawn, mut why: Option<&mut Refusals>) -> Option<(DefId, Job, Entity)> {
    let defs = w.defs.clone();
    let level: Vec<u8> = (0..defs.work_types.len() as DefId).map(|t| crate::rules::effective(w, p, t)).collect();
    let wanted = |t: DefId| level[t as usize] > 0;
    // Within a level, urgent work comes before calm work, and then the
    // nearest (DESIGN.md §4d): the key is the distance, plus CALM for work
    // that isn't urgent. Urgent: raising a shelter (walls, doors, a bed)
    // while the colony has none, and a comfort (a fire) while it has none,
    // and clearing the ground for either.
    let (shelterless, comfortless) = (std::cell::OnceCell::new(), std::cell::OnceCell::new());
    let urgent_build = |thing: DefId| {
        let td = defs.thing(thing);
        ((td.blocks || td.door || td.bed.is_some()) && *shelterless.get_or_init(|| !w.has_shelter()))
            || (td.comforts && *comfortless.get_or_init(|| !w.has_comfort()))
    };
    let key = |d: u32, urgent: bool| if urgent { d } else { d.saturating_add(CALM) };
    // The best job of each work type: (key, job, what to reserve).
    let mut best: Vec<Option<(u32, Job, Entity)>> = vec![None; defs.work_types.len()];
    let nearer =
        |b: &Option<(u32, Job, Entity)>, d: u32, r: Entity| b.as_ref().is_none_or(|b| (d, r.id()) < (b.0, b.2.id()));

    // Blueprints, nearest first; stop at the first that yields a job.
    if let Some(bw) = defs.build_work.filter(|&t| wanted(t)) {
        /// (key, blueprint, where to stand, first missing material and how many)
        type Candidate = (u32, Entity, Goal, Option<(DefId, u32)>);
        let mut bps: Vec<Candidate> = Vec::new();
        for (be, t, bp) in w.ecs.query::<(Entity, &Thing, &Blueprint)>().iter() {
            if w.reserved_by_other(be, e) {
                if let Some(r) = why.as_deref_mut() {
                    r.note(bw, t.pos.octile(p.pos), Why::Reserved);
                }
                continue;
            }
            let missing = bp.cost.iter().zip(&bp.delivered).find(|(c, d)| **d < c.1).map(|(c, d)| (c.0, c.1 - d));
            bps.push((key(t.pos.octile(p.pos), urgent_build(t.def)), be, w.reach_goal(t), missing));
        }
        bps.sort_by_key(|b| (b.0, b.1.id()));
        // A material with nothing to bring for one plan has nothing for
        // the next: asked once, not once per plan across the whole map.
        let mut none_of: Vec<DefId> = Vec::new();
        for (k, be, goal, missing) in bps {
            let d = k % CALM;
            if !w.map.can_reach(p.pos, goal) {
                if let Some(r) = why.as_deref_mut() {
                    r.note(bw, d, Why::Unreachable);
                }
                continue;
            }
            match missing {
                None => {
                    // A build that needs a tool fetches it first; the walk
                    // there counts, as a material's does.
                    let need = w.thing(be).and_then(|t| defs.thing(t.def).build.as_ref().map(|b| b.requires_r));
                    match tool_for(w, e, p, need.unwrap_or(0), w.colony_tools()) {
                        Some((extra, tool)) => {
                            best[bw as usize] = Some((k + extra, Job::Construct { bp: be, tool }, be));
                            break;
                        }
                        None => {
                            if let Some(r) = why.as_deref_mut() {
                                r.note(bw, d, Why::NeedsTool(need.unwrap_or(0)));
                            }
                        }
                    }
                }
                Some((mdef, want)) => {
                    if !none_of.contains(&mdef) {
                        if let Some((sd, src)) = nearest_item(w, e, p.pos, mdef) {
                            best[bw as usize] = Some((k + sd, Job::Deliver { bp: be, src, want, stage: 0 }, be));
                            break;
                        }
                        none_of.push(mdef);
                    }
                    if let Some(r) = why.as_deref_mut() {
                        r.note(bw, d, Why::NoMaterials(Some(mdef)));
                    }
                }
            }
        }
    }

    // Designated fixtures: harvest the natural ones, take down the built ones.
    // Gated harvests need a tool: what the colony's tools cover is worked
    // out once, so one nobody could do costs a mask test.
    let have = w.colony_tools();
    for (te, t, des) in w.ecs.query::<(Entity, &Thing, &Designated)>().without::<&Blueprint>().iter() {
        let dd = &defs.designations[des.0 as usize];
        let wt = designated_work(w, te, dd.work_r);
        let d = t.pos.octile(p.pos);
        let urgent = w.ecs.get::<&Planned>(te).is_ok_and(|pl| urgent_build(pl.thing));
        let k = key(d, urgent);
        if !wanted(wt) || !nearer(&best[wt as usize], k, te) {
            continue;
        }
        if w.reserved_by_other(te, e) {
            if let Some(r) = why.as_deref_mut() {
                r.note(wt, d, Why::Reserved);
            }
            continue;
        }
        // The walk to fetch a tool counts, as the walk to a material does.
        let (job, detour) = match dd.targets {
            Targets::Built => (Job::Deconstruct { target: te }, 0),
            _ => match defs.thing(t.def).harvest_for(des.0) {
                Some(h) if w.harvest_ready(te, h.key()) => match tool_for(w, e, p, h.requires_r, have) {
                    Some((extra, tool)) => (Job::Harvest { target: te, forced: false, harvest: h.key(), tool }, extra),
                    None => {
                        if let Some(r) = why.as_deref_mut() {
                            r.note(wt, d, Why::NeedsTool(h.requires_r));
                        }
                        continue;
                    }
                },
                _ => continue,
            },
        };
        let nearest = detour == 0 || nearer(&best[wt as usize], k + detour, te);
        if nearest && w.map.can_reach(p.pos, w.reach_goal(t)) {
            best[wt as usize] = Some((k + detour, job, te));
        } else if nearest {
            if let Some(r) = why.as_deref_mut() {
                r.note(wt, d, Why::Unreachable);
            }
        }
    }

    // Work orders: bring what's missing, or work one that has it all.
    for (se, t, o) in w.ecs.query::<(Entity, &Thing, &Order)>().without::<&Blueprint>().iter() {
        let wt = o.work_type;
        let d = t.pos.octile(p.pos);
        if !wanted(wt) || !nearer(&best[wt as usize], key(d, false), se) {
            continue;
        }
        let reserved = w.reserved_by_other(se, e);
        if reserved || !w.map.can_reach(p.pos, w.reach_goal(t)) {
            if let Some(r) = why.as_deref_mut() {
                r.note(wt, d, if reserved { Why::Reserved } else { Why::Unreachable });
            }
            continue;
        }
        let job = match o.missing() {
            Some((i, want)) => {
                let need = &o.needs[i];
                nearest_item_where(w, e, p.pos, |def| need.takes(&defs, def))
                    .map(|(sd, src)| (sd + d, Job::Supply { site: se, src, need: i as u8, want, stage: 0 }))
            }
            None => defs
                .tool_mask(&o.requires)
                .and_then(|need| tool_for(w, e, p, need, have))
                .map(|(extra, tool)| (d + extra, Job::Craft { site: se, tool })),
        };
        match job.map(|(dist, job)| (key(dist, false), job)) {
            Some((k, job)) if nearer(&best[wt as usize], k, se) => best[wt as usize] = Some((k, job, se)),
            Some(_) => {}
            None => {
                if let Some(r) = why.as_deref_mut() {
                    let refusal = match o.missing() {
                        Some(_) => Why::NoMaterials(None),
                        None => Why::NeedsTool(defs.tool_mask(&o.requires).unwrap_or(0)),
                    };
                    r.note(wt, d, refusal);
                }
            }
        }
    }

    // Loose items a stockpile would take, unless work at a better level
    // was already found: hauling is the costliest search.
    if let Some(hw) = defs.haul_work.filter(|&t| wanted(t)) {
        let beaten = best.iter().enumerate().find(|(t, b)| b.is_some() && level[*t] < level[hw as usize]);
        // Not searched: the costliest search, and it couldn't win.
        if let (Some((t, _)), Some(r)) = (beaten, why.as_deref_mut()) {
            if !w.zones.list.is_empty() {
                r.note(hw, u32::MAX, Why::Beaten(t as DefId));
            }
        }
        if beaten.is_none() {
            if let Some((d, job, src)) = find_haul(w, e, p.pos) {
                if nearer(&best[hw as usize], key(d, false), src) {
                    best[hw as usize] = Some((key(d, false), job, src));
                }
            }
        }
    }

    // Designated creatures (hunt).
    for &o in &w.pawns {
        let Ok(des) = w.ecs.get::<&Designated>(o).map(|d| *d) else { continue };
        let wt = defs.designations[des.0 as usize].work_r;
        if !wanted(wt) {
            continue;
        }
        let Some(op) = w.pawn_pos(o) else { continue };
        let d = op.octile(p.pos);
        if w.reserved_by_other(o, e) {
            if let Some(r) = why.as_deref_mut() {
                r.note(wt, d, Why::Reserved);
            }
            continue;
        }
        if nearer(&best[wt as usize], key(d, false), o) {
            if w.map.can_reach(p.pos, Goal::Touch(op)) {
                best[wt as usize] = Some((key(d, false), Job::Attack { target: o, until: w.tick + 2400 }, o));
            } else if let Some(r) = why.as_deref_mut() {
                r.note(wt, d, Why::Unreachable);
            }
        }
    }

    let rank = |t: DefId| defs.work_order.iter().position(|&o| o == t).unwrap_or(usize::MAX);
    let chosen = best
        .iter()
        .enumerate()
        .filter_map(|(t, b)| Some((t as DefId, b.as_ref()?)))
        .min_by_key(|(t, b)| (level[*t as usize], b.0, rank(*t), b.2.id()))
        .map(|(t, _)| t)?;
    // Every other type that had a job lost to this one.
    if let Some(r) = why {
        for (t, b) in best.iter().enumerate().filter(|(t, _)| *t as DefId != chosen) {
            if let Some(b) = b {
                r.0[t] = Some((b.0 % CALM, Why::Beaten(chosen)));
            }
        }
    }
    let (_, job, res) = best.swap_remove(chosen as usize)?;
    Some((chosen, job, res))
}

/// The work type a designated thing's job counts under: its designation's,
/// except that clearing the ground for a building planned over it is
/// building, at the build priority rather than the gathering one.
fn designated_work(w: &World, e: Entity, designation_work: DefId) -> DefId {
    match w.ecs.get::<&Planned>(e) {
        Ok(_) => w.defs.build_work.unwrap_or(designation_work),
        Err(_) => designation_work,
    }
}

/// How many jobs of each work type are waiting, from the same sources
/// `find_work` takes them from: blueprints, designations whose work is
/// ready, work orders, loose items a stockpile would take, and designated
/// creatures. Reach and reservations aren't counted: this is what's there
/// to do, not who can do it. For the Work Board's column headers.
pub fn work_waiting(w: &World) -> Vec<u32> {
    let defs = &w.defs;
    let mut n = vec![0u32; defs.work_types.len()];
    if let Some(bw) = defs.build_work {
        n[bw as usize] += w.ecs.query::<&Blueprint>().iter().count() as u32;
    }
    for (te, t, des) in w.ecs.query::<(Entity, &Thing, &Designated)>().without::<&Blueprint>().iter() {
        let dd = &defs.designations[des.0 as usize];
        let ready = match dd.targets {
            Targets::Built => true,
            _ => defs.thing(t.def).harvest_for(des.0).is_some_and(|h| w.harvest_ready(te, h.key())),
        };
        if ready {
            n[designated_work(w, te, dd.work_r) as usize] += 1;
        }
    }
    for o in w.ecs.query::<&Order>().without::<&Blueprint>().iter() {
        n[o.work_type as usize] += 1;
    }
    if let Some(hw) = defs.haul_work.filter(|_| !w.zones.list.is_empty()) {
        // Stacks with somewhere better to be and a store with room for them.
        let bound = Bound { cells: BTreeSet::new(), units: BTreeMap::new() };
        let mut room: BTreeMap<(crate::store::Kind, i32, Option<u8>, u32), bool> = BTreeMap::new();
        for (_, set) in w.stores.unsorted() {
            for &bits in set {
                let Some(te) = Entity::from_bits(bits) else { continue };
                let (Some(s), Some(t)) = (w.stack_at(te), w.thing(te)) else { continue };
                let key = (s.kind, t.hp, s.level, w.map.region_at(t.pos));
                let has = *room
                    .entry(key)
                    .or_insert_with(|| haul_destination(w, s.kind, Some(t.hp), t.pos, None, s.level, &bound).is_some());
                if has {
                    n[hw as usize] += 1;
                }
            }
        }
    }
    for &o in &w.pawns {
        if let Ok(des) = w.ecs.get::<&Designated>(o) {
            n[defs.designations[des.0 as usize].work_r as usize] += 1;
        }
    }
    n
}

/// Where a stack should go, and why (DESIGN.md §4f).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HaulPlan {
    /// It lies in a store that keeps it, and no store higher has room.
    Stays { level: u8 },
    /// To this store, at this level: a cell of a zone, or a container
    /// standing at `to`.
    Moves { store: StoreKey, level: u8, to: IVec },
    /// Loose, or in a store that lets it go, and nowhere has room for it.
    Waits,
}

/// Where a stack on the item layer or in a container should go, and why.
pub fn haul_plan(w: &World, te: Entity) -> Option<HaulPlan> {
    let s = w.stack_at(te)?;
    let t = w.thing(te)?;
    let bound = Bound::of(w, None);
    Some(match haul_destination(w, s.kind, Some(t.hp), t.pos, None, s.level, &bound) {
        Some((store, level, to)) => HaulPlan::Moves { store, level, to: to.at },
        None => s.level.map_or(HaulPlan::Waits, |level| HaulPlan::Stays { level }),
    })
}

/// Room haulers are already bound for, so two never race for it: a zone
/// cell is one stack's room; a container's is the units on their way.
pub struct Bound {
    cells: BTreeSet<u32>,
    units: BTreeMap<u64, u32>,
}

impl Bound {
    pub fn of(w: &World, except: Option<Entity>) -> Bound {
        let mut b = Bound { cells: BTreeSet::new(), units: BTreeMap::new() };
        for &o in w.pawns.iter().filter(|&&o| Some(o) != except) {
            let Ok(p) = w.ecs.get::<&Pawn>(o) else { continue };
            let Job::Haul { src, to, into, .. } = p.job else { continue };
            match into {
                Some(c) => {
                    let n = p.carry.map(|l| l.count).or_else(|| w.thing(src).map(|t| t.count.min(CARRY_CAPACITY)));
                    *b.units.entry(c.to_bits().get()).or_default() += n.unwrap_or(0);
                }
                None if w.map.inb(to) => {
                    b.cells.insert(w.map.idx(to) as u32);
                }
                None => {}
            }
        }
        b
    }
}

/// Where a haul sets down: on a cell, or into a container standing at `at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dest {
    pub at: IVec,
    pub into: Option<Entity>,
}

/// The best place for a stack of `kind` at `hp` lying at `at`, in a store
/// above level `above` (any level for `None`): the highest level that has
/// room, then the nearest, a zone's partial stack of the same kind or its
/// open cell or a container alike. `reach` is where the walk starts, when
/// that isn't `at` (a stack in a container lies on a cell nobody stands on).
pub fn haul_destination(
    w: &World,
    kind: crate::store::Kind,
    hp: Option<i32>,
    at: IVec,
    reach: Option<IVec>,
    above: Option<u8>,
    bound: &Bound,
) -> Option<(StoreKey, u8, Dest)> {
    let reach = reach.unwrap_or(at);
    let accepts = w.stores.accepts(kind.0);
    let mut i = 0;
    while i < accepts.len() {
        let level = accepts[i].0;
        if above.is_some_and(|a| level <= a) {
            break;
        }
        // (distance, tie, store, where)
        let mut best: Option<(u32, u64, StoreKey, Dest)> = None;
        while i < accepts.len() && accepts[i].0 == level {
            let key = accepts[i].1;
            i += 1;
            match key {
                StoreKey::Zone(zone) => {
                    if !w.zones.get(zone).is_some_and(|z| z.keeps(&w.defs, kind.0, kind.1, hp)) {
                        continue;
                    }
                    for c in w.stores.partial(zone, kind).chain(w.stores.open(zone)) {
                        let p = w.map.pos(c as usize);
                        let d = p.octile(at);
                        if bound.cells.contains(&c)
                            || best.is_some_and(|b| (b.0, b.1) <= (d, c as u64))
                            || w.room_for(kind.0, kind.1, p) == 0
                            || !w.map.can_reach(reach, Goal::Cell(p))
                        {
                            continue;
                        }
                        best = Some((d, c as u64, key, Dest { at: p, into: None }));
                    }
                }
                StoreKey::Thing(bits) => {
                    let Some(c) = Entity::from_bits(bits) else { continue };
                    let Some(ct) = w.thing(c) else { continue };
                    let d = ct.pos.octile(at);
                    // After every zone cell at this distance, by id.
                    let tie = (1 << 32) | c.id() as u64;
                    let taken = bound.units.get(&bits).copied().unwrap_or(0);
                    if best.is_some_and(|b| (b.0, b.1) <= (d, tie))
                        || !w.store_keeps(c, kind.0, kind.1, hp)
                        || w.store_room(c, kind.0, kind.1) <= taken
                        || !w.map.can_reach(reach, w.reach_goal(&ct))
                    {
                        continue;
                    }
                    best = Some((d, tie, key, Dest { at: ct.pos, into: Some(c) }));
                }
            }
        }
        if let Some((_, _, key, dest)) = best {
            return Some((key, level, dest));
        }
    }
    None
}

/// The nearest stack that has somewhere better to be, and the best place
/// for it: (the walk there and on to the place, the job, the stack to
/// reserve). It looks only at the store index's unsorted stacks, in the
/// chunks nearest `from` first, and stops once no chunk left could hold a
/// nearer one.
fn find_haul(w: &World, e: Entity, from: IVec) -> Option<(u32, Job, Entity)> {
    if w.stores.unsorted_count() == 0 {
        return None;
    }
    let bound = Bound::of(w, Some(e));
    let (mw, mh) = (w.map.w, w.map.h);
    let mut order: Vec<(u32, u32)> = w
        .stores
        .unsorted()
        .map(|(c, _)| {
            let o = w.map.chunk_origin(c as usize);
            let hi = IVec::new((o.x + CHUNK).min(mw) - 1, (o.y + CHUNK).min(mh) - 1);
            (IVec::new(from.x.clamp(o.x, hi.x), from.y.clamp(o.y, hi.y)).octile(from), c)
        })
        .collect();
    order.sort_unstable();
    // Kinds found to have nowhere to go from a region, this search: with
    // every store full, that's all an idle hauler has to learn.
    let mut nowhere: BTreeSet<(crate::store::Kind, i32, Option<u8>, u32)> = BTreeSet::new();
    let mut best: Option<(u32, Entity, IVec, Dest)> = None;
    let chunks: BTreeMap<u32, &BTreeSet<u64>> = w.stores.unsorted().collect();
    for (near, c) in order {
        if best.is_some_and(|b| near > b.0) {
            break;
        }
        for &bits in chunks[&c].iter() {
            let Some(te) = Entity::from_bits(bits) else { continue };
            let Some(s) = w.stack_at(te) else { continue };
            let Some(t) = w.thing(te) else { continue };
            let d = t.pos.octile(from);
            if best.is_some_and(|b| (b.0, b.1.id()) <= (d, te.id()))
                || w.reserved_by_other(te, e)
                || !w.map.can_reach(from, w.stack_goal(te, t.pos))
            {
                continue;
            }
            let key = (s.kind, t.hp, s.level, w.map.region_at(from));
            if nowhere.contains(&key) {
                continue;
            }
            match haul_destination(w, s.kind, Some(t.hp), t.pos, Some(from), s.level, &bound) {
                Some((_, _, dest)) => best = Some((d, te, t.pos, dest)),
                None => {
                    nowhere.insert(key);
                }
            }
        }
    }
    best.map(|(d, src, at, dest)| {
        (d + dest.at.octile(at), Job::Haul { src, to: dest.at, stage: 0, into: dest.into }, src)
    })
}

/// Nearest reachable stack of `def` that nobody but `e` has claimed.
pub fn nearest_item(w: &World, e: Entity, from: IVec, def: DefId) -> Option<(u32, Entity)> {
    nearest_item_where(w, e, from, |d| d == def)
}

/// `nearest_item`, of whatever `takes` accepts: a work order's input by tag.
/// It looks only in the chunks that hold something it takes (the stock's
/// holdings), nearest first, and stops once no chunk left could hold
/// anything nearer. The same answer as walking every stack: the nearest,
/// ties to the lowest id.
pub fn nearest_item_where(w: &World, e: Entity, from: IVec, takes: impl Fn(DefId) -> bool) -> Option<(u32, Entity)> {
    let mut chunks: BTreeSet<u32> = BTreeSet::new();
    for d in 0..w.defs.things.len() as DefId {
        if w.stock.on_map(d) > 0 && takes(d) {
            chunks.extend(w.stock.chunks(d).map(|(c, _)| c));
        }
    }
    let (mw, mh) = (w.map.w, w.map.h);
    let corner = |c: u32| {
        let o = w.map.chunk_origin(c as usize);
        (o, IVec::new((o.x + CHUNK).min(mw) - 1, (o.y + CHUNK).min(mh) - 1))
    };
    let mut order: Vec<(u32, u32)> = chunks
        .into_iter()
        .map(|c| {
            let (lo, hi) = corner(c);
            (IVec::new(from.x.clamp(lo.x, hi.x), from.y.clamp(lo.y, hi.y)).octile(from), c)
        })
        .collect();
    order.sort_unstable();
    let mut best: Option<(u32, Entity)> = None;
    for (near, c) in order {
        if best.is_some_and(|b| near > b.0) {
            break;
        }
        // Stacks in the chunk's containers, beside the ones on its cells.
        for bits in w.stores.containers_in(c) {
            let Some(store) = Entity::from_bits(bits) else { continue };
            let Ok(slots) = w.ecs.get::<&Store>(store).map(|s| s.slots.clone()) else { continue };
            let Some(st) = w.thing(store) else { continue };
            for te in slots.into_iter().flatten() {
                let Ok(def) = w.ecs.get::<&Thing>(te).map(|t| t.def) else { continue };
                if !takes(def) {
                    continue;
                }
                let d = st.pos.octile(from);
                if best.is_some_and(|b| (b.0, b.1.id()) <= (d, te.id()))
                    || w.reserved_by_other(te, e)
                    || !w.map.can_reach(from, w.reach_goal(&st))
                {
                    continue;
                }
                best = Some((d, te));
            }
        }
        let (lo, hi) = corner(c);
        for y in lo.y..=hi.y {
            for x in lo.x..=hi.x {
                // On the chunk's own plane: the origin carries its level.
                let p = lo.offset(x - lo.x, y - lo.y);
                let Some(te) = w.map.item_at(p) else { continue };
                let Ok(def) = w.ecs.get::<&Thing>(te).map(|t| t.def) else { continue };
                if !takes(def) {
                    continue;
                }
                let d = p.octile(from);
                if best.is_some_and(|b| (b.0, b.1.id()) <= (d, te.id()))
                    || w.reserved_by_other(te, e)
                    || !w.map.can_reach(from, Goal::Cell(p))
                {
                    continue;
                }
                best = Some((d, te));
            }
        }
    }
    best
}

// ================================================================ running jobs

fn run_job(w: &mut World, e: Entity, p: &mut Pawn) {
    let job = std::mem::take(&mut p.job);
    let (next, delay) = match job {
        Job::Idle => return,
        j @ Job::Wander { to, until } => match go_to(w, p, Goal::Cell(to)) {
            Go::Moving if w.tick < until => (Some(j), 0),
            _ => (None, 60 + w.rng.below(120) as u64),
        },
        j @ Job::MoveTo { to } => match go_to(w, p, Goal::Cell(to)) {
            Go::Moving => (Some(j), 0),
            _ => (None, 0),
        },
        j @ Job::Leave { to } => match go_to(w, p, Goal::Cell(to)) {
            Go::Moving => (Some(j), 0),
            Go::Arrived => {
                p.left = true;
                (None, 0)
            }
            Go::Failed => (None, 0),
        },
        j @ Job::Flee { to, until } => match go_to(w, p, Goal::Cell(to)) {
            Go::Moving if w.tick < until => (Some(j), 0),
            _ => (None, 30),
        },
        Job::Harvest { target, forced, harvest, tool } => (run_harvest(w, e, p, target, forced, harvest, tool), 0),
        Job::Deliver { bp, src, want, stage } => (run_deliver(w, p, bp, src, want, stage), 0),
        Job::Haul { src, to, stage, into } => (run_haul(w, p, src, to, stage, into), 0),
        Job::Supply { site, src, need, want, stage } => (run_supply(w, p, site, src, need, want, stage), 0),
        Job::Craft { site, tool } => (run_craft(w, e, p, site, tool), 0),
        Job::Construct { bp, tool } => (run_construct(w, e, p, bp, tool), 0),
        Job::Deconstruct { target } => (run_deconstruct(w, p, target), 0),
        Job::Eat { src, t, seat, stage } => (run_eat(w, p, src, t, seat, stage), 0),
        Job::Sleep { bed, spot, stage } => (run_sleep(w, e, p, bed, spot, stage), 0),
        Job::Comfort { to, need, until } => (run_comfort(w, p, to, need, until), 0),
        Job::Attack { target, until } => (run_attack(w, e, p, target, until), 0),
        Job::Breach { target } => (run_breach(w, p, target), 0),
    };
    match next {
        Some(j) => p.job = j,
        None => end_job(w, e, p, delay),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_harvest(
    w: &mut World,
    e: Entity,
    p: &mut Pawn,
    target: Entity,
    forced: bool,
    harvest: HarvestKey,
    tool: Option<Entity>,
) -> Option<Job> {
    let t = w.thing(target)?;
    let defs = w.defs.clone();
    let hd = defs.thing(t.def).harvest_by_key(harvest)?;
    let marked = w.ecs.get::<&Designated>(target).is_ok_and(|d| d.0 == hd.desig_r);
    if !w.harvest_ready(target, harvest) || (!forced && !marked) {
        return None;
    }
    // First the tool, if it needs one this pawn doesn't hold.
    if let Some(tl) = tool {
        let taken = fetch(w, e, p, tl)?;
        return Some(Job::Harvest { target, forced, harvest, tool: tool.filter(|_| !taken) });
    }
    if !w.hand_covers(p, hd.requires_r) {
        return None;
    }
    let goal = w.reach_goal(&t);
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => Some(Job::Harvest { target, forced, harvest, tool }),
        Go::Arrived => {
            // A tool sets the pace for the whole job, when it starts.
            let speed = match p.hand.filter(|_| hd.requires_r != 0) {
                Some(tl) => w.tool_speed(tl),
                None => 1.0,
            };
            let total = |_: &World| (hd.work as f64 / speed.max(0.01)).ceil() as u32;
            let skill = defs.work_types[defs.designations[hd.desig_r as usize].work_r as usize].skill_r;
            let amount = p.work_amount(skill);
            let work = w.work_on(target, t.pos, p.pos, Some(hd.desig_r), amount, total)?;
            if work.finished() {
                if marked {
                    let _ = w.ecs.remove_one::<Designated>(target);
                }
                if hd.destroy {
                    w.despawn_thing(target);
                } else {
                    let ready_at = w.tick + (hd.regrow_days * crate::TICKS_PER_DAY as f64) as u64;
                    w.regrow(target, harvest, ready_at);
                    w.finish_work(target);
                    w.map.touch(t.pos);
                }
                for &(yd, n) in &hd.yields_r {
                    w.place_item(yd, t.pos, n);
                }
                if hd.requires_r != 0 {
                    w.wear_tool(p);
                }
                None
            } else {
                Some(Job::Harvest { target, forced, harvest, tool })
            }
        }
    }
}

/// Walk to a tool lying about and take it: `Some(true)` once it's in hand,
/// `Some(false)` on the way, `None` if it's gone or out of reach.
fn fetch(w: &mut World, e: Entity, p: &mut Pawn, tool: Entity) -> Option<bool> {
    let lying = w.ecs.get::<&Held>(tool).is_err();
    let at = w.thing(tool).map(|t| t.pos).filter(|_| lying)?;
    match go_to(w, p, w.stack_goal(tool, at)) {
        Go::Failed => None,
        Go::Moving => Some(false),
        Go::Arrived => {
            w.take_tool(e, p, tool);
            Some(true)
        }
    }
}

/// Bring a work order's input: take it up at `src`, carry it to the site.
fn run_supply(w: &mut World, p: &mut Pawn, site: Entity, src: Entity, need: u8, want: u32, stage: u8) -> Option<Job> {
    let short = |w: &World| w.ecs.get::<&Order>(site).ok().and_then(|o| Some(o.needs.get(need as usize)?.missing()));
    if short(w).unwrap_or(0) == 0 {
        return None;
    }
    if stage == 0 {
        let s = w.thing(src)?;
        return match go_to(w, p, w.stack_goal(src, s.pos)) {
            Go::Failed => None,
            Go::Moving => Some(Job::Supply { site, src, need, want, stage }),
            Go::Arrived => {
                let lot = w.pick_up(src, want.min(CARRY_CAPACITY))?;
                w.reservations.remove(&src);
                p.carry = Some(lot);
                Some(Job::Supply { site, src, need, want, stage: 1 })
            }
        };
    }
    let st = w.thing(site)?;
    let at = st.pos;
    let goal = w.reach_goal(&st);
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => Some(Job::Supply { site, src, need, want, stage }),
        Go::Arrived => {
            let lot = p.carry?;
            let defs = w.defs.clone();
            if let Ok(mut o) = w.ecs.get::<&mut Order>(site) {
                if let Some(n) = o.needs.get_mut(need as usize).filter(|n| n.takes(&defs, lot.def)) {
                    let add = lot.count.min(n.missing());
                    if add > 0 {
                        // Lots alike in every way share a row; two axes worn
                        // differently stay two, and come back so if it's cancelled.
                        let same = |d: &&mut Lot| d.def == lot.def && d.made_of == lot.made_of && d.hp == lot.hp;
                        match n.delivered.iter_mut().find(same) {
                            Some(d) => d.count += add,
                            None => n.delivered.push(Lot { count: add, ..lot }),
                        }
                    }
                    p.carry = (lot.count > add).then_some(Lot { count: lot.count - add, ..lot });
                }
            }
            w.map.touch(at);
            None
        }
    }
}

/// Work a work order that has everything, with its tool.
fn run_craft(w: &mut World, e: Entity, p: &mut Pawn, site: Entity, tool: Option<Entity>) -> Option<Job> {
    let t = w.thing(site)?;
    let (requires, work, skill) = {
        let o = w.ecs.get::<&Order>(site).ok()?;
        if o.missing().is_some() {
            return None;
        }
        (o.requires.clone(), o.work, w.defs.work_types[o.work_type as usize].skill_r)
    };
    let need = w.defs.tool_mask(&requires)?;
    if let Some(tl) = tool {
        let taken = fetch(w, e, p, tl)?;
        return Some(Job::Craft { site, tool: tool.filter(|_| !taken) });
    }
    if !w.hand_covers(p, need) {
        return None;
    }
    let goal = w.reach_goal(&t);
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => Some(Job::Craft { site, tool }),
        Go::Arrived => {
            let speed = p.hand.filter(|_| need != 0).map_or(1.0, |tl| w.tool_speed(tl));
            let finished = {
                let mut o = w.ecs.get::<&mut Order>(site).ok()?;
                if o.total == 0 {
                    o.total = ((work as f64 / speed.max(0.01)).ceil() as u32).max(1);
                }
                o.done = (o.done + p.work_amount(skill)).min(o.total);
                o.done >= o.total
            };
            w.mark_worksite(site, t.pos);
            if !finished {
                return Some(Job::Craft { site, tool });
            }
            finish_order(w, site);
            if need != 0 {
                w.wear_tool(p);
            }
            None
        }
    }
}

/// A work order is done: its inputs are used up, and the mod that posted it
/// hears what went in, to make what it makes.
pub fn finish_order(w: &mut World, site: Entity) {
    let Ok(o) = w.ecs.remove_one::<Order>(site) else { return };
    let mut inputs: Vec<Lot> = Vec::new();
    for lot in o.needs.iter().flat_map(|n| &n.delivered) {
        match inputs.iter_mut().find(|i| i.def == lot.def && i.made_of == lot.made_of) {
            Some(i) => i.count += lot.count,
            None => inputs.push(*lot),
        }
    }
    // A material that went in as itself (flint), or else what the first
    // made thing that went in was made of (a flint axe, rehafted).
    let stuff = inputs
        .iter()
        .map(|i| i.def)
        .find(|&d| w.defs.thing(d).stuff.is_some())
        .or_else(|| inputs.iter().find_map(|i| i.made_of));
    if let Some(t) = w.thing(site) {
        w.map.touch(t.pos);
    }
    w.events.push(GameEvent::OrderDone { site, owner: o.owner, label: o.label, inputs, stuff });
}

fn run_deliver(w: &mut World, p: &mut Pawn, bp: Entity, src: Entity, want: u32, stage: u8) -> Option<Job> {
    if w.ecs.get::<&Blueprint>(bp).is_err() {
        return None;
    }
    if stage == 0 {
        let s = w.thing(src)?;
        return match go_to(w, p, w.stack_goal(src, s.pos)) {
            Go::Failed => None,
            Go::Moving => Some(Job::Deliver { bp, src, want, stage }),
            Go::Arrived => {
                let lot = w.pick_up(src, want.min(CARRY_CAPACITY))?;
                w.reservations.remove(&src);
                p.carry = Some(lot);
                Some(Job::Deliver { bp, src, want, stage: 1 })
            }
        };
    }
    let b = w.thing(bp)?;
    let goal = w.reach_goal(&b);
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => Some(Job::Deliver { bp, src, want, stage }),
        Go::Arrived => {
            let lot = p.carry?;
            if let Ok(mut bpc) = w.ecs.get::<&mut Blueprint>(bp) {
                if let Some(i) = bpc.cost.iter().position(|c| c.0 == lot.def) {
                    let add = lot.count.min(bpc.cost[i].1.saturating_sub(bpc.delivered[i]));
                    bpc.delivered[i] += add;
                    p.carry = (lot.count > add).then_some(Lot { count: lot.count - add, ..lot });
                }
            }
            // Nothing else about the map changed: tell whoever draws it
            // that the plan's materials arrived.
            w.map.touch(b.pos);
            None
        }
    }
}

/// Fetch a stack (as much as the cell will take, up to a carry), then set
/// it down on the stockpile cell. What no longer fits there is dropped
/// when the job ends.
fn run_haul(w: &mut World, p: &mut Pawn, src: Entity, to: IVec, stage: u8, into: Option<Entity>) -> Option<Job> {
    let again = |stage| Some(Job::Haul { src, to, stage, into });
    if stage == 0 {
        let s = w.thing(src)?;
        return match go_to(w, p, w.stack_goal(src, s.pos)) {
            Go::Failed => None,
            Go::Moving => again(0),
            Go::Arrived => {
                let of = w.made_of(src);
                let room = match into {
                    Some(c) => w.store_room(c, s.def, of),
                    None => w.room_for(s.def, of, to),
                };
                let lot = w.pick_up(src, s.count.min(CARRY_CAPACITY).min(room))?;
                w.reservations.remove(&src);
                p.carry = Some(lot);
                again(1)
            }
        };
    }
    let goal = match into.and_then(|c| w.thing(c)) {
        Some(ct) => w.reach_goal(&ct),
        None => Goal::Cell(to),
    };
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => again(1),
        Go::Arrived => {
            let lot = p.carry?;
            let left = match into {
                Some(c) => w.put_in_store(c, lot),
                None => w.put_lot(lot, to),
            };
            if left == 0 {
                p.carry = None;
                return None;
            }
            // The room went on the way: on to the next best place, or, with
            // none, set down nearby when the job ends.
            let lot = Lot { count: left, ..lot };
            p.carry = Some(lot);
            let next = haul_destination(w, (lot.def, lot.made_of), lot.hp, p.pos, None, None, &Bound::of(w, None));
            next.map(|(_, _, d)| Job::Haul { src, to: d.at, stage: 1, into: d.into })
        }
    }
}

/// Take a built thing down. As much work as it took to put up, and a
/// fraction of what it was made of comes back.
fn run_deconstruct(w: &mut World, p: &mut Pawn, target: Entity) -> Option<Job> {
    let t = w.thing(target)?;
    let Ok(desig) = w.ecs.get::<&Designated>(target).map(|d| d.0) else {
        return None; // cancelled
    };
    let goal = w.reach_goal(&t);
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => Some(Job::Deconstruct { target }),
        Go::Arrived => {
            let defs = w.defs.clone();
            let amount = p.work_amount(defs.work_types[defs.designations[desig as usize].work_r as usize].skill_r);
            let work = w.work_on(target, t.pos, p.pos, Some(desig), amount, |w| work_total(w, target))?;
            if !work.finished() {
                return Some(Job::Deconstruct { target });
            }
            let refund = w.defs.thing(t.def).build.as_ref().map_or(0.0, |b| b.refund);
            let cost = w.cost_of(target).unwrap_or_default();
            w.despawn_thing(target);
            for (d, n) in cost {
                let back = (n as f64 * refund).round() as u32;
                if back > 0 {
                    w.place_item(d, t.pos, back);
                }
            }
            None
        }
    }
}

fn run_construct(w: &mut World, e: Entity, p: &mut Pawn, bp: Entity, tool: Option<Entity>) -> Option<Job> {
    let b = w.thing(bp)?;
    {
        let bpc = w.ecs.get::<&Blueprint>(bp).ok()?;
        if bpc.cost.iter().zip(&bpc.delivered).any(|(c, d)| *d < c.1) {
            return None;
        }
    }
    // First the tool, if the build needs one this pawn doesn't hold.
    if let Some(tl) = tool {
        let taken = fetch(w, e, p, tl)?;
        return Some(Job::Construct { bp, tool: tool.filter(|_| !taken) });
    }
    let need = w.defs.thing(b.def).build.as_ref().map_or(0, |bd| bd.requires_r);
    if !w.hand_covers(p, need) {
        return None;
    }
    let goal = w.reach_goal(&b);
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => Some(Job::Construct { bp, tool }),
        Go::Arrived => {
            let skill = w.defs.build_work.and_then(|t| w.defs.work_types[t as usize].skill_r);
            let amount = p.work_amount(skill);
            let work = w.work_on(bp, b.pos, p.pos, None, amount, |w| work_total(w, bp))?;
            if work.finished() {
                // A pawn standing on a fresh wall, anywhere in its
                // footprint, steps out first.
                let td = w.defs.thing(b.def);
                let inside = [Some(p.pos), p.next].into_iter().flatten().find(|&c| td.footprint(b.pos).any(|f| f == c));
                if let Some(cell) = inside.filter(|_| td.blocks) {
                    return step_off(w, p, cell).then_some(Job::Construct { bp, tool });
                }
                complete_building(w, bp);
                if need != 0 {
                    w.wear_tool(p);
                }
                None
            } else {
                Some(Job::Construct { bp, tool })
            }
        }
    }
}

/// What it takes to put `e` up, or to take it down again.
fn work_total(w: &World, e: Entity) -> u32 {
    w.stat(e, "work").map_or(1, |x| x.round().max(1.0) as u32)
}

/// Move the pawn off `cell` to an adjacent open cell. Returns false if stuck.
fn step_off(w: &mut World, p: &mut Pawn, cell: IVec) -> bool {
    for (dx, dy) in crate::map::NEIGHBORS8 {
        let q = cell.offset(dx, dy);
        if w.map.passable(q) && w.map.fixture_at(q).is_none_or(|f| w.ecs.get::<&Blueprint>(f).is_err()) {
            p.path = vec![q];
            p.path_goal = None;
            return true;
        }
    }
    false
}

pub fn complete_building(w: &mut World, bp: Entity) {
    let Some(t) = w.thing(bp) else { return };
    let _ = w.ecs.remove_one::<Blueprint>(bp);
    w.touch_roles(t.def);
    // Taking it down later is work of its own, counted from zero.
    let _ = w.ecs.remove_one::<Work>(bp);
    let all = w.defs.clone();
    let td = all.thing(t.def);
    // The colony built it, so the colony owns it. A door only opens for
    // its owner; everyone else has to come through it the hard way.
    let _ = w.ecs.insert_one(bp, Owner(Faction::Player));
    if td.category == crate::defs::Category::Floor {
        w.map.set_floor(t.pos, Some(bp), td.path_cost);
        let defs = w.defs.clone();
        w.fields.add_emitters(&defs, &w.map, bp, t.def, t.pos);
        w.events.push(GameEvent::BuildingComplete { id: bp, def: t.def, pos: t.pos });
        return;
    }
    let (blocks, cost, door) = (td.blocks, td.path_cost, td.door);
    for c in td.footprint(t.pos) {
        w.map.set_fixture(c, Some(bp), blocks, cost, door);
        w.map.set_owner(c, Some(Faction::Player));
    }
    let span = w.support_span(bp);
    if span > 0 {
        w.set_support(bp, span);
    }
    let defs = w.defs.clone();
    w.fields.add_emitters(&defs, &w.map, bp, t.def, t.pos);
    if blocks {
        // Anyone else caught inside gets nudged out.
        for i in 0..w.pawns.len() {
            let e = w.pawns[i];
            let at = w.ecs.get::<&Pawn>(e).ok().filter(|o| o.active).map(|o| o.pos);
            if let Some(at) = at.filter(|&a| td.footprint(t.pos).any(|c| c == a)) {
                // The nearest open cell, ring by ring: from the middle of
                // a big thing, its neighbours are the thing.
                let ring = |r: i32| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy)));
                let out = (1..=crate::defs::MAX_SIZE as i32)
                    .flat_map(|r| ring(r).filter(move |(dx, dy)| dx.abs() == r || dy.abs() == r))
                    .map(|(dx, dy)| at.offset(dx, dy))
                    .find(|q| w.map.passable(*q));
                if let Some(q) = out {
                    if let Ok(mut o) = w.ecs.get::<&mut Pawn>(e) {
                        o.pos = q;
                        o.next = None;
                        o.path.clear();
                        o.path_goal = None;
                    }
                }
            }
        }
    }
    w.open_store(bp);
    w.events.push(GameEvent::BuildingComplete { id: bp, def: t.def, pos: t.pos });
}

/// Eat where the food lies, or carry one portion to a seat and eat there.
fn run_eat(w: &mut World, p: &mut Pawn, src: Entity, t: u32, seat: Option<(Entity, IVec)>, stage: u8) -> Option<Job> {
    let defs = w.defs.clone();
    let again = |src, seat| Job::Eat { src, t: 0, seat, stage: 0 };
    let eat = |p: &mut Pawn, food: DefId| -> bool {
        let nutrition = (defs.thing(food).food.as_ref().map_or(0.0, |f| f.nutrition) * NEED_MAX as f64) as i32;
        let mut full = true;
        for n in &mut p.needs {
            if defs.need(n.0).satisfier == Satisfier::Food {
                n.1 = (n.1 + nutrition).min(NEED_MAX);
                full = n.1 >= NEED_MAX * 9 / 10;
            }
        }
        full
    };

    // Stage 1: at the seat, or on the way to it, with a portion in hand.
    if stage == 1 {
        let (se, cell) = seat?;
        if w.thing(se).is_none() {
            // The chair went away: eat standing up, right here.
            let food = p.carry.take()?.def;
            eat(p, food);
            return None;
        }
        return match go_to(w, p, Goal::Cell(cell)) {
            Go::Failed => None,
            Go::Moving => Some(Job::Eat { src, t, seat, stage }),
            Go::Arrived => {
                if t < 90 {
                    return Some(Job::Eat { src, t: t + 1, seat, stage });
                }
                let food = p.carry.take()?.def;
                let full = eat(p, food);
                if full || w.thing(src).is_none() {
                    None
                } else {
                    Some(again(src, seat))
                }
            }
        };
    }

    // Stage 0: go to the food.
    let s = w.thing(src)?;
    match go_to(w, p, w.stack_goal(src, s.pos)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Eat { src, t, seat, stage }),
        Go::Arrived => {
            if let Some(seat) = seat {
                // Pick one portion up and take it to the seat.
                p.carry = Some(w.pick_up(src, 1)?);
                return Some(Job::Eat { src, t: 0, seat: Some(seat), stage: 1 });
            }
            if t < 90 {
                return Some(Job::Eat { src, t: t + 1, seat, stage });
            }
            if w.take_from_stack(src, 1) == 0 {
                return None;
            }
            let full = eat(p, s.def);
            if full || w.thing(src).is_none() {
                None
            } else {
                Some(again(src, None))
            }
        }
    }
}

fn run_comfort(w: &mut World, p: &mut Pawn, to: IVec, need: DefId, until: u64) -> Option<Job> {
    if w.tick > until {
        return None;
    }
    match go_to(w, p, Goal::Cell(to)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Comfort { to, need, until }),
        // Stay until the need has mostly recovered.
        Go::Arrived => (p.need(need)? < NEED_MAX * 9 / 10).then_some(Job::Comfort { to, need, until }),
    }
}

/// Sleepers get up when a field need turns dangerous: freezing in your sleep
/// is not restful.
const WAKE_BELOW: i32 = NEED_MAX * 15 / 100;

fn run_sleep(w: &mut World, e: Entity, p: &mut Pawn, bed: Option<Entity>, spot: IVec, stage: u8) -> Option<Job> {
    if p.last_attacker.is_some() {
        return None;
    }
    let defs = w.defs.clone();
    if stage == 0 {
        let rate = match bed.and_then(|b| w.thing(b)) {
            Some(b) => (defs.thing(b.def).bed.as_ref()?.rest_rate * 100.0) as u32,
            None if bed.is_some() => return None, // the bed went away
            None => 100,
        };
        return match go_to(w, p, Goal::Cell(spot)) {
            Go::Failed => None,
            Go::Moving => Some(Job::Sleep { bed, spot, stage }),
            Go::Arrived => {
                p.asleep = true;
                p.sleep_rate = rate;
                Some(Job::Sleep { bed, spot, stage: 1 })
            }
        };
    }
    p.asleep = true;
    // Wake when rested, or when an enemy gets close (checked every second).
    let rested = p.needs.iter().all(|n| defs.need(n.0).satisfier != Satisfier::Rest || n.1 >= NEED_MAX * 98 / 100);
    if rested {
        return None;
    }
    if p.needs.iter().any(|n| defs.need(n.0).satisfier == Satisfier::Field && n.1 < WAKE_BELOW) {
        return None;
    }
    if w.tick.is_multiple_of(60) && nearest_pawn(w, e, p.pos, 4, |o| o.faction == Faction::Hostile).is_some() {
        return None;
    }
    Some(Job::Sleep { bed, spot, stage })
}

fn run_attack(w: &mut World, e: Entity, p: &mut Pawn, target: Entity, until: u64) -> Option<Job> {
    let time_to_go = p.leave_at.is_some_and(|t| w.tick >= t);
    if (w.tick > until || wounded(&w.defs, p) || time_to_go) && !p.drafted {
        return None;
    }
    let tpos = w.pawn_pos(target)?;
    let target_retreating = w.ecs.get::<&Pawn>(target).is_ok_and(|t| retreating(&t));
    if target_retreating && p.pos.chebyshev(tpos) > 1 && !p.drafted {
        return None; // let them go
    }
    if p.pos.chebyshev(tpos) <= 1 && p.next.is_none() {
        p.path.clear();
        p.path_goal = None;
        if p.cooldown == 0 {
            hit(w, e, p, target, tpos);
        }
        return Some(Job::Attack { target, until });
    }
    // Chasing: follow the current path, replanning a few times a second.
    if p.moving() && w.tick < p.repath_at {
        return Some(Job::Attack { target, until });
    }
    p.repath_at = w.tick + 15;
    match go_to(w, p, Goal::Touch(tpos)) {
        Go::Failed => None,
        _ => Some(Job::Attack { target, until }),
    }
}

/// Hack at a piece of the wall until it gives. The job ends when it is
/// gone, and the next think finds the way in now open.
fn run_breach(w: &mut World, p: &mut Pawn, target: Entity) -> Option<Job> {
    let t = w.thing(target)?;
    let goal = w.reach_goal(&t);
    match go_to(w, p, goal) {
        Go::Failed => None,
        Go::Moving => Some(Job::Breach { target }),
        Go::Arrived => {
            if p.cooldown == 0 {
                hit_thing(w, p, target, t.pos);
            }
            Some(Job::Breach { target })
        }
    }
}

/// What a pawn's swing is worth before the roll: its creature's melee
/// damage scaled by its melee skill, plus the start's founder bonus when
/// this is the founder. The founder matters most when alone, and this is
/// where it shows; the bonus is flat, whatever the founder's skill.
pub fn melee_base(defs: &crate::defs::DefDb, p: &Pawn) -> i32 {
    let bonus = if p.founder { defs.start.as_ref().map_or(0, |s| s.founder_damage_bonus) } else { 0 };
    defs.creature(p.def).melee_damage * melee_skill_pct(defs, p) / 100 + bonus
}

/// A person's melee skill as a damage percentage: 80 untrained, 160 at the
/// top. 100 for an animal, and wherever no skill says `melee`.
fn melee_skill_pct(defs: &crate::defs::DefDb, p: &Pawn) -> i32 {
    match defs.melee_skill.filter(|_| defs.creature(p.def).intelligent) {
        Some(s) => 80 + 4 * p.skill(s) as i32,
        None => 100,
    }
}

/// The least and most a swing can do: the base rolled at 70% to 130%.
pub fn melee_bounds(defs: &crate::defs::DefDb, p: &Pawn) -> (i32, i32) {
    let base = melee_base(defs, p);
    ((base * 70 / 100).max(1), (base * 130 / 100).max(1))
}

/// Experience a melee swing is worth.
const SWING_XP: u32 = 20;

/// One melee swing, before it is applied to anything. It trains a person's
/// melee skill.
fn swing(w: &mut World, p: &mut Pawn) -> i32 {
    let defs = w.defs.clone();
    p.cooldown = defs.creature(p.def).melee_cooldown;
    let dmg = (melee_base(&defs, p) * (70 + w.rng.below(61) as i32) / 100).max(1);
    if let Some(s) = defs.melee_skill.filter(|_| defs.creature(p.def).intelligent) {
        p.learn(s, SWING_XP);
    }
    dmg
}

fn mark_hit(w: &mut World, tpos: IVec) {
    w.hits.push((tpos, w.tick));
    if w.hits.len() > 64 {
        w.hits.remove(0);
    }
}

fn hit_thing(w: &mut World, p: &mut Pawn, target: Entity, tpos: IVec) {
    let dmg = swing(w, p);
    let broken = match w.ecs.get::<&mut Thing>(target) {
        Ok(mut t) => {
            t.hp -= dmg;
            t.hp <= 0
        }
        Err(_) => return,
    };
    if broken {
        let label = w.thing(target).map(|t| w.defs.thing(t.def).label.clone()).unwrap_or_default();
        w.despawn_thing(target);
        w.message(format!("The {label} is broken down."), MsgKind::Threat);
    } else {
        // Lost hp wears it like work does; drawn live while under attack.
        w.mark_worksite(target, tpos);
    }
    mark_hit(w, tpos);
}

fn hit(w: &mut World, e: Entity, p: &mut Pawn, target: Entity, tpos: IVec) {
    let dmg = swing(w, p);
    if let Ok(mut t) = w.ecs.get::<&mut Pawn>(target) {
        t.hp -= dmg;
        t.last_attacker = Some(e);
        if t.hp <= 0 {
            t.dead = true;
        }
    }
    mark_hit(w, tpos);
}
