//! Interval systems. Each declares how often it runs in `sim.rs`.

use crate::defs::*;
use crate::world::*;
use crate::{IVec, TICKS_PER_DAY};

/// Every `NEEDS_INTERVAL` ticks: decay needs, apply sleep, starvation, healing.
pub const NEEDS_INTERVAL: u64 = 60;

pub fn needs(w: &mut World) {
    let defs = w.defs.clone();
    let frac = NEEDS_INTERVAL as f64 / TICKS_PER_DAY as f64;
    for i in 0..w.pawns.len() {
        let e = w.pawns[i];
        let Ok(mut p) = w.ecs.get::<&mut Pawn>(e) else { continue };
        if !p.active || p.dead {
            continue;
        }
        let mut starving = false;
        let asleep = p.asleep;
        let sleep_rate = p.sleep_rate.max(1) as f64 / 100.0;
        let mut dmg = 0.0;
        for n in &mut p.needs {
            let nd = defs.need(n.0);
            let delta = if nd.satisfier == Satisfier::Rest && asleep {
                // A full night's sleep on the ground restores ~1/3 day of rest.
                NEED_MAX as f64 * frac / 0.3 * sleep_rate
            } else {
                -(NEED_MAX as f64) * frac / nd.days_to_empty
            };
            n.1 = (n.1 + w.rng.round(delta)).clamp(0, NEED_MAX);
            if n.1 == 0 && nd.empty_damage_per_day > 0.0 {
                starving = true;
                dmg += nd.empty_damage_per_day * frac;
            }
        }
        let max = defs.creature(p.def).max_hp;
        if starving {
            p.hp -= w.rng.round(dmg);
            if p.hp <= 0 {
                p.dead = true;
            }
        } else if p.hp < max {
            // Heal ~25% of max hp per day, double while asleep.
            let rate = max as f64 * 0.25 * frac * if asleep { 2.0 } else { 1.0 };
            p.hp = (p.hp + w.rng.round(rate)).min(max);
        }
    }
}

/// Remove dead pawns, drop what they carried and what they're made of.
pub fn deaths(w: &mut World) {
    let mut i = 0;
    while i < w.pawns.len() {
        let e = w.pawns[i];
        let dead = w.ecs.get::<&Pawn>(e).map(|p| p.dead).unwrap_or(true);
        if !dead {
            i += 1;
            continue;
        }
        w.pawns.remove(i);
        let Ok(p) = w.ecs.remove_one::<Pawn>(e) else { continue };
        let _ = w.ecs.despawn(e);
        w.release_all(e);
        w.reservations.remove(&e);
        let defs = w.defs.clone();
        let cd = defs.creature(p.def);
        if let Some((d, n)) = p.carry {
            w.place_item(d, p.pos, n);
        }
        for &(d, n) in &cd.butcher_r {
            w.place_item(d, p.pos, n);
        }
        match p.faction {
            Faction::Player => w.message(format!("{} has died.", p.name), MsgKind::Bad),
            Faction::Hostile if cd.intelligent => w.message(format!("Raider {} was killed.", p.name), MsgKind::Info),
            _ => {}
        }
        w.events.push(GameEvent::PawnDied { id: e, name: p.name, def: p.def, faction: p.faction, pos: p.pos });
    }
    if !w.colony_lost && w.tick > 0 && w.colonists().next().is_none() {
        w.colony_lost = true;
        w.message("Everyone is dead. The colony is lost.", MsgKind::Bad);
        w.events.push(GameEvent::ColonyLost);
    }
}

pub fn regrow(w: &mut World) {
    let tick = w.tick;
    let ready: Vec<_> = w.ecs.query::<&Regrow>().iter().filter(|(_, r)| r.ready_at <= tick).map(|(e, _)| e).collect();
    for e in ready {
        let _ = w.ecs.remove_one::<Regrow>(e);
    }
}

/// Colony wealth: player-made things and items, plus colonists.
pub fn wealth(w: &mut World) {
    let defs = w.defs.clone();
    let mut total = 0.0;
    for (_, t) in w.ecs.query::<&Thing>().without::<&Blueprint>().iter() {
        let td = defs.thing(t.def);
        if !td.natural {
            total += td.market_value * t.count as f64;
        }
    }
    for e in w.colonists() {
        if let Ok(p) = w.ecs.get::<&Pawn>(e) {
            total += defs.creature(p.def).market_value;
        }
    }
    w.wealth = total;
}

/// Plants with `spread = true` slowly reseed empty ground.
pub fn spread_plants(w: &mut World) {
    let defs = w.defs.clone();
    for _ in 0..4 {
        let p = IVec::new(w.rng.below(w.map.w as u32) as i32, w.rng.below(w.map.h as u32) as i32);
        let i = w.map.idx(p);
        if w.map.fixture[i].is_some() || w.map.item[i].is_some() {
            continue;
        }
        let terrain = w.map.terrain[i];
        for (di, td) in defs.things.iter().enumerate() {
            let Some(s) = &td.spawn else { continue };
            if s.spread && s.terrain_r.contains(&terrain) && w.rng.chance(s.density * 4.0) {
                w.spawn_fixture(di as DefId, p, false);
                break;
            }
        }
    }
}
