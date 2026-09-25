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
        let pos = p.pos;
        let mut dmg = 0.0;
        for n in &mut p.needs {
            let nd = defs.need(n.0);
            let delta = match nd.satisfier {
                // A full night's sleep on the ground restores ~1/3 day of rest.
                Satisfier::Rest if asleep => NEED_MAX as f64 * frac / 0.3 * sleep_rate,
                Satisfier::Field => {
                    // Drains in proportion to how far outside comfort the
                    // cell is (days_to_empty is the rate at 10 units out);
                    // refills while comfortable.
                    let v = w.fields.value(&defs, &w.map, nd.field_r as usize, pos);
                    let off = (nd.comfort[0] - v).max(v - nd.comfort[1]).max(0.0);
                    if off > 0.0 {
                        -(NEED_MAX as f64) * frac / nd.days_to_empty * off / 10.0
                    } else {
                        NEED_MAX as f64 * frac / nd.recover_days
                    }
                }
                _ => -(NEED_MAX as f64) * frac / nd.days_to_empty,
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
        let (dead, left) = w.ecs.get::<&Pawn>(e).map(|p| (p.dead, p.left)).unwrap_or((true, false));
        if left && !dead {
            w.pawns.remove(i);
            let Ok(p) = w.ecs.remove_one::<Pawn>(e) else { continue };
            let _ = w.ecs.despawn(e);
            w.release_all(e);
            w.reservations.remove(&e);
            let cd = w.defs.creature(p.def);
            if p.faction == Faction::Hostile {
                let who = if cd.intelligent { format!("Raider {}", p.name) } else { format!("The {}", cd.label) };
                w.message(format!("{who} fled."), MsgKind::Info);
            }
            w.note_event("left", e, &p.name);
            w.events.push(GameEvent::PawnLeft { id: e, name: p.name, def: p.def, faction: p.faction });
            continue;
        }
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
            // The founder's death is a colony event, told as one; the
            // colony goes on if anyone is left.
            Faction::Player if p.founder => {
                w.message(format!("{}, the founder, has fallen. The colony goes on.", p.name), MsgKind::Bad)
            }
            Faction::Player => w.message(format!("{} has died.", p.name), MsgKind::Bad),
            Faction::Hostile if cd.intelligent => w.message(format!("Raider {} was killed.", p.name), MsgKind::Info),
            _ => {}
        }
        w.note_event(if p.founder { "founder_died" } else { "died" }, e, &p.name);
        w.events.push(GameEvent::PawnDied {
            id: e,
            name: p.name,
            def: p.def,
            faction: p.faction,
            pos: p.pos,
            founder: p.founder,
        });
    }
    if !w.colony_lost && w.tick > 0 && w.colonists().next().is_none() {
        w.colony_lost = true;
        w.message("Everyone is dead. The colony is lost.", MsgKind::Bad);
        w.events.push(GameEvent::ColonyLost);
    }
}

pub fn regrow(w: &mut World) {
    let tick = w.tick;
    let mut ready = Vec::new();
    for (e, r) in w.ecs.query_mut::<(hecs::Entity, &mut Regrow)>() {
        if !r.ripen(tick) {
            ready.push(e);
        }
    }
    for e in ready {
        let _ = w.ecs.remove_one::<Regrow>(e);
        w.touch(e);
    }
}

/// Colony wealth: player-made things and items, plus colonists.
pub fn wealth(w: &mut World) {
    let defs = w.defs.clone();
    // Summed in id order: float addition isn't associative, and hecs's
    // iteration order isn't something a load reproduces.
    let mut values: Vec<(hecs::Entity, f64)> = w
        .ecs
        .query::<(hecs::Entity, &Thing, Option<&MadeOf>)>()
        .without::<&Blueprint>()
        .iter()
        .filter(|(_, t, _)| !defs.thing(t.def).natural)
        .map(|(e, t, made_of)| {
            (e, defs.thing(t.def).market_value * defs.factor(made_of.map(|m| m.0), "value") * t.count as f64)
        })
        .collect();
    values.sort_unstable_by_key(|v| v.0.id());
    let mut total: f64 = values.iter().map(|v| v.1).sum();
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
