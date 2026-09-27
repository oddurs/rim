//! Interval systems. Each declares how often it runs in `sim.rs`.

use crate::defs::*;
use crate::world::*;
use crate::{terms, IVec, TICKS_PER_DAY};
use hecs::Entity;

/// Ticks between spoil passes; each pass works out a quarter of the
/// spoiling stacks, by entity id, so each once every `SPOIL_EVERY` passes.
pub const SPOIL_PASS: u64 = 250;
pub const SPOIL_EVERY: u64 = 4;

/// Stacks that spoil lose condition: by their def's rate terms where they
/// lie, slower in a store that keeps (its `keeps` terms), and out of the
/// weather in one that shelters. At no condition left a stack rots away.
/// Returns the stacks it worked out, so a test counts the staggering rather
/// than timing it.
pub fn spoil(w: &mut World) -> usize {
    let defs = w.defs.clone();
    let slot = (w.tick / SPOIL_PASS) % SPOIL_EVERY;
    let ticks = (SPOIL_PASS * SPOIL_EVERY) as i128;
    let mut changed: Vec<(Entity, i32, u16)> = Vec::new();
    let mut worked = 0;
    for (e, t, lost, held) in w.ecs.query::<(Entity, &Thing, Option<&Spoiling>, Option<&Contained>)>().iter() {
        if e.id() as u64 % SPOIL_EVERY != slot {
            continue;
        }
        let Some(sp) = &defs.thing(t.def).spoil else { continue };
        // Only stacks the colony could have: loose, or in a store.
        if held.is_none() && w.map.item_at(t.pos) != Some(e) {
            continue;
        }
        worked += 1;
        let store = held
            .and_then(|c| w.ecs.get::<&Thing>(c.store).ok().map(|s| s.def))
            .and_then(|d| defs.thing(d).store.as_ref());
        let rate = match (sp.rate_terms.is_empty(), store.is_some_and(|s| s.shelter)) {
            (true, _) => terms::Q,
            (false, true) => w.fields.eval_sheltered(&defs, &w.map, &sp.rate_terms, t.pos),
            (false, false) => w.fields.eval_at(&defs, &w.map, &sp.rate_terms, t.pos),
        };
        let keeps = match store.filter(|s| !s.keeps_terms.is_empty()) {
            Some(s) => w.fields.eval_at(&defs, &w.map, &s.keeps_terms, t.pos).max(terms::Q / 10),
            None => terms::Q,
        };
        if rate <= 0 {
            continue;
        }
        // Condition lost this pass, in 1/Q of a hit point: the share of its
        // life the rate eats in these ticks, times its whole condition.
        let whole = defs.thing(t.def).hp as i128;
        let days = terms::to_q(sp.days) as i128;
        let loss = whole * terms::Q as i128 * rate as i128 * ticks * terms::Q as i128
            / (keeps as i128 * TICKS_PER_DAY as i128 * days);
        let was = lost.map(|l| l.lost);
        let lost = was.unwrap_or(0) as i128 + loss;
        let (points, rest) = (lost / terms::Q as i128, (lost % terms::Q as i128) as u16);
        if points > 0 || was != Some(rest) {
            changed.push((e, t.hp - points.min(i32::MAX as i128) as i32, rest));
        }
    }
    // In id order: a stack that rots away changes the ledger and the map,
    // and a load doesn't reproduce the ECS's iteration order.
    changed.sort_unstable_by_key(|c| c.0.id());
    for (e, hp, rest) in changed {
        let _ = w.ecs.insert_one(e, Spoiling { lost: rest });
        if w.ecs.get::<&Thing>(e).is_ok_and(|t| t.hp != hp) {
            w.set_stack_hp(e, hp);
        }
    }
    worked
}

/// Every `NEEDS_INTERVAL` ticks: decay needs, apply sleep, starvation, healing.
pub const NEEDS_INTERVAL: u64 = 60;

pub fn needs(w: &mut World) {
    let defs = w.defs.clone();
    let frac = NEEDS_INTERVAL as f64 / TICKS_PER_DAY as f64;
    // Lines needs speak as they cross their level, said after the pass.
    let mut said: Vec<(Entity, usize)> = Vec::new();
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
        let talks = defs.creature(p.def).intelligent;
        let mut dmg = 0.0;
        // What it wears keeps the cold off an insulated need.
        let warmth = if p.worn.is_empty() { 0.0 } else { w.warmth_worn(&p) };
        for (k, n) in p.needs.iter_mut().enumerate() {
            let nd = defs.need(n.0);
            let was = n.1;
            let delta = match nd.satisfier {
                // A full night's sleep on the ground restores ~1/3 day of rest.
                Satisfier::Rest if asleep => NEED_MAX as f64 * frac / 0.3 * sleep_rate,
                Satisfier::Field => {
                    // Drains in proportion to how far outside comfort the
                    // cell is (days_to_empty is the rate at 10 units out);
                    // refills while comfortable.
                    let v = w.fields.value(&defs, &w.map, nd.field_r as usize, pos);
                    let low = nd.comfort[0] - if nd.insulated { warmth } else { 0.0 };
                    let off = (low - v).max(v - nd.comfort[1]).max(0.0);
                    if off > 0.0 {
                        -(NEED_MAX as f64) * frac / nd.days_to_empty * off / 10.0
                    } else {
                        NEED_MAX as f64 * frac / nd.recover_days
                    }
                }
                _ => -(NEED_MAX as f64) * frac / nd.days_to_empty,
            };
            n.1 = (n.1 + w.rng.round(delta)).clamp(0, NEED_MAX);
            if let Some(say) = nd.say.as_ref().filter(|_| talks) {
                let level = (say.below * NEED_MAX as f64) as i32;
                if was >= level && n.1 < level {
                    said.push((e, k));
                }
            }
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
    wear_apparel(w, frac);
    for (e, k) in said {
        let Some(need) = w.ecs.get::<&Pawn>(e).ok().map(|p| p.needs[k].0) else { continue };
        let Some(say) = defs.need(need).say.as_ref() else { continue };
        // Which line: the pawn and the moment, not the world's RNG, so a
        // word said changes nothing that follows.
        let pick = (e.id() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ w.tick;
        let line = &say.lines[(pick % say.lines.len() as u64) as usize];
        w.say(e, line, say.ticks, 1);
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
            // Whoever walks off the map takes what they hold with them.
            if let Some(t) = p.hand {
                w.despawn_thing(t);
            }
            for &g in &p.worn {
                w.despawn_thing(g);
            }
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
        if let Some(lot) = p.carry {
            w.place_lot(lot, p.pos);
        }
        if let Some(t) = p.hand {
            w.put_down(t, p.pos);
        }
        for &g in &p.worn {
            let _ = w.ecs.remove_one::<Worn>(g);
            w.put_down(g, p.pos);
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

/// Garments wear as they're worn, by their `wear_per_day`; one with no hit
/// points left is worn out and gone.
fn wear_apparel(w: &mut World, frac: f64) {
    let defs = w.defs.clone();
    let mut worn_out: Vec<(Entity, Entity)> = Vec::new();
    for i in 0..w.pawns.len() {
        let e = w.pawns[i];
        let worn = match w.ecs.get::<&Pawn>(e) {
            Ok(p) if !p.worn.is_empty() => p.worn.clone(),
            _ => continue,
        };
        for g in worn {
            let Some(t) = w.thing(g) else { continue };
            let per_day = defs.thing(t.def).apparel.as_ref().map_or(0.0, |a| a.wear_per_day);
            if per_day <= 0.0 {
                continue;
            }
            let hp = t.hp - w.rng.round(per_day * frac);
            if hp > 0 {
                if let Ok(mut t) = w.ecs.get::<&mut Thing>(g) {
                    t.hp = hp;
                }
            } else {
                worn_out.push((e, g));
            }
        }
    }
    for (e, g) in worn_out {
        let label = w.thing(g).map(|t| defs.thing(t.def).label.clone()).unwrap_or_default();
        let name = w.ecs.get::<&mut Pawn>(e).ok().map(|mut p| {
            p.worn.retain(|&x| x != g);
            p.name.clone()
        });
        if let Some(name) = name {
            w.message(format!("{name}'s {label} wore out."), MsgKind::Info);
        }
        w.despawn_thing(g);
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

/// Plants with `spread = true` slowly reseed empty ground. One that grows
/// takes more readily where it would grow well, and starts as a seedling.
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
            if !s.spread || !s.terrain_r.contains(&terrain) {
                continue;
            }
            // Twice as likely where it grows twice as fast; never where it can't.
            let weight = td.grow.as_ref().map_or(1.0, |g| match g.rate_terms.is_empty() {
                true => 1.0,
                false => terms::from_q(w.fields.eval_at(&defs, &w.map, &g.rate_terms, p)).clamp(0.0, 2.0),
            });
            if w.rng.chance(s.density * 4.0 * weight) {
                if let Some(e) = w.spawn_fixture(di as DefId, p, false) {
                    if td.grow.is_some() {
                        w.plant_seedling(e);
                    }
                }
                break;
            }
        }
    }
}

/// Growing zones: lay the plant's plan on each empty cell while it would
/// grow there, take back plans nobody has started once it wouldn't, and
/// mark each grown plant for its harvest. Every plant pass.
pub fn tend(w: &mut World) {
    let defs = w.defs.clone();
    let cells: Vec<(DefId, u32)> = w.zones.growing().collect();
    let mut sow: Vec<(DefId, IVec)> = Vec::new();
    let mut unsow: Vec<Entity> = Vec::new();
    let mut reap: Vec<(Entity, DefId)> = Vec::new();
    for (plant, c) in cells {
        let (i, p) = (c as usize, w.map.pos(c as usize));
        let Some(gd) = &defs.thing(plant).grow else { continue };
        let in_season = || gd.rate_terms.is_empty() || w.fields.eval_at(&defs, &w.map, &gd.rate_terms, p) > 0;
        match w.map.fixture[i] {
            None if w.map.item[i].is_none() && w.map.floor[i].is_none() && w.map.passable(p) => {
                if in_season() {
                    sow.push((plant, p));
                }
            }
            Some(f) => {
                let Some(t) = w.thing(f).filter(|t| t.def == plant) else { continue };
                if w.ecs.get::<&Blueprint>(f).is_ok() {
                    let untouched = w.ecs.get::<&Work>(f).map_or(true, |k| k.done == 0);
                    if untouched && !w.reservations.contains_key(&f) && !in_season() {
                        unsow.push(f);
                    }
                } else if w.ecs.get::<&Growth>(f).is_ok_and(|g| g.progress == GROWN)
                    && w.ecs.get::<&Designated>(f).is_err()
                {
                    if let Some(h) = defs.thing(t.def).harvest.iter().find(|h| h.destroy) {
                        reap.push((f, h.desig_r));
                    }
                }
            }
            None => {}
        }
    }
    for (plant, p) in sow {
        w.spawn_fixture(plant, p, true);
    }
    for f in unsow {
        w.despawn_thing(f);
    }
    for (f, d) in reap {
        let _ = w.ecs.insert_one(f, Designated(d));
        w.touch(f);
    }
}

/// Plants are worked out once in this many plant passes, a quarter of
/// them each pass, by entity id.
pub const GROW_EVERY: u64 = 4;

/// Ticks between plant passes.
pub const PLANT_PASS: u64 = 250;

/// Grow a quarter of the plants: by their rate terms over the time since
/// they were last worked out, less what their harm terms take. A plant
/// with no health left dies; one grown again bears its crop. Returns the
/// plants it worked out, so a test counts the staggering rather than timing it.
pub fn grow(w: &mut World) -> usize {
    let defs = w.defs.clone();
    let slot = (w.tick / PLANT_PASS) % GROW_EVERY;
    let days = (PLANT_PASS * GROW_EVERY) as i64 * terms::Q / TICKS_PER_DAY as i64;
    let mut changed: Vec<(Entity, Growth, bool)> = Vec::new();
    let mut worked = 0;
    for (e, t, g) in w.ecs.query::<(Entity, &Thing, &Growth)>().iter() {
        if e.id() as u64 % GROW_EVERY != slot {
            continue;
        }
        let Some(gd) = &defs.thing(t.def).grow else { continue };
        worked += 1;
        let rate = match gd.rate_terms.is_empty() {
            true => terms::Q,
            false => w.fields.eval_at(&defs, &w.map, &gd.rate_terms, t.pos),
        };
        let harm = match gd.harm_terms.is_empty() {
            true => 0,
            false => w.fields.eval_at(&defs, &w.map, &gd.harm_terms, t.pos).max(0),
        };
        let mut n = *g;
        n.dormant = rate <= 0;
        if rate > 0 {
            let step = rate * days / terms::to_q(gd.days) * GROWN as i64 / terms::Q;
            n.progress = (n.progress as i64 + step).min(GROWN as i64) as u16;
        }
        let health = match harm > 0 {
            true => n.health as i64 - harm * days / terms::Q * GROWN as i64 / terms::Q,
            false => n.health as i64 + terms::to_q(gd.heal) * days / terms::Q * GROWN as i64 / terms::Q,
        };
        n.health = health.clamp(0, GROWN as i64) as u16;
        // Redrawn only when it looks different: a size, dormancy, or
        // falling below half health.
        let hurt = |g: &Growth| g.health < GROWN / 2;
        let redraw = n.stage(gd.stages) != g.stage(gd.stages) || n.dormant != g.dormant || hurt(&n) != hurt(g);
        if n != *g {
            changed.push((e, n, redraw));
        }
    }
    // In id order: deaths and redraws touch the map, and a load doesn't
    // reproduce the ECS's iteration order.
    changed.sort_unstable_by_key(|c| c.0.id());
    for (e, n, redraw) in changed {
        if n.health == 0 {
            w.despawn_thing(e);
            continue;
        }
        let _ = w.ecs.insert_one(e, n);
        if n.progress == GROWN {
            w.bear(e);
        }
        if redraw {
            w.touch(e);
        }
    }
    worked
}
