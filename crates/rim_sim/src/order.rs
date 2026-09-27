//! Right-click orders: every order a spot offers a pawn, and which one a
//! plain click gives.
//!
//! `options` is the only place that decides what a right-click can mean.
//! The client reads it to label the cursor and fill the orders menu, and
//! `Command::Order` reads it again when the click lands, so neither can
//! promise something the order won't do.
//!
//! A plain right-click gives the first *safe* option (`resolve`). An option
//! that takes something away (deconstructing what the colony built, felling
//! a tree nobody marked) is `damaging`: it is only ever given when the
//! player picks it by name, from the menu.
//!
//! Orders are read out of the defs. A mod that adds a harvestable plant or a
//! buildable thing gets working orders without touching the engine.

use crate::ai;
use crate::defs::{HarvestDef, HarvestKey, Targets, ThingDef};
use crate::path::Goal;
use crate::world::*;
use crate::IVec;
use hecs::Entity;

/// How long an ordered hunt is pursued before the pawn gives up, matching a
/// hunt the pawn took from a designation.
const HUNT_TICKS: u64 = 2400;

/// A right-click resolved against the world.
pub struct Order {
    /// What the cursor promises: "Chop oak", "Hunt deer", "Go here".
    pub label: String,
    pub job: Job,
    /// Claimed when the order is given, so nobody else takes the same work.
    pub reserve: Vec<Entity>,
}

/// One thing a pawn could be ordered to do at a spot.
pub struct Choice {
    /// Stable for this spot and these things: what `Command::Order` names
    /// to give this choice rather than the default ("move",
    /// "deconstruct:<entity>").
    pub key: String,
    pub label: String,
    /// Takes something away: never a plain click's order.
    pub damaging: bool,
    /// The order, or `None` when it can't be given now, with `reason`.
    pub order: Option<Order>,
    pub reason: Option<String>,
}

impl Choice {
    fn ready(key: String, damaging: bool, order: Order) -> Choice {
        Choice { key, label: order.label.clone(), damaging, order: Some(order), reason: None }
    }
}

/// Every order `pawn` could be given at `cell`, safe ones first. `on` is
/// the creature the click landed on, which the caller hits first because a
/// creature stands on a cell rather than occupying it.
///
/// Read-only, and free of RNG: calling it every frame changes nothing.
pub fn options(w: &World, pawn: Entity, cell: IVec, on: Option<Entity>) -> Vec<Choice> {
    let mut out = Vec::new();
    let Ok(p) = w.ecs.get::<&Pawn>(pawn) else { return out };
    if !p.active || p.dead || p.faction != Faction::Player {
        return out;
    }
    let (from, drafted) = (p.pos, p.drafted);
    drop(p);

    if let Some(target) = on.filter(|&t| t != pawn) {
        if let Some(o) = creature(w, from, target, drafted) {
            out.push(Choice::ready(format!("attack:{}", target.to_bits()), false, o));
        }
    }
    // A drafted pawn is a soldier: it moves and it fights, and nothing else.
    if !drafted {
        for f in [w.map.fixture_at(cell), w.map.floor_at(cell)].into_iter().flatten() {
            fixture(w, pawn, from, f, &mut out);
        }
        if w.map.fixture_at(cell).is_none() {
            if let Some(thing) = w.solid_at(cell).and_then(|s| s.thing_r) {
                let td = w.defs.thing(thing);
                if w.map.can_reach(from, Goal::Touch(cell)) {
                    harvests(w, pawn, td, Entity::DANGLING, None, "rock".into(), &mut out);
                }
            }
        }
        if let Some(o) = w.map.item_at(cell).and_then(|i| item(w, from, i)) {
            out.push(Choice::ready("eat".into(), false, o));
        }
    }
    if w.map.passable(cell) && w.map.can_reach(from, Goal::Cell(cell)) {
        out.push(Choice::ready(
            "move".into(),
            false,
            Order { label: "Go here".into(), job: Job::MoveTo { to: cell }, reserve: Vec::new() },
        ));
    }
    // Safe first, keeping each kind's own order.
    out.sort_by_key(|c| c.damaging);
    out
}

/// The thing a job is aimed at, if any: what an undo checks the pawn is
/// still busy with.
pub fn target_of(job: &Job) -> Option<Entity> {
    match *job {
        Job::Harvest { target, .. } | Job::Deconstruct { target, .. } | Job::Attack { target, .. } => Some(target),
        Job::Construct { bp, .. } | Job::Deliver { bp, .. } => Some(bp),
        Job::Eat { src, .. } => Some(src),
        _ => None,
    }
}

/// What a plain right-click at `cell` makes `pawn` do: the first safe
/// order there, if any. Never a damaging one.
pub fn resolve(w: &World, pawn: Entity, cell: IVec, on: Option<Entity>) -> Option<Order> {
    options(w, pawn, cell, on).into_iter().filter(|c| !c.damaging).find_map(|c| c.order)
}

/// The order named `key` at this spot, if it can be given now: what a pick
/// from the orders menu gives, damaging or not.
pub fn choose(w: &World, pawn: Entity, cell: IVec, on: Option<Entity>, key: &str) -> Option<Order> {
    options(w, pawn, cell, on).into_iter().find(|c| c.key == key).and_then(|c| c.order)
}

fn creature(w: &World, from: IVec, target: Entity, drafted: bool) -> Option<Order> {
    let t = w.ecs.get::<&Pawn>(target).ok()?;
    if !t.active || t.dead || t.faction == Faction::Player || !w.map.can_reach(from, Goal::Touch(t.pos)) {
        return None;
    }
    let cd = w.defs.creature(t.def);
    // Hunting is killing something we can butcher; the rest is a fight.
    let hunt = t.faction == Faction::Wild && !cd.butcher_r.is_empty();
    Some(Order {
        label: format!("{} {}", if hunt { "Hunt" } else { "Attack" }, cd.label),
        // A soldier holds the target until the player says otherwise; a
        // worker sent hunting eventually gives up, as designated hunts do.
        job: Job::Attack { target, until: if drafted { u64::MAX } else { w.tick + HUNT_TICKS } },
        reserve: vec![target],
    })
}

/// A fixture's or floor's orders: build or supply its blueprint, each
/// harvest that's ready, and taking down what the colony built.
fn fixture(w: &World, pawn: Entity, from: IVec, f: Entity, out: &mut Vec<Choice>) {
    let Some(t) = w.thing(f) else { return };
    let td = w.defs.thing(t.def);
    if !w.map.can_reach(from, w.reach_goal(&t)) {
        return;
    }
    let id = f.to_bits();
    if let Ok(bp) = w.ecs.get::<&Blueprint>(f) {
        let missing = bp.cost.iter().zip(&bp.delivered).find(|(c, d)| **d < c.1).map(|(c, d)| (c.0, c.1 - d));
        drop(bp);
        let need = w.build_requires(f);
        let tool = w.ecs.get::<&Pawn>(pawn).ok().map(|p| ai::tool_for(w, pawn, &p, need, w.colony_tools()));
        match (missing, tool) {
            (None, Some(Some((_, tool)))) => out.push(Choice::ready(
                format!("build:{id}"),
                false,
                Order {
                    label: format!("Build {}", td.label),
                    job: Job::Construct { bp: f, tool },
                    reserve: std::iter::once(f).chain(tool).collect(),
                },
            )),
            (None, _) => {
                let tags = w.defs.tool_tag_names(need).join(" and ");
                let reason = if tags.is_empty() { "needs a tool".to_string() } else { format!("no {tags}") };
                let label = format!("Build {}", td.label);
                out.push(Choice {
                    key: format!("build:{id}"),
                    label,
                    damaging: false,
                    order: None,
                    reason: Some(reason),
                });
            }
            (Some((def, want)), _) => {
                if let Some((_, src)) = ai::nearest_item(w, pawn, from, def) {
                    out.push(Choice::ready(
                        format!("haul:{id}"),
                        false,
                        Order {
                            label: format!("Haul {} to {}", w.defs.thing(def).label, td.label),
                            job: Job::Deliver { bp: f, src, want, stage: 0 },
                            reserve: vec![f, src],
                        },
                    ));
                }
            }
        }
        return;
    }
    // Every harvest that's ready.
    let marked = w.ecs.get::<&Designated>(f).ok().and_then(|d| td.harvest_for(d.0)).map(|h| h.key());
    // Rock is offered from its terrain before it is a thing (DESIGN.md
    // §6d), so its key can't name the entity: the menu and the order it
    // sends must agree.
    let id = if w.is_rock(f) { "rock".to_string() } else { id.to_string() };
    harvests(w, pawn, td, f, marked, id.clone(), out);
    if !td.harvest.is_empty() {
        return;
    }
    // Something the colony built, if any mod offers a way to take it down.
    let ours = w.ecs.get::<&Owner>(f).is_ok_and(|o| o.0 == Faction::Player);
    let Some(take_down) = w.defs.designations.iter().find(|d| d.targets == Targets::Built) else { return };
    if ours && td.build.is_some() {
        out.push(Choice::ready(
            format!("deconstruct:{id}"),
            true,
            Order {
                label: format!("{} {}", take_down.label, td.label),
                job: Job::Deconstruct { target: f },
                reserve: vec![f],
            },
        ));
    }
}

/// Each harvest of `td` that's ready, as orders on `target`. The one the
/// player marked, or one that leaves the thing standing, is safe; felling
/// what nobody marked is damaging. A harvest needing a tool this pawn can't
/// get is listed with why.
fn harvests(
    w: &World,
    pawn: Entity,
    td: &ThingDef,
    target: Entity,
    marked: Option<HarvestKey>,
    id: String,
    out: &mut Vec<Choice>,
) {
    let Ok(p) = w.ecs.get::<&Pawn>(pawn) else { return };
    let have = w.colony_tools();
    // The marked harvest first, so a click on a tree marked for chopping
    // still chops it.
    let mut list: Vec<&HarvestDef> = td.harvest.iter().collect();
    list.sort_by_key(|h| marked != Some(h.key()));
    for h in list {
        if !w.harvest_ready(target, h.key()) {
            continue;
        }
        let label = format!("{} {}", w.defs.designations[h.desig_r as usize].label, td.label);
        let key = format!("harvest:{}:{id}", h.key().map_or(0, |k| k as u32 + 1));
        let damaging = h.destroy && marked != Some(h.key());
        match ai::tool_for(w, pawn, &p, h.requires_r, have) {
            Some((_, tool)) => out.push(Choice::ready(
                key,
                damaging,
                Order {
                    label,
                    job: Job::Harvest { target, forced: true, harvest: h.key(), tool },
                    reserve: std::iter::once(target).chain(tool).collect(),
                },
            )),
            None => {
                let tags = w.defs.tool_tag_names(h.requires_r).join(" and ");
                let reason = if tags.is_empty() { "needs a tool".to_string() } else { format!("no {tags}") };
                out.push(Choice { key, label, damaging, order: None, reason: Some(reason) });
            }
        }
    }
}

/// The pawn's current job in the same words the order that started it used,
/// so the panel and the cursor agree. Jobs the pawn chose for itself, and
/// anything whose target has gone, fall back to the generic label.
pub fn job_text(w: &World, p: &Pawn) -> String {
    describe(w, &p.job)
}

/// Why a colonist does or passes over a work type, in words, for the why
/// panel, the inspector and scripts.
pub fn why_text(w: &World, why: &crate::ai::Why) -> String {
    use crate::ai::Why;
    let defs = &w.defs;
    match why {
        Why::Picked(job) => format!("Next: {}", describe(w, job)),
        Why::Never => "Never".into(),
        Why::Nothing => "Nothing waiting".into(),
        Why::Reserved => "Someone else has the nearest".into(),
        Why::Unreachable => "Can't reach any".into(),
        Why::NoMaterials(Some(d)) => format!("No {} to bring", defs.thing(*d).label),
        Why::NoMaterials(None) => "Nothing to bring".into(),
        Why::NeedsTool(m) => match defs.tool_tag_names(*m).join(" and ") {
            tags if tags.is_empty() => "Needs a tool".into(),
            tags => format!("Needs a {tags} tool"),
        },
        Why::Beaten(t) => format!("{} first", defs.work_types[*t as usize].label),
    }
}

/// A why panel row in words: `why_text`, plus what an urgent mark did to
/// the pick. Above First it lifted the job a level; at First it could only
/// put it ahead of the rest there.
pub fn work_why_text(w: &World, x: &crate::ai::WorkWhy) -> String {
    let text = why_text(w, &x.why);
    match (x.urgent, x.level) {
        (false, _) => text,
        (true, 1) => format!("{text} (urgent: ahead of the rest)"),
        (true, _) => format!("{text} (urgent: a level sooner)"),
    }
}

fn item(w: &World, from: IVec, i: Entity) -> Option<Order> {
    let t = w.thing(i)?;
    let td = w.defs.thing(t.def);
    td.food.as_ref()?;
    w.map.can_reach(from, w.stack_goal(i, t.pos)).then(|| Order {
        label: format!("Eat {}", td.label),
        job: Job::Eat { src: i, t: 0, seat: None, stage: 0 },
        reserve: vec![i],
    })
}

pub fn describe(w: &World, job: &Job) -> String {
    let thing_label = |e: Entity| w.thing(e).map(|t| w.defs.thing(t.def).label.clone());
    let named = match job {
        Job::Harvest { target, harvest, .. } => w.thing(*target).and_then(|t| {
            let td = w.defs.thing(t.def);
            let hd = td.harvest_by_key(*harvest)?;
            Some(format!("{} {}", w.defs.designations[hd.desig_r as usize].label, td.label))
        }),
        Job::Construct { bp, .. } => thing_label(*bp).map(|l| format!("Build {l}")),
        Job::Deconstruct { target, .. } => {
            let verb = w.defs.designations.iter().find(|d| d.targets == Targets::Built).map(|d| d.label.as_str());
            thing_label(*target).map(|l| format!("{} {l}", verb.unwrap_or("Take down")))
        }
        Job::Deliver { bp, .. } => thing_label(*bp).map(|l| format!("Haul to {l}")),
        Job::Eat { src, .. } => thing_label(*src).map(|l| format!("Eat {l}")),
        Job::Attack { target, .. } => w.ecs.get::<&Pawn>(*target).ok().map(|t| {
            let cd = w.defs.creature(t.def);
            let hunt = t.faction == Faction::Wild && !cd.butcher_r.is_empty();
            format!("{} {}", if hunt { "Hunt" } else { "Attack" }, cd.label)
        }),
        _ => None,
    };
    named.unwrap_or_else(|| job.label().to_string())
}
