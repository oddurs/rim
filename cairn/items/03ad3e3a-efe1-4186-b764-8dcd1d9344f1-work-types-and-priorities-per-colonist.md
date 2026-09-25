---
id: 03ad3e3a-efe1-4186-b764-8dcd1d9344f1
title: Work types and priorities per colonist
type: feature
status: backlog
milestone: colony
created: 2026-09-22
updated: 2026-09-25
priority: p0
api: additive
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

- [ ] Work types and levels load from core and can be patched
- [ ] A colonist with Build 1 and Haul 2 builds while any build work is reachable (test)
- [ ] Priority 0 is never chosen, including by right-click fallbacks that aren't forced
- [ ] Determinism test passes with priority changes in the command log

## 2026-09-24

Split on 2026-09-23 per DESIGN.md §4d: work pools (0e73145a), the Work Board (f1b96df4), the why panel (f1924f03), rules and stances (0cb48faf).

## 2026-09-25

The stone age adds work that needs a work type: gathering (the gather designation) and crafting (orders posted by the crafting plugin, 049e2f73). Orders name their work type, so the Work Board gets a crafting column with no special case.
