//! Right-click orders: one click, one pawn, one job.
//!
//! `resolve` is the only place that decides what a right-click means. The
//! client calls it each frame to label the cursor and `Command::Order` calls
//! it again when the click lands, so the cursor cannot promise something the
//! click won't do.
//!
//! Orders are read out of the defs. A mod that adds a harvestable plant or a
//! buildable thing gets a working right-click without touching the engine.

use crate::ai;
use crate::defs::{HarvestDef, Targets};
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

/// What a right-click at `cell` would make `pawn` do, if anything. `on` is
/// the creature the click landed on, which the caller hits first because a
/// creature stands on a cell rather than occupying it.
///
/// Read-only, and free of RNG: calling it every frame changes nothing.
pub fn resolve(w: &World, pawn: Entity, cell: IVec, on: Option<Entity>) -> Option<Order> {
    let (from, drafted) = {
        let p = w.ecs.get::<&Pawn>(pawn).ok()?;
        if !p.active || p.dead || p.faction != Faction::Player {
            return None;
        }
        (p.pos, p.drafted)
    };

    if let Some(target) = on.filter(|&t| t != pawn) {
        if let Some(o) = creature(w, from, target, drafted) {
            return Some(o);
        }
    }
    // A drafted pawn is a soldier: it moves and it fights, and nothing else.
    if !drafted {
        if let Some(o) = w.map.fixture_at(cell).and_then(|f| fixture(w, pawn, from, f)) {
            return Some(o);
        }
        if let Some(o) = w.map.floor_at(cell).and_then(|f| fixture(w, pawn, from, f)) {
            return Some(o);
        }
        if let Some(o) = w.map.item_at(cell).and_then(|i| item(w, from, i)) {
            return Some(o);
        }
    }
    (w.map.passable(cell) && w.map.can_reach(from, Goal::Cell(cell))).then(|| Order {
        label: "Go here".into(),
        job: Job::MoveTo { to: cell },
        reserve: Vec::new(),
    })
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

fn fixture(w: &World, pawn: Entity, from: IVec, f: Entity) -> Option<Order> {
    let t = w.thing(f)?;
    let td = w.defs.thing(t.def);
    if !w.map.can_reach(from, Goal::Touch(t.pos)) {
        return None;
    }
    if let Ok(bp) = w.ecs.get::<&Blueprint>(f) {
        let missing = bp.cost.iter().zip(&bp.delivered).find(|(c, d)| **d < c.1).map(|(c, d)| (c.0, c.1 - d));
        drop(bp);
        return Some(match missing {
            None => Order { label: format!("Build {}", td.label), job: Job::Construct { bp: f }, reserve: vec![f] },
            Some((def, want)) => {
                let (_, src) = ai::nearest_item(w, pawn, from, def)?;
                Order {
                    label: format!("Haul {} to {}", w.defs.thing(def).label, td.label),
                    job: Job::Deliver { bp: f, src, want, stage: 0 },
                    reserve: vec![f, src],
                }
            }
        });
    }
    // The harvest it's designated for, and only that. Otherwise a gentle
    // one (the thing stays), so a click gathers from a tree rather than
    // felling it, even while those branches grow back. Something with no
    // gentle harvest is taken as the click says. Either way it must be
    // ready, and a harvest that needs a tool this pawn can't get isn't on
    // offer.
    let p = w.ecs.get::<&Pawn>(pawn).ok()?;
    let have = w.colony_tools();
    // The tool to fetch for a harvest on offer: Some(None) when none is needed.
    let offer = |h: &HarvestDef| {
        w.harvest_ready(f, h.key()).then(|| ai::tool_for(w, pawn, &p, h.requires_r, have)).flatten().map(|t| t.1)
    };
    let gentle = td.harvest.iter().any(|h| !h.destroy);
    let pick = match w.ecs.get::<&Designated>(f).ok().and_then(|d| td.harvest_for(d.0)) {
        Some(h) => offer(h).map(|t| (h, t)),
        None => td.harvest.iter().filter(|h| !gentle || !h.destroy).find_map(|h| offer(h).map(|t| (h, t))),
    };
    if let Some((hd, tool)) = pick {
        return Some(Order {
            label: format!("{} {}", w.defs.designations[hd.desig_r as usize].label, td.label),
            job: Job::Harvest { target: f, forced: true, harvest: hd.key(), tool },
            reserve: std::iter::once(f).chain(tool).collect(),
        });
    }
    if !td.harvest.is_empty() {
        return None;
    }
    // Something the colony built, if any mod offers a way to take it down.
    let ours = w.ecs.get::<&Owner>(f).is_ok_and(|o| o.0 == Faction::Player);
    let take_down = w.defs.designations.iter().find(|d| d.targets == Targets::Built)?;
    (ours && td.build.is_some()).then(|| Order {
        label: format!("{} {}", take_down.label, td.label),
        job: Job::Deconstruct { target: f },
        reserve: vec![f],
    })
}

fn item(w: &World, from: IVec, i: Entity) -> Option<Order> {
    let t = w.thing(i)?;
    let td = w.defs.thing(t.def);
    td.food.as_ref()?;
    w.map.can_reach(from, Goal::Cell(t.pos)).then(|| Order {
        label: format!("Eat {}", td.label),
        job: Job::Eat { src: i, t: 0, seat: None, stage: 0 },
        reserve: vec![i],
    })
}

/// The pawn's current job in the same words the order that started it used,
/// so the panel and the cursor agree. Jobs the pawn chose for itself, and
/// anything whose target has gone, fall back to the generic label.
pub fn job_text(w: &World, p: &Pawn) -> String {
    let thing_label = |e: Entity| w.thing(e).map(|t| w.defs.thing(t.def).label.clone());
    let named = match &p.job {
        Job::Harvest { target, harvest, .. } => w.thing(*target).and_then(|t| {
            let td = w.defs.thing(t.def);
            let hd = td.harvest_by_key(*harvest)?;
            Some(format!("{} {}", w.defs.designations[hd.desig_r as usize].label, td.label))
        }),
        Job::Construct { bp } => thing_label(*bp).map(|l| format!("Build {l}")),
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
    named.unwrap_or_else(|| p.job.label().to_string())
}
