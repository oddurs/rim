---
id: 540
uid: dbb92ebe-dc7b-4217-87f8-d64567858117
title: 'One ladder for contested slots: a compatibility mod, the player''s pick, then labelled load order'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: l
layer: engine
area: modding
pillar:
- plugin-first
- determinism
---

## Why

When two mods want the same slot, rim gives five different answers across
nine mechanisms:

| Slot | Today |
|---|---|
| A built-in singleton (calendar, sky, priority scale, store priority) | load error: patch it instead (`modloader.rs`) |
| `[[start]]` | the last one loaded wins, silently (`defs.start = Some(..)`) |
| `rim.on_generate_level(z)` | load error naming both mods (`script.rs`) |
| A field two patches `set` | warning; load order decides (`merge`) |
| A list one mod replaced after another edited it | warning; load order decides |
| `ui.replace`, `ui.remove`, `ui.window` on one id | warning; load order decides (docs/modding/ui.md) |
| A theme token | warning; load order decides |
| A room role | the first in load order, no warning |
| A claim (96db8898, planned) | the player picks; no pick blocks a new game |

Nine rules is nine things to learn, and some are silent. DESIGN.md §10,
"Tension: when two mods want one slot".

## What

A **contested slot** is one field of one def, one UI id, one theme token,
one level generator, or one claim tag, wanted by two or more mods. Every
mechanism above reports one kind of record,
`Contest { slot, contenders, winner, rung }`, and resolves it by the first
rung that applies:

1. **A compatibility mod.** A mod that has every contender in its `depends`
   or `optional` and sets the slot itself wins. The report says "settled by
   <mod>", and `rim check --strict` doesn't fail on it. Load order already
   puts that mod last; what changes is that no one is asked.
2. **The player's pick**, as `{ slot, winner, contenders: [(id, version)] }`.
   Sim picks live in the save's epoch (then `mods.lock`, 0152). Client picks
   (UI ids, tokens) live in the player's settings. When a contender's
   version changes, its pick is dropped and the slot is reported again, so
   no stale answer is reused.
3. **Load order**, as today, reported as "decided by load order" in F3 and
   listed on the New colony screen (73751f4f) with a Pick link.

A contest never blocks a game from starting.

**What changes per mechanism**

- A second `[[start]]` becomes a contest instead of winning silently (until
  premises replace it).
- A second `on_generate_level` for one level becomes a contest instead of a
  load error.
- Claims (96db8898) use the ladder instead of blocking New colony.
- Redefining a singleton stays a load error: that's an authoring mistake.
  Two patches to its fields are the contest.
- Room roles leave load order entirely (04d659f3).

## Acceptance criteria

- [ ] Each mechanism in the table reports a `Contest` with slot, contenders, winner and rung, in F3 and in `rim check` (a test per mechanism)
- [ ] A mod depending on both contenders that sets the slot wins at rung 1, and `rim check --strict` passes (test)
- [ ] A pick overrides load order; changing a contender's version drops the pick and reports the slot again (test)
- [ ] Two generators for one level load, and a pick or load order chooses (test)
- [ ] No contest prevents New colony from starting
- [ ] docs/modding/patches.md and ui.md describe the ladder where they describe conflicts today
