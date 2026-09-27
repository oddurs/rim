---
id: 704889c9-7531-42de-a010-79202962ebff
title: Research as a plugin
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: additive
effort: l
layer: plugin
area: modding
pillar:
- plugin-first
---

## Why

Tech progress without engine knowledge of research.

## Acceptance criteria

- [x] Research gates buildables through the stat pipeline

## 2026-09-27

The engine gets a small, generic stat pipeline, and research is its first reader.
- [[modifier]] def kind: id, stat, thing, value, reason, group, on (default true). Modifiers are indexed per thing at load. One on a thing no loaded mod defines is a warning and does nothing, so research can gate the optional timber and iron.
- World::def_stat(def, stat) is the def's base (hp, work, value, and buildable = 1 for a buildable) plus every modifier on it that is on, summed in load order. World::stat(e, …) now reads def_stat, so with no modifiers nothing changes.
- World::build_lock(def) gives the reason when buildable is 0 or less. command::build_preview returns Blocker::Locked for the whole order as an early return, before the per-cell loop, as calm-forest asked, so #241's Replaces can't slip a locked thing in. That covers Build and each PlacePlan piece.
- rim.set_modifiers(group, on) switches the calling mod's modifiers in the group, plus any mod's whose group is '<caller>:<group>'. Switches live in World.modifiers_switched by id: saved in the world section (only when non-empty, so older saves read as before) and hashed. Also rim.stat_of and rim.modifier_defs.
- UI: ToolView.locked and view.tools' locked; a locked tile shows the reason in place of its cost, and the card and the pill say it.
Research plugin (mods/research, depends core, optional timber and iron):
- [[research.project]] data (label, note, work, requires), a Research skill and work type, and a research desk (production menu, 15 structural).
- Projects: Joinery (shelf, tool rack), Granaries (granary, after joinery), Smithing (forge).
- scripts/research.luau posts 200-work orders at every desk for the chosen project, adds each finished one to progress, and on reaching the work switches the project's group off and says so.
- A Research screen in the top bar, and the desk's inspector line.
Tests: tests/research.rs covers locked before, placed after studying, progress and unlocks surviving save/load with equal hashes, the engine modifier mechanics, and the guide samples. api=additive: new engine kind, new rim and ui API, nothing removed.

## 2026-09-27

Two changes from the gate.
(1) There's no Research skill. The work type has none, like hauling. A new skill adds a draw per pawn at creation, which moved seed 21's world in tests/furniture.rs and seed 7's in the UI tests. A study skill can come with its own tuning.
(2) The eighth work type overflowed the inspector's Work tab (the steady test). This PR carries the exact hunk from cooking (#236): a 300 slot and a scrolling list. Whichever of the two merges second rebases cleanly, since the changes are identical.

## 2026-09-27

A modifier on a thing whose mod isn't installed is silent, as a patch to it is. rim check of research alone had warned for timber and iron. One on a loaded mod's missing thing still warns.
