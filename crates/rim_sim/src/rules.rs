//! Priority rules and stances (DESIGN.md §4d): a colonist's effective
//! priority is the one the player set plus the rules that hold, and each
//! value can say how it got there. The colony-wide half of a rule (hours,
//! season, stance) is worked out when one of those changes; the colonist's
//! half (a need) is checked when the colonist looks for work.

use crate::defs::{DefDb, DefId};
use crate::world::{GameEvent, Pawn, World, NEED_MAX};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The rules whose colony-wide conditions hold, and what they were
/// worked out for. Derived: rebuilt on load.
#[derive(Default, Clone, Debug)]
pub struct Rules {
    /// Indices into `DefDb::priority_rules`, in load order.
    pub on: Vec<u16>,
    /// (hour, season, stance, standing orders' generation), with the parts
    /// no rule reads left at 0.
    key: Option<(u32, u32, Option<DefId>, u64)>,
    /// How many times `on` was worked out, for tests and the profiler.
    pub evaluations: u64,
}

/// Colony readings and the standing orders on them (DESIGN.md §4d). Saved,
/// unlike `Rules`: a rule with a band holds or not depending on where its
/// reading has been, not only where it is. Rules are named by id, so a
/// save outlives mods being added or removed.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Standing {
    /// What scripts published, by qualified id, in thousandths.
    pub readings: BTreeMap<String, i64>,
    /// Reading rules that hold now.
    pub on: BTreeSet<String>,
    /// Rules the colony switched off.
    pub off: BTreeSet<String>,
    /// Bumped when `on` or `off` changes, so `update_rules` stays a key
    /// compare for readings that move without crossing a mark.
    #[serde(skip)]
    pub generation: u64,
}

impl Standing {
    pub fn hash(&self, mut h: u64) -> u64 {
        let bytes = |h: u64, s: &str| s.bytes().fold(h, |h, b| crate::rng::mix(h ^ b as u64));
        for (k, v) in &self.readings {
            h = crate::rng::mix(bytes(h, k) ^ *v as u64);
        }
        for k in &self.on {
            h = bytes(h ^ 0x0a, k);
        }
        for k in &self.off {
            h = bytes(h ^ 0x0f, k);
        }
        h
    }

    /// A reading as scripts see it.
    pub fn reading(&self, id: &str) -> Option<f64> {
        self.readings.get(id).map(|&v| v as f64 / 1000.0)
    }
}

/// One step of an explanation: what moved the value, and by how much.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub label: String,
    pub delta: i32,
}

impl World {
    /// Bring the colony's rules up to date with the hour, the season and
    /// the stance. Cheap when nothing changed: a key compare.
    pub fn update_rules(&mut self) {
        let defs = &self.defs;
        let hour = if defs.rules_read_hour { self.hour() as u32 } else { 0 };
        let season = if defs.rules_read_season { self.season_index() } else { 0 };
        let key = (hour, season, self.stance, self.standing.generation);
        if self.rules.key == Some(key) {
            return;
        }
        let standing = &self.standing;
        self.rules.on = (0..defs.priority_rules.len())
            .filter(|&i| {
                let rd = &defs.priority_rules[i];
                if standing.off.contains(&rd.id) || (rd.when.band_r.is_some() && !standing.on.contains(&rd.id)) {
                    return false;
                }
                let w = &rd.when;
                let hours = w.hours.is_none_or(|[a, b]| match a < b {
                    true => (a..b).contains(&hour),
                    false => hour >= a || hour < b,
                });
                let seasons = w.season_r.is_empty() || w.season_r.contains(&season);
                let stance = w.stance_r.is_none_or(|s| self.stance == Some(s));
                hours && seasons && stance
            })
            .map(|i| i as u16)
            .collect();
        self.rules.key = Some(key);
        self.rules.evaluations += 1;
    }

    /// A script publishes a colony reading. The rules that read it switch
    /// on or off only when it crosses a mark, each announcing it; a reading
    /// that moves inside its band costs a compare per rule reading it.
    pub fn set_reading(&mut self, id: &str, value: f64) {
        let v = crate::defs::milli(value);
        self.standing.readings.insert(id.to_string(), v);
        let defs = self.defs.clone();
        let mut changed = false;
        for rd in &defs.priority_rules {
            let Some(band) = rd.when.band_r.filter(|_| rd.when.reading.as_deref() == Some(id)) else { continue };
            let was = self.standing.on.contains(&rd.id);
            let now = band.holds(v, was);
            if now == was {
                continue;
            }
            changed = true;
            let rule = rd.id.clone();
            if now {
                self.standing.on.insert(rule.clone());
                self.events.push(GameEvent::RuleStarted { rule });
            } else {
                self.standing.on.remove(&rule);
                self.events.push(GameEvent::RuleStopped { rule });
            }
        }
        if changed {
            self.standing.generation += 1;
            self.update_rules();
        }
    }

    /// Switch a rule off for this colony, or back on.
    pub fn set_rule_enabled(&mut self, rule: DefId, on: bool) {
        let Some(rd) = self.defs.priority_rules.get(rule as usize) else { return };
        let changed = if on { self.standing.off.remove(&rd.id) } else { self.standing.off.insert(rd.id.clone()) };
        if changed {
            self.standing.generation += 1;
            self.update_rules();
        }
    }
}

/// A colonist's effective priority for a work type: 1 first, 0 never.
pub fn effective(defs: &DefDb, rules: &Rules, p: &Pawn, work: DefId) -> u8 {
    eval(defs, rules, p, work, &mut |_, _| {})
}

/// The effective priority and its parts, which sum to it: the base the
/// player set (or the work type's default), then each rule that moved it.
/// A rule that holds but can't move it (a shift past the last level)
/// shows as 0, so the player sees it was considered.
pub fn explain(defs: &DefDb, rules: &Rules, p: &Pawn, work: DefId) -> (u8, Vec<Part>) {
    let mut parts = Vec::new();
    let v = eval(defs, rules, p, work, &mut |label, delta| parts.push(Part { label: label.to_string(), delta }));
    (v, parts)
}

fn eval(defs: &DefDb, rules: &Rules, p: &Pawn, work: DefId, part: &mut dyn FnMut(&str, i32)) -> u8 {
    let levels = defs.priority_scale.levels as i32;
    let mut v = p.priority(defs, work) as i32;
    part("base", v);
    for &r in &rules.on {
        let rd = &defs.priority_rules[r as usize];
        let set = rd.set_r.binary_search_by_key(&work, |s| s.0).ok().map(|i| rd.set_r[i].1 as i32);
        let shift = rd.shift_r.binary_search_by_key(&work, |s| s.0).ok().map(|i| rd.shift_r[i].1);
        if (set.is_none() && shift.is_none()) || !holds_for(&rd.when, p) {
            continue;
        }
        let label = label(defs, r);
        let mut n = set.unwrap_or(v);
        if let Some(d) = shift.filter(|_| n > 0) {
            n = (n + d).clamp(1, levels);
        }
        part(label, n - v);
        v = n;
    }
    v as u8
}

/// The colonist's half of a rule: its need condition, if it has one.
fn holds_for(w: &crate::defs::WhenDef, p: &Pawn) -> bool {
    let Some(n) = w.need_r else { return true };
    let Some(v) = p.need(n) else { return false };
    let frac = v as f64 / NEED_MAX as f64;
    w.below.is_none_or(|b| frac < b) && w.above.is_none_or(|a| frac > a)
}

/// How a rule reads in an explanation: its label, its stance's, or its id.
pub fn label(defs: &DefDb, rule: u16) -> &str {
    let rd = &defs.priority_rules[rule as usize];
    if !rd.label.is_empty() {
        return &rd.label;
    }
    match rd.when.stance_r {
        Some(s) => &defs.stances[s as usize].label,
        None => &rd.id,
    }
}
