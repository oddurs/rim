---
id: 0cb48faf-617a-4a64-922a-d6ebb842145a
title: Priority rules and colony stances
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- 03ad3e3a-efe1-4186-b764-8dcd1d9344f1
created: 2026-09-24
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: additive
pillar:
- growth
effort: m
layer: engine
area: ai
---

## Why

Players rewrite the whole grid for winter and again for a siege. Rules change priorities when something holds, and stances switch a set of rules with one click (DESIGN.md §4d).

## What

- `[[priority_rule]]`: `when` (hour range, season, alert, need, stance, or a cached Luau predicate) and `shift` or `set` per work type.
- `[[stance]]`: a named set of rules. Core ships Normal, Harvest, Winter prep and Siege; mods add more.
- Effective priority = base + active rules, with a breakdown per cell.
- A stance bar on the Work Board, and `rim.set_stance` for scripts and incidents.

## Acceptance criteria

- [x] Effective priorities explain themselves, and the parts sum to the value
- [x] Rules re-evaluate only when their inputs change (measured)
- [x] A mod adds a stance with data alone
- [x] Two runs with stance changes hash identically

## 2026-09-25

Built as crates/rim_sim/src/rules.rs. Defs: [[stance]] (id, label, icon, order; the first by order is a new colony's) and [[priority_rule]] (when = { hours = [a, b] wrapping past midnight, season = [...], stance, need + below/above as a fraction }; set = { work = level }; shift = { work = delta }, negative is sooner). A rule names its stance, not the other way round, so a mod can add rules to core's stances without patching them. Effective = base, then each rule that holds in load order: a set puts it at a level, a shift moves it within 1..levels, and a player's 0 (never) stays 0 under a shift; only a set overrides never. explain() returns the parts, which always sum to the value (a rule that holds but can't move it shows 0). Colony-wide conditions are worked out in World::update_rules, keyed on (hour, season, stance) with the parts no rule reads left at 0: with core's rules, which read only the stance, it runs once per stance change and never per hour (measured in tests/stances.rs: 0 evaluations over a day, 1 per SetStance, 23 over a day with an hour rule). The need half is checked per colonist in find_work. The stance is in WorldSection (serde default, remapped by id; a removed mod's stance falls back to the first, with a note), the state hash, the def table and the text save; changes come by Command::SetStance or rim.set_stance from scripts. Script API: rim.priority is now effective, plus rim.priority_parts, rim.stance and rim.set_stance. UI API 0.6: view.stances, view.effective (value and why), act.set_stance. core:work has a stance bar and cells read 3→1 with the explanation as a tooltip. Alerts don't exist in the engine yet, and Luau predicates want their own caching design: split to 99398b15 (plugin-api).
