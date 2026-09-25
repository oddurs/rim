---
id: 03ad3e3a-efe1-4186-b764-8dcd1d9344f1
title: Work types and priorities per colonist
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-25
closed_at: 2026-09-25
priority: p0
api: breaking
effort: m
layer: engine
area: ai
pillar:
- growth
---

## Why

With several colonists, who does what must be the player's choice. This is the core of DESIGN.md §4d: the data and the rule for choosing, which the Work Board, work pools, rules and the why panel build on.

## What

- `[[work_type]]` in core: id, label, icon, skill, default priority, `order` (the tie-break). Blueprints, designations and hunts each belong to one.
- `[[priority_scale]]` in core: `levels = 4`; 0 means never. A mod raises it to 9 with one patch.
- A priority per colonist per work type, stored in the world and in the state hash, changed only by `Command::SetPriority`.
- Choosing work: work types in priority order, then `order`, and the first level with reachable work wins. Nearest inside it for now; the work pools item replaces the scan and adds the score.
- `rim.priority(pawn, work)`, `view.priorities()` for the UI.
- A temporary grid in core's UI so it can be played before the Work Board lands.

## Acceptance criteria

- [x] Work types and levels load from core and can be patched
- [x] A colonist with Build 1 and Haul 2 builds while any build work is reachable (test)
- [x] Priority 0 is never chosen, including by right-click fallbacks that aren't forced
- [x] Determinism test passes with priority changes in the command log

## 2026-09-24

Split on 2026-09-23 per DESIGN.md §4d: work pools (0e73145a), the Work Board (f1b96df4), the why panel (f1924f03), rules and stances (0cb48faf).

## 2026-09-25

The stone age adds work that needs a work type: gathering (the gather designation) and crafting (orders posted by the crafting plugin, 049e2f73). Orders name their work type, so the Work Board gets a crafting column with no special case.

## 2026-09-25

Built. Designations now require work_type; the engine's own jobs are claimed by a work type's jobs list (only 'build': construct and deliver), so no content id sits in the engine. Pawn.priorities holds only what the player changed, sorted by work type; anything else is the def's default, so a mod's new work type appears on every colonist at its default. find_work keeps its scans (the pools item replaces them) but gathers the nearest job per work type, skipping types at 0, then picks by (level, distance, order, id): within a level the nearest wins and order only breaks ties, per DESIGN.md §4d's criticism of leftmost-wins. Right-click orders stay forced and ignore priorities (DESIGN.md: a right-click order still forces it); nothing the AI picks for itself runs at 0. rim.priority(id, work) for scripts; view.work_types/priorities/priority_levels and act.set_priority for the UI, so the UI API goes to 0.3 (additions are breaking before 1.0). Temporary grid: mods/core/ui/work.luau, key P. The strings-through-one-door test now scans every core UI script rather than a fixed list, which had missed title.luau too.

## 2026-09-25

Review: text saves (savetext, from rim save) wrote work types as raw indices; DEF_REFS now maps pawn priorities and SetPriority to ids. A saved level above a scale a mod has since shrunk now counts as the last level. And a required work_type on designations breaks existing mods, so the sim API goes to 0.5 as well as the UI API to 0.3.
