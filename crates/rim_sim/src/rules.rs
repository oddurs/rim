//! Priority rules and stances (DESIGN.md §4d): a colonist's effective
//! priority is the one the player set plus the rules that hold, and each
//! value can say how it got there. The colony-wide half of a rule (hours,
//! season, stance) is worked out when one of those changes; the colonist's
//! half (a need) is checked when the colonist looks for work.

use crate::defs::{DefDb, DefId};
use crate::world::{Pawn, World, NEED_MAX};

/// The rules whose colony-wide conditions hold, and what they were
/// worked out for. Derived: rebuilt on load.
#[derive(Default, Clone, Debug)]
pub struct Rules {
    /// Indices into `DefDb::priority_rules`, in load order.
    pub on: Vec<u16>,
    /// (hour, season, stance), with the parts no rule reads left at 0.
    key: Option<(u32, u32, Option<DefId>)>,
    /// How many times `on` was worked out, for tests and the profiler.
    pub evaluations: u64,
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
        let key = (hour, season, self.stance);
        if self.rules.key == Some(key) {
            return;
        }
        self.rules.on = (0..defs.priority_rules.len())
            .filter(|&i| {
                let w = &defs.priority_rules[i].when;
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
