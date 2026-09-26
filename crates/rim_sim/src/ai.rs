//! Pawn AI: think (pick a job when idle, staggered), run the job as a small
//! state machine, then advance movement.
//!
//! While a pawn is being processed its component is swapped out for an
//! inactive placeholder, so the job code has `&mut World` freely.

use crate::defs::*;
use crate::path::Goal;
use crate::world::*;
use crate::IVec;
use hecs::Entity;
use std::collections::BTreeMap;

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
    match w.pf.find(&w.map, p.pos, goal, 30_000, p.faction) {
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
    for &(nid, v) in &p.needs {
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
        if touches(t.pos, mine) && touches(t.pos, theirs) && w.map.can_reach_for(from, Goal::Touch(t.pos), who) {
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
    let d = w.ecs.get::<&Designated>(e).ok()?.0;
    let t = w.thing(e)?;
    if w.defs.designations[d as usize].targets == Targets::Thing {
        let h = w.defs.thing(t.def).harvest_for(d)?;
        if !w.harvest_ready(e, h.key()) {
            return Some("Growing back.".into());
        }
    }
    // Drafted colonists take no work.
    let free: Vec<Entity> =
        w.colonists().filter(|&c| w.ecs.get::<&Pawn>(c).is_ok_and(|p| !p.drafted && p.active)).collect();
    if free.is_empty() && w.colonists().next().is_some() {
        return Some("Everyone is drafted.".into());
    }
    let reachable = free.iter().filter_map(|&c| w.pawn_pos(c)).any(|p| w.map.can_reach(p, Goal::Touch(t.pos)));
    if !reachable {
        return Some("No colonist can reach it.".into());
    }
    let need = w.defs.thing(t.def).harvest_for(d).map_or(0, |h| h.requires_r);
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
        let goal = if is_item { Goal::Cell(t.pos) } else { Goal::Touch(t.pos) };
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

/// How much better a spot must be than where the pawn stands (in field units
/// outside comfort) before it's worth walking to.
const COMFORT_GAIN: f64 = 2.0;

/// Nearest cell, by walking, where every field need of this pawn is inside
/// its comfort range; failing that, the least uncomfortable cell within
/// reach, if it's clearly better than here (an unheated hut beats the night
/// outside). `None` if the pawn is already comfortable or nothing nearby is
/// better.
pub fn comfortable_spot(w: &World, p: &Pawn) -> Option<IVec> {
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
    let mut seen = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    seen.insert(p.pos);
    queue.push_back(p.pos);
    while let Some(c) = queue.pop_front() {
        let o = off(c);
        if o == 0.0 {
            return Some(c);
        }
        if o < best.0 {
            best = (o, c);
        }
        if seen.len() > COMFORT_SEARCH {
            break;
        }
        for (dx, dy) in crate::map::NEIGHBORS8 {
            let q = c.offset(dx, dy);
            if !w.map.passable(q) {
                continue;
            }
            // Check the corner before marking the cell seen: a door is only
            // ever reached straight on, and marking it on a rejected diagonal
            // would hide every room behind a door.
            if dx != 0 && dy != 0 && (!w.map.passable(c.offset(dx, 0)) || !w.map.passable(c.offset(0, dy))) {
                continue;
            }
            if seen.insert(q) {
                queue.push_back(q);
            }
        }
    }
    (best.0 + COMFORT_GAIN < here).then_some(best.1)
}

/// The work a colonist should do next (DESIGN.md §4d): of the work types
/// not at 0 once the priority rules have had their say, those at its lowest priority level that have reachable
/// work, and of those the nearest job; `order` breaks a tie. Work comes from
/// blueprints (the work type that covers "build"), designated things and
/// designated creatures, each designation naming its work type.
fn find_work(w: &mut World, e: Entity, p: &Pawn) -> Option<Job> {
    w.map.ensure_regions();
    let defs = w.defs.clone();
    let level: Vec<u8> =
        (0..defs.work_types.len() as DefId).map(|t| crate::rules::effective(&defs, &w.rules, p, t)).collect();
    let wanted = |t: DefId| level[t as usize] > 0;
    // The nearest job of each work type: (distance, job, what to reserve).
    let mut best: Vec<Option<(u32, Job, Entity)>> = vec![None; defs.work_types.len()];
    let nearer =
        |b: &Option<(u32, Job, Entity)>, d: u32, r: Entity| b.as_ref().is_none_or(|b| (d, r.id()) < (b.0, b.2.id()));

    // Blueprints, nearest first; stop at the first that yields a job.
    if let Some(bw) = defs.build_work.filter(|&t| wanted(t)) {
        /// (distance, blueprint, position, first missing material and how many)
        type Candidate = (u32, Entity, IVec, Option<(DefId, u32)>);
        let mut bps: Vec<Candidate> = Vec::new();
        for (be, t, bp) in w.ecs.query::<(Entity, &Thing, &Blueprint)>().iter() {
            if w.reserved_by_other(be, e) {
                continue;
            }
            let missing = bp.cost.iter().zip(&bp.delivered).find(|(c, d)| **d < c.1).map(|(c, d)| (c.0, c.1 - d));
            bps.push((t.pos.octile(p.pos), be, t.pos, missing));
        }
        bps.sort_by_key(|b| (b.0, b.1.id()));
        for (d, be, bpos, missing) in bps {
            if !w.map.can_reach(p.pos, Goal::Touch(bpos)) {
                continue;
            }
            match missing {
                None => {
                    best[bw as usize] = Some((d, Job::Construct { bp: be }, be));
                    break;
                }
                Some((mdef, want)) => {
                    if let Some((sd, src)) = nearest_item(w, e, p.pos, mdef) {
                        best[bw as usize] = Some((sd + d, Job::Deliver { bp: be, src, want, stage: 0 }, be));
                        break;
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
        let d = t.pos.octile(p.pos);
        if !wanted(dd.work_r) || !nearer(&best[dd.work_r as usize], d, te) || w.reserved_by_other(te, e) {
            continue;
        }
        // The walk to fetch a tool counts, as the walk to a material does.
        let (job, detour) = match dd.targets {
            Targets::Built => (Job::Deconstruct { target: te }, 0),
            _ => match defs.thing(t.def).harvest_for(des.0) {
                Some(h) if w.harvest_ready(te, h.key()) => match tool_for(w, e, p, h.requires_r, have) {
                    Some((extra, tool)) => (Job::Harvest { target: te, forced: false, harvest: h.key(), tool }, extra),
                    None => continue,
                },
                _ => continue,
            },
        };
        let nearest = detour == 0 || nearer(&best[dd.work_r as usize], d + detour, te);
        if nearest && w.map.can_reach(p.pos, Goal::Touch(t.pos)) {
            best[dd.work_r as usize] = Some((d + detour, job, te));
        }
    }

    // Work orders: bring what's missing, or work one that has it all.
    for (se, t, o) in w.ecs.query::<(Entity, &Thing, &Order)>().without::<&Blueprint>().iter() {
        let wt = o.work_type;
        let d = t.pos.octile(p.pos);
        if !wanted(wt) || !nearer(&best[wt as usize], d, se) || w.reserved_by_other(se, e) {
            continue;
        }
        if !w.map.can_reach(p.pos, Goal::Touch(t.pos)) {
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
        if let Some((dist, job)) = job.filter(|j| nearer(&best[wt as usize], j.0, se)) {
            best[wt as usize] = Some((dist, job, se));
        }
    }

    // Loose items a stockpile would take, unless work at a better level
    // was already found: hauling is the costliest search.
    if let Some(hw) = defs.haul_work.filter(|&t| wanted(t)) {
        let beaten = best.iter().enumerate().any(|(t, b)| b.is_some() && level[t] < level[hw as usize]);
        if !beaten {
            if let Some(h) = find_haul(w, e, p.pos) {
                if nearer(&best[hw as usize], h.0, h.2) {
                    best[hw as usize] = Some(h);
                }
            }
        }
    }

    // Designated creatures (hunt).
    for &o in &w.pawns {
        let Ok(des) = w.ecs.get::<&Designated>(o).map(|d| *d) else { continue };
        let wt = defs.designations[des.0 as usize].work_r;
        if !wanted(wt) || w.reserved_by_other(o, e) {
            continue;
        }
        let Some(op) = w.pawn_pos(o) else { continue };
        let d = op.octile(p.pos);
        if nearer(&best[wt as usize], d, o) && w.map.can_reach(p.pos, Goal::Touch(op)) {
            best[wt as usize] = Some((d, Job::Attack { target: o, until: w.tick + 2400 }, o));
        }
    }

    let rank = |t: DefId| defs.work_order.iter().position(|&o| o == t).unwrap_or(usize::MAX);
    let (_, (_, job, res)) = best
        .into_iter()
        .enumerate()
        .filter_map(|(t, b)| Some((t as DefId, b?)))
        .min_by_key(|(t, b)| (level[*t as usize], b.0, rank(*t), b.2.id()))?;
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

/// The nearest stack lying where no stockpile keeps it that one would take,
/// and the nearest cell with room for it: (the walk there and on to the
/// cell, the job, the stack to reserve).
fn find_haul(w: &World, e: Entity, from: IVec) -> Option<(u32, Job, Entity)> {
    if w.zones.list.is_empty() {
        return None;
    }
    // Cells other haulers are already bound for.
    let claimed: Vec<IVec> = w
        .pawns
        .iter()
        .filter(|&&o| o != e)
        .filter_map(|&o| match w.ecs.get::<&Pawn>(o).ok()?.job {
            Job::Haul { to, .. } => Some(to),
            _ => None,
        })
        .collect();
    let free = |def: DefId, of: Option<DefId>, c: IVec| !claimed.contains(&c) && w.room_for(def, of, c) > 0;
    // Whether any zone has room for a thing at all, worked out once per def
    // and material: with every stockpile full, that's all an idle hauler
    // has to learn.
    let mut room: BTreeMap<(DefId, Option<DefId>), bool> = BTreeMap::new();
    let mut best: Option<(u32, Entity, IVec)> = None;
    for (te, t) in w.ecs.query::<(Entity, &Thing)>().without::<&Blueprint>().iter() {
        if w.map.item_at(t.pos) != Some(te) || w.zones.at(&w.map, t.pos).is_some_and(|z| z.takes(t.def)) {
            continue;
        }
        let of = w.made_of(te);
        let any_room = *room.entry((t.def, of)).or_insert_with(|| {
            w.zones.members().any(|(z, c)| z.takes(t.def) && free(t.def, of, w.map.pos(c as usize)))
        });
        if !any_room {
            continue;
        }
        let d = t.pos.octile(from);
        if best.is_some_and(|b| (b.0, b.1.id()) <= (d, te.id()))
            || w.reserved_by_other(te, e)
            || !w.map.can_reach(from, Goal::Cell(t.pos))
        {
            continue;
        }
        let dest = w
            .zones
            .members()
            .filter(|(z, _)| z.takes(t.def))
            .map(|(_, c)| w.map.pos(c as usize))
            .filter(|&c| free(t.def, of, c))
            .map(|c| (c.octile(t.pos), c))
            .filter(|&(_, c)| w.map.can_reach(t.pos, Goal::Cell(c)))
            .min();
        if let Some((d2, to)) = dest {
            if best.is_none_or(|b| (d + d2, te.id()) < (b.0, b.1.id())) {
                best = Some((d + d2, te, to));
            }
        }
    }
    best.map(|(d, src, to)| (d, Job::Haul { src, to, stage: 0 }, src))
}

/// Nearest reachable stack of `def` that nobody but `e` has claimed.
pub fn nearest_item(w: &World, e: Entity, from: IVec, def: DefId) -> Option<(u32, Entity)> {
    nearest_item_where(w, e, from, |d| d == def)
}

/// `nearest_item`, of whatever `takes` accepts: a work order's input by tag.
pub fn nearest_item_where(w: &World, e: Entity, from: IVec, takes: impl Fn(DefId) -> bool) -> Option<(u32, Entity)> {
    let mut best: Option<(u32, Entity)> = None;
    for (te, t) in w.ecs.query::<(Entity, &Thing)>().without::<&Blueprint>().iter() {
        if !takes(t.def) || w.map.item_at(t.pos) != Some(te) {
            continue;
        }
        let d = t.pos.octile(from);
        if best.is_some_and(|b| (b.0, b.1.id()) <= (d, te.id()))
            || w.reserved_by_other(te, e)
            || !w.map.can_reach(from, Goal::Cell(t.pos))
        {
            continue;
        }
        best = Some((d, te));
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
        Job::Haul { src, to, stage } => (run_haul(w, p, src, to, stage), 0),
        Job::Supply { site, src, need, want, stage } => (run_supply(w, p, site, src, need, want, stage), 0),
        Job::Craft { site, tool } => (run_craft(w, e, p, site, tool), 0),
        Job::Construct { bp } => (run_construct(w, p, bp), 0),
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
    match go_to(w, p, Goal::Touch(t.pos)) {
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
                    // What grows back is harvested from scratch.
                    let _ = w.ecs.remove_one::<Work>(target);
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
    match go_to(w, p, Goal::Cell(at)) {
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
        return match go_to(w, p, Goal::Cell(s.pos)) {
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
    let at = w.thing(site)?.pos;
    match go_to(w, p, Goal::Touch(at)) {
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
    match go_to(w, p, Goal::Touch(t.pos)) {
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
        return match go_to(w, p, Goal::Cell(s.pos)) {
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
    match go_to(w, p, Goal::Touch(b.pos)) {
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
fn run_haul(w: &mut World, p: &mut Pawn, src: Entity, to: IVec, stage: u8) -> Option<Job> {
    if stage == 0 {
        let s = w.thing(src)?;
        return match go_to(w, p, Goal::Cell(s.pos)) {
            Go::Failed => None,
            Go::Moving => Some(Job::Haul { src, to, stage }),
            Go::Arrived => {
                let want = s.count.min(CARRY_CAPACITY).min(w.room_for(s.def, w.made_of(src), to));
                let lot = w.pick_up(src, want)?;
                w.reservations.remove(&src);
                p.carry = Some(lot);
                Some(Job::Haul { src, to, stage: 1 })
            }
        };
    }
    match go_to(w, p, Goal::Cell(to)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Haul { src, to, stage }),
        Go::Arrived => {
            let lot = p.carry?;
            let left = w.put_lot(lot, to);
            p.carry = (left > 0).then_some(Lot { count: left, ..lot });
            None
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
    match go_to(w, p, Goal::Touch(t.pos)) {
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

fn run_construct(w: &mut World, p: &mut Pawn, bp: Entity) -> Option<Job> {
    let b = w.thing(bp)?;
    {
        let bpc = w.ecs.get::<&Blueprint>(bp).ok()?;
        if bpc.cost.iter().zip(&bpc.delivered).any(|(c, d)| *d < c.1) {
            return None;
        }
    }
    match go_to(w, p, Goal::Touch(b.pos)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Construct { bp }),
        Go::Arrived => {
            let skill = w.defs.build_work.and_then(|t| w.defs.work_types[t as usize].skill_r);
            let amount = p.work_amount(skill);
            let work = w.work_on(bp, b.pos, p.pos, None, amount, |w| work_total(w, bp))?;
            if work.finished() {
                // A pawn standing on a fresh wall steps out first.
                if w.defs.thing(b.def).blocks && (p.pos == b.pos || p.next == Some(b.pos)) {
                    return step_off(w, p, b.pos).then_some(Job::Construct { bp });
                }
                complete_building(w, bp);
                None
            } else {
                Some(Job::Construct { bp })
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
    // Taking it down later is work of its own, counted from zero.
    let _ = w.ecs.remove_one::<Work>(bp);
    let td = w.defs.thing(t.def);
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
    w.map.set_fixture(t.pos, Some(bp), blocks, cost, door);
    w.map.set_owner(t.pos, Some(Faction::Player));
    let defs = w.defs.clone();
    w.fields.add_emitters(&defs, &w.map, bp, t.def, t.pos);
    if blocks {
        // Anyone else caught inside gets nudged out.
        for i in 0..w.pawns.len() {
            let e = w.pawns[i];
            let stuck = w.ecs.get::<&Pawn>(e).map(|o| o.active && o.pos == t.pos).unwrap_or(false);
            if stuck {
                if let Some(q) =
                    crate::map::NEIGHBORS8.iter().map(|(dx, dy)| t.pos.offset(*dx, *dy)).find(|q| w.map.passable(*q))
                {
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
    match go_to(w, p, Goal::Cell(s.pos)) {
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
    match go_to(w, p, Goal::Touch(t.pos)) {
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
