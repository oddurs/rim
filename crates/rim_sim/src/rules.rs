//! Work roles, priority rules and stances (DESIGN.md §4d): a colonist's
//! effective priority starts at their work role's level (or the work type's
//! default), takes the pin the player set, then the rules that hold, and
//! each value can say how it got there. The colony-wide half of a rule
//! (hours, season, stance) is worked out when one of those changes; the
//! colonist's half (a need) is checked when the colonist looks for work.

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
/// What the colony keeps of a work role: a copy of a `[[work_role]]`, or
/// one the player made. Colonists name it by its index here, and the
/// player edits the copy, never the def. Saved.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRole {
    /// The def it was seeded from, by qualified id; none for the player's.
    pub def: Option<String>,
    pub label: String,
    pub order: i32,
    /// The levels it sets, by work type, sorted; what it leaves out is the
    /// work type's default.
    pub priorities: Vec<(DefId, u8)>,
    /// The player changed it, so it no longer follows its def.
    pub edited: bool,
    /// The planner that sets its members' levels, for a planned role
    /// (Auto): always its def's, since the player edits levels, not code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planner: Option<String>,
}

/// A level a planner set for a colonist, and why, in the planner's words.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Planned {
    pub work: DefId,
    pub level: u8,
    pub reason: String,
}

impl WorkRole {
    /// The level the role sets for a work type, if it sets one.
    pub fn level(&self, work: DefId) -> Option<u8> {
        self.priorities.binary_search_by_key(&work, |p| p.0).ok().map(|i| self.priorities[i].1)
    }

    /// Set a work type's level, or leave it to the default with `None`,
    /// keeping the list sorted so the state doesn't depend on edit order.
    pub fn set(&mut self, work: DefId, level: Option<u8>) {
        match (self.priorities.binary_search_by_key(&work, |p| p.0), level) {
            (Ok(i), Some(l)) => self.priorities[i].1 = l,
            (Err(i), Some(l)) => self.priorities.insert(i, (work, l)),
            (Ok(i), None) => {
                self.priorities.remove(i);
            }
            (Err(_), None) => {}
        }
    }
}

/// What kind of step an explanation part is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    /// The work type's default: the first part, always.
    Default,
    /// The colonist's work role set it.
    Role,
    /// The player pinned it for this colonist.
    Pin,
    /// A planned role's planner set it; the label is its reason.
    Plan,
    /// A rule or the stance moved it.
    Rule,
}

impl PartKind {
    pub fn name(self) -> &'static str {
        match self {
            PartKind::Default => "default",
            PartKind::Role => "role",
            PartKind::Pin => "pin",
            PartKind::Plan => "plan",
            PartKind::Rule => "rule",
        }
    }
}

/// One step of an explanation: what moved the value, and by how much. A
/// role's part is labelled with the role, a pin's with the colonist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub kind: PartKind,
    pub label: String,
    pub delta: i32,
}

impl World {
    /// Bring the colony's rules up to date with the hour, the season and
    /// the stance. Cheap when nothing changed: a key compare.
    pub fn update_rules(&mut self) {
        let defs = self.defs.clone();
        let hour = if defs.rules_read_hour { self.hour() as u32 } else { 0 };
        let season = if defs.rules_read_season { self.season_index() } else { 0 };
        let key = (hour, season, self.stance, self.standing.generation);
        if self.rules.key == Some(key) {
            return;
        }
        // The first work-out (a new game, a load) only learns what holds; after
        // that, a reading rule entering or leaving is news.
        let first = self.rules.key.is_none();
        let before = std::mem::take(&mut self.rules.on);
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
        if !first {
            for (i, rd) in defs.priority_rules.iter().enumerate().filter(|(_, r)| r.when.band_r.is_some()) {
                let (was, now) = (before.contains(&(i as u16)), self.rules.on.contains(&(i as u16)));
                match (was, now) {
                    (false, true) => self.events.push(GameEvent::RuleStarted { rule: rd.id.clone() }),
                    (true, false) => self.events.push(GameEvent::RuleStopped { rule: rd.id.clone() }),
                    _ => {}
                }
            }
        }
    }

    /// A script publishes a colony reading. The rules that read it switch
    /// on or off only when it crosses a mark; a reading that moves inside
    /// its band costs a compare per rule reading it. A rule is announced
    /// when it starts or stops holding (`update_rules`), with its season
    /// and the colony's switch taken into account.
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
            let rule = rd.id.clone();
            // A rule the colony switched off keeps track of its reading, so it
            // is right the moment it's switched back on, but moves nothing
            // meanwhile.
            changed |= !self.standing.off.contains(&rule);
            if now {
                self.standing.on.insert(rule);
            } else {
                self.standing.on.remove(&rule);
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

impl World {
    /// Copy work roles in from the defs: a new one joins the colony's, one
    /// the player never edited follows its def, and an edited one stays
    /// the player's. At a new game and on every load.
    pub fn seed_work_roles(&mut self) {
        let defs = self.defs.clone();
        for d in &defs.work_roles {
            match self.work_roles.iter_mut().find(|r| r.def.as_deref() == Some(d.id.as_str())) {
                Some(r) if r.edited => r.planner = d.planner.clone(),
                Some(r) => {
                    r.label = d.label.clone();
                    r.order = d.order;
                    r.priorities = d.priorities_r.clone();
                    r.planner = d.planner.clone();
                }
                None => self.work_roles.push(WorkRole {
                    def: Some(d.id.clone()),
                    label: d.label.clone(),
                    order: d.order,
                    priorities: d.priorities_r.clone(),
                    edited: false,
                    planner: d.planner.clone(),
                }),
            }
        }
    }

    /// Delete one of the player's own work roles (a def's can't go: the
    /// next load would seed it back). The roles after it move down one, so
    /// the list stays what the UI and scripts index, with no gaps to skip;
    /// every colonist's index follows. Its members go to the default role
    /// and keep their pins. False if there was nothing to delete.
    pub fn delete_work_role(&mut self, role: u16) -> bool {
        if self.work_roles.get(role as usize).is_none_or(|r| r.def.is_some()) {
            return false;
        }
        self.work_roles.remove(role as usize);
        let default = self.default_work_role();
        for p in self.ecs.query_mut::<&mut Pawn>() {
            match p.work_role {
                Some(r) if r == role => {
                    p.work_role = default;
                    p.plan.clear();
                    p.proposal.clear();
                }
                Some(r) if r > role => p.work_role = Some(r - 1),
                _ => {}
            }
        }
        true
    }

    /// The role colonists start in: the colony's copy of the first role by
    /// order, or its first role; none when no mod defines any.
    pub fn default_work_role(&self) -> Option<u16> {
        let first = self.defs.default_work_role.map(|d| self.defs.work_roles[d as usize].id.as_str());
        first
            .and_then(|id| self.work_roles.iter().position(|r| r.def.as_deref() == Some(id)))
            .or((!self.work_roles.is_empty()).then_some(0))
            .map(|i| i as u16)
    }

    /// A colonist's work role: theirs, or the default for a colonist from
    /// before roles.
    pub fn work_role_of(&self, p: &Pawn) -> Option<u16> {
        p.work_role.filter(|&r| (r as usize) < self.work_roles.len()).or_else(|| self.default_work_role())
    }

    /// Whether a colonist's role is planned (Auto).
    pub fn is_planned(&self, p: &Pawn) -> bool {
        self.work_role_of(p).is_some_and(|r| self.work_roles[r as usize].planner.is_some())
    }

    /// Where a colonist's level for a work type starts: their plan's in a
    /// planned role, else their role's, or the work type's default. A pin
    /// is measured against this.
    pub fn inherited_priority(&self, p: &Pawn, work: DefId) -> u8 {
        let set = match self.work_role_of(p).map(|r| &self.work_roles[r as usize]) {
            Some(r) if r.planner.is_some() => p.planned(work).map(|x| x.level),
            Some(r) => r.level(work),
            None => None,
        };
        set.unwrap_or(self.defs.work_types[work as usize].priority).min(self.defs.priority_scale.levels)
    }

    /// Take a planner's proposal for a colonist in a planned role (DESIGN.md
    /// §4d). A level changes only when two proposals in a row agree, so
    /// nobody swaps jobs every hour; a work type with no plan yet takes the
    /// first. A reason follows the level it explains. The planner may not
    /// say never, nor plan a pinned cell: each such cell is refused, named
    /// in the returned errors, and keeps its plan.
    pub fn apply_plan(&mut self, pawn: hecs::Entity, proposal: &[(DefId, u8, String)]) -> Vec<String> {
        let levels = self.defs.priority_scale.levels;
        let defs = self.defs.clone();
        let planned = self.ecs.get::<&Pawn>(pawn).is_ok_and(|p| self.is_planned(&p));
        let Ok(mut p) = self.ecs.get::<&mut Pawn>(pawn) else { return vec!["not a pawn".into()] };
        if !planned {
            return vec![format!("{} isn't in a planned role", p.name)];
        }
        let mut errors = Vec::new();
        let mut seen = Vec::new();
        for (work, level, reason) in proposal {
            let (work, level) = (*work, *level);
            let wid = &defs.work_types[work as usize].id;
            if level == 0 || level > levels {
                errors.push(format!("{wid} for {}: a plan is a level from 1 to {levels}, not {level}", p.name));
                continue;
            }
            if p.own_priority(work).is_some() {
                errors.push(format!("{wid} for {}: pinned by the player", p.name));
                continue;
            }
            seen.push(work);
            let before = p.proposal.iter().find(|x| x.0 == work).map(|x| x.1);
            match p.plan.iter_mut().find(|x| x.work == work) {
                None => p.plan.push(Planned { work, level, reason: reason.clone() }),
                Some(x) if x.level == level => x.reason = reason.clone(),
                Some(x) if before == Some(level) => {
                    x.level = level;
                    x.reason = reason.clone();
                }
                Some(_) => {}
            }
            match p.proposal.iter_mut().find(|x| x.0 == work) {
                Some(x) => x.1 = level,
                None => p.proposal.push((work, level)),
            }
        }
        p.plan.sort_unstable_by_key(|x| x.work);
        p.proposal.retain(|x| seen.contains(&x.0));
        p.proposal.sort_unstable();
        errors
    }

    /// A colonist's level before the rules: their pin, or what they
    /// inherit. A level saved above a scale a mod has since shrunk counts
    /// as the last level.
    pub fn base_priority(&self, p: &Pawn, work: DefId) -> u8 {
        match p.own_priority(work) {
            Some(l) => l.min(self.defs.priority_scale.levels),
            None => self.inherited_priority(p, work),
        }
    }
}

/// A colonist's effective priority for a work type: 1 first, 0 never.
pub fn effective(w: &World, p: &Pawn, work: DefId) -> u8 {
    eval(w, p, work, &mut |_, _, _| {})
}

/// The effective priority and its parts, which sum to it: the work type's
/// default, the role if it sets one, the pin if there is one, then each
/// rule that moved it. A rule that holds but can't move it (a shift past
/// the last level) shows as 0, so the player sees it was considered.
pub fn explain(w: &World, p: &Pawn, work: DefId) -> (u8, Vec<Part>) {
    let mut parts = Vec::new();
    let v = eval(w, p, work, &mut |kind, label, delta| parts.push(Part { kind, label: label.to_string(), delta }));
    (v, parts)
}

fn eval(w: &World, p: &Pawn, work: DefId, part: &mut dyn FnMut(PartKind, &str, i32)) -> u8 {
    let (defs, rules) = (&w.defs, &w.rules);
    let levels = defs.priority_scale.levels as i32;
    let mut v = defs.work_types[work as usize].priority.min(defs.priority_scale.levels) as i32;
    part(PartKind::Default, "default", v);
    if let Some(role) = w.work_role_of(p).map(|r| &w.work_roles[r as usize]) {
        if role.planner.is_some() {
            if let Some(x) = p.planned(work) {
                let n = (x.level as i32).min(levels);
                part(PartKind::Plan, &x.reason, n - v);
                v = n;
            }
        } else if let Some(l) = role.level(work) {
            let n = (l as i32).min(levels);
            part(PartKind::Role, &role.label, n - v);
            v = n;
        }
    }
    if let Some(l) = p.own_priority(work) {
        let n = (l as i32).min(levels);
        part(PartKind::Pin, &p.name, n - v);
        v = n;
    }
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
        part(PartKind::Rule, label, n - v);
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
