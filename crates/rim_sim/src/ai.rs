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
    if let Some((def, n)) = p.carry.take() {
        w.place_item(def, p.pos, n);
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
    let mut best: Option<((i32, u32), Entity)> = None;
    for (de, (t, owner)) in w.ecs.query::<(&Thing, &Owner)>().without::<&Blueprint>().iter() {
        if owner.0 == who || !w.map.blocks_fields(w.map.idx(t.pos)) {
            continue;
        }
        let key = (t.hp, t.pos.octile(from));
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

fn find_food(w: &mut World, e: Entity, p: &Pawn) -> Option<Job> {
    let defs = w.defs.clone();
    w.map.ensure_regions();
    let mut best: Option<(u32, Entity, bool)> = None;
    for (te, t) in w.ecs.query::<&Thing>().without::<&Blueprint>().without::<&Regrow>().iter() {
        let td = defs.thing(t.def);
        let is_item = td.category == Category::Item && td.food.is_some();
        let is_plant =
            !is_item && td.harvest.as_ref().is_some_and(|h| h.yields_r.iter().any(|y| defs.thing(y.0).food.is_some()));
        if !is_item && !is_plant {
            continue;
        }
        // Prefer ready food over foraging.
        let d = t.pos.octile(p.pos) + if is_plant { 150 } else { 0 };
        if best.is_some_and(|b| b.0 <= d) || w.reserved_by_other(te, e) {
            continue;
        }
        let goal = if is_item { Goal::Cell(t.pos) } else { Goal::Touch(t.pos) };
        if !w.map.can_reach(p.pos, goal) {
            continue;
        }
        best = Some((d, te, is_item));
    }
    let (_, t, is_item) = best?;
    w.reserve(t, e);
    Some(if is_item { Job::Eat { src: t, t: 0 } } else { Job::Harvest { target: t, work: 0, forced: true } })
}

fn find_bed(w: &mut World, e: Entity, p: &mut Pawn) -> Job {
    let defs = w.defs.clone();
    w.map.ensure_regions();
    let mut best: Option<(u32, Entity)> = None;
    for (te, t) in w.ecs.query::<&Thing>().without::<&Blueprint>().iter() {
        if defs.thing(t.def).bed.is_none() {
            continue;
        }
        let d = t.pos.octile(p.pos);
        if best.is_some_and(|b| b.0 <= d) || w.reserved_by_other(te, e) || !w.map.can_reach(p.pos, Goal::Cell(t.pos)) {
            continue;
        }
        best = Some((d, te));
    }
    let bed = best.map(|b| b.1);
    if let Some(b) = bed {
        w.reserve(b, e);
    }
    // No bed: sleep somewhere comfortable if the ground here isn't.
    let spot = if bed.is_some() { p.pos } else { comfortable_spot(w, p).unwrap_or(p.pos) };
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

/// Nearest job among: construct, deliver materials, designated harvest, hunt.
fn find_work(w: &mut World, e: Entity, p: &Pawn) -> Option<Job> {
    w.map.ensure_regions();
    let mut best: Option<(u32, Job, Entity)> = None;
    let consider = |best: &mut Option<(u32, Job, Entity)>, d: u32, job: Job, reserve: Entity| {
        if best.as_ref().is_none_or(|b| d < b.0) {
            *best = Some((d, job, reserve));
        }
    };

    // Blueprints, nearest first; stop at the first that yields a job.
    /// (distance, blueprint, position, first missing material and how many)
    type Candidate = (u32, Entity, IVec, Option<(DefId, u32)>);
    let mut bps: Vec<Candidate> = Vec::new();
    for (be, (t, bp)) in w.ecs.query::<(&Thing, &Blueprint)>().iter() {
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
                consider(&mut best, d, Job::Construct { bp: be }, be);
                break;
            }
            Some((mdef, want)) => {
                if let Some((sd, src)) = nearest_item(w, e, p.pos, mdef) {
                    consider(&mut best, sd + d, Job::Deliver { bp: be, src, want, stage: 0 }, be);
                    break;
                }
            }
        }
    }

    // Designated fixtures: harvest the natural ones, take down the built ones.
    for (te, (t, des)) in w.ecs.query::<(&Thing, &Designated)>().without::<&Regrow>().without::<&Blueprint>().iter() {
        let d = t.pos.octile(p.pos);
        if best.as_ref().is_some_and(|b| b.0 <= d) || w.reserved_by_other(te, e) {
            continue;
        }
        if w.map.can_reach(p.pos, Goal::Touch(t.pos)) {
            let job = match w.defs.designations[des.0 as usize].targets {
                Targets::Built => Job::Deconstruct { target: te, work: 0 },
                _ => Job::Harvest { target: te, work: 0, forced: false },
            };
            consider(&mut best, d, job, te);
        }
    }

    // Designated creatures (hunt).
    for &o in &w.pawns {
        if w.ecs.get::<&Designated>(o).is_err() || w.reserved_by_other(o, e) {
            continue;
        }
        let Some(op) = w.pawn_pos(o) else { continue };
        let d = op.octile(p.pos);
        if best.as_ref().is_none_or(|b| d < b.0) && w.map.can_reach(p.pos, Goal::Touch(op)) {
            consider(&mut best, d, Job::Attack { target: o, until: w.tick + 2400 }, o);
        }
    }

    let (_, job, res) = best?;
    w.reserve(res, e);
    if let Job::Deliver { src, .. } = job {
        w.reserve(src, e);
    }
    Some(job)
}

/// Nearest reachable stack of `def` that nobody but `e` has claimed.
pub fn nearest_item(w: &World, e: Entity, from: IVec, def: DefId) -> Option<(u32, Entity)> {
    let mut best: Option<(u32, Entity)> = None;
    for (te, t) in w.ecs.query::<&Thing>().without::<&Blueprint>().iter() {
        if t.def != def || w.map.item_at(t.pos) != Some(te) {
            continue;
        }
        let d = t.pos.octile(from);
        if best.is_some_and(|b| b.0 <= d) || w.reserved_by_other(te, e) || !w.map.can_reach(from, Goal::Cell(t.pos)) {
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
        Job::Harvest { target, work, forced } => (run_harvest(w, p, target, work, forced), 0),
        Job::Deliver { bp, src, want, stage } => (run_deliver(w, p, bp, src, want, stage), 0),
        Job::Construct { bp } => (run_construct(w, p, bp), 0),
        Job::Deconstruct { target, work } => (run_deconstruct(w, p, target, work), 0),
        Job::Eat { src, t } => (run_eat(w, p, src, t), 0),
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

fn run_harvest(w: &mut World, p: &mut Pawn, target: Entity, work: u32, forced: bool) -> Option<Job> {
    let t = w.thing(target)?;
    if w.ecs.get::<&Regrow>(target).is_ok() || (!forced && w.ecs.get::<&Designated>(target).is_err()) {
        return None;
    }
    let defs = w.defs.clone();
    let hd = defs.thing(t.def).harvest.as_ref()?;
    match go_to(w, p, Goal::Touch(t.pos)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Harvest { target, work, forced }),
        Go::Arrived => {
            if work + 1 >= hd.work {
                let _ = w.ecs.remove_one::<Designated>(target);
                if hd.destroy {
                    w.despawn_thing(target);
                } else {
                    let ready_at = w.tick + (hd.regrow_days * crate::TICKS_PER_DAY as f64) as u64;
                    let _ = w.ecs.insert_one(target, Regrow { ready_at });
                }
                for &(yd, n) in &hd.yields_r {
                    w.place_item(yd, t.pos, n);
                }
                None
            } else {
                Some(Job::Harvest { target, work: work + 1, forced })
            }
        }
    }
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
                let n = w.take_from_stack(src, want.min(CARRY_CAPACITY));
                if n == 0 {
                    return None;
                }
                w.reservations.remove(&src);
                p.carry = Some((s.def, n));
                Some(Job::Deliver { bp, src, want, stage: 1 })
            }
        };
    }
    let b = w.thing(bp)?;
    match go_to(w, p, Goal::Touch(b.pos)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Deliver { bp, src, want, stage }),
        Go::Arrived => {
            let (cdef, cn) = p.carry?;
            if let Ok(mut bpc) = w.ecs.get::<&mut Blueprint>(bp) {
                if let Some(i) = bpc.cost.iter().position(|c| c.0 == cdef) {
                    let add = cn.min(bpc.cost[i].1.saturating_sub(bpc.delivered[i]));
                    bpc.delivered[i] += add;
                    p.carry = (cn > add).then_some((cdef, cn - add));
                }
            }
            None
        }
    }
}

/// Take a built thing down. As much work as it took to put up, and a
/// fraction of what it was made of comes back.
fn run_deconstruct(w: &mut World, p: &mut Pawn, target: Entity, work: u32) -> Option<Job> {
    let t = w.thing(target)?;
    if w.ecs.get::<&Designated>(target).is_err() {
        return None; // cancelled
    }
    let total = w.stat(target, "work").map_or(1, |x| x.round().max(1.0) as u32);
    match go_to(w, p, Goal::Touch(t.pos)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Deconstruct { target, work }),
        Go::Arrived => {
            if work + 1 < total {
                return Some(Job::Deconstruct { target, work: work + 1 });
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
            let done = {
                let mut bpc = w.ecs.get::<&mut Blueprint>(bp).ok()?;
                bpc.work_left = bpc.work_left.saturating_sub(1);
                bpc.work_left == 0
            };
            if done {
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
    let td = w.defs.thing(t.def);
    let (blocks, cost, door) = (td.blocks, td.path_cost, td.door);
    w.map.set_fixture(t.pos, Some(bp), blocks, cost, door);
    // The colony built it, so the colony owns it. A door only opens for
    // its owner; everyone else has to come through it the hard way.
    let _ = w.ecs.insert_one(bp, Owner(Faction::Player));
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

fn run_eat(w: &mut World, p: &mut Pawn, src: Entity, t: u32) -> Option<Job> {
    let s = w.thing(src)?;
    match go_to(w, p, Goal::Cell(s.pos)) {
        Go::Failed => None,
        Go::Moving => Some(Job::Eat { src, t }),
        Go::Arrived => {
            if t < 90 {
                return Some(Job::Eat { src, t: t + 1 });
            }
            let defs = w.defs.clone();
            let nutrition = (defs.thing(s.def).food.as_ref()?.nutrition * NEED_MAX as f64) as i32;
            if w.take_from_stack(src, 1) == 0 {
                return None;
            }
            let mut full = true;
            for n in &mut p.needs {
                if defs.need(n.0).satisfier == Satisfier::Food {
                    n.1 = (n.1 + nutrition).min(NEED_MAX);
                    full = n.1 >= NEED_MAX * 9 / 10;
                }
            }
            if full || w.thing(src).is_none() {
                None
            } else {
                Some(Job::Eat { src, t: 0 })
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
        let (goal, rate) = match bed.and_then(|b| w.thing(b)) {
            Some(b) => (Goal::Cell(b.pos), (defs.thing(b.def).bed.as_ref()?.rest_rate * 100.0) as u32),
            None => (Goal::Cell(spot), 100),
        };
        return match go_to(w, p, goal) {
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

/// One melee swing, before it is applied to anything.
fn swing(w: &mut World, p: &mut Pawn) -> i32 {
    let defs = w.defs.clone();
    let cd = defs.creature(p.def);
    let bonus = if p.founder { defs.start.as_ref().map_or(0, |s| s.founder_damage_bonus) } else { 0 };
    p.cooldown = cd.melee_cooldown;
    ((cd.melee_damage + bonus) * (70 + w.rng.below(61) as i32) / 100).max(1)
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
