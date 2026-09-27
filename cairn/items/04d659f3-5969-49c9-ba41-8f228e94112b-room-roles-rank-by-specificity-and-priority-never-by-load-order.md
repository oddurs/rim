---
id: 04d659f3-5969-49c9-ba41-8f228e94112b
title: Room roles rank by specificity and priority, never by load order
type: feature
status: backlog
milestone: plugin-api
depends_on:
- dbb92ebe-dc7b-4217-87f8-d64567858117
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: additive
effort: s
layer: engine
area: sim
pillar:
- plugin-first
---

## Why

"In load order, the first role a room meets names it" (DESIGN.md §6c,
docs/modding/vocabulary.md). Every plugin depends on core, so core's four
roles always come first: no plugin can offer a better fit for a two-bed room
than core's dormitory. Among plugins, order breaks ties by id, so the
plugin whose id sorts first wins, and renaming a mod changes what rooms are.

Paradox shows where this leads: filenames compared in ASCII order decide
outcomes, and players prefix mod names with `!` and `~` to steer them.

## What

- The candidates are the roles whose `needs` the room meets and whose limits
  (`min_cells`, `enclosed`) allow.
- They are ranked by:
  1. **specificity**: the total count of needed tags, so `bed = 2, fire = 1`
     beats `bed = 2`;
  2. an optional `priority` (int, default 0), higher first;
  3. otherwise a contested slot (dbb92ebe), reported and settled by the ladder.
- Core's four roles get priorities that keep today's result on every room
  core's own tests build.
- Load order never enters the ranking.

## Acceptance criteria

- [ ] A plugin role with more needs than core's beats it for a room that meets both (test)
- [ ] Renaming a plugin's id changes no room's role (test)
- [ ] Two roles equal in specificity and priority are reported as a contest (test)
- [ ] Core's existing room tests pass unchanged
- [ ] DESIGN.md §6c and docs/modding/vocabulary.md describe the ranking
