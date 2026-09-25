---
id: a14a5ff6-adad-488c-9810-8a02f6409b7c
title: Work progress lives on the thing
type: feature
status: doing
milestone: building
assignee: Oddur Sigurdsson
claimed: 2026-09-25
created: 2026-09-25
updated: 2026-09-25
priority: p1
api: additive
effort: m
layer: engine
area: sim
pillar:
- plugin-first
---

## Why

Chop, mine and take-down progress is a counter in the pawn's job (`Job::Harvest { work }`, `Job::Deconstruct { work }`), so an interrupted job starts over and nothing can draw it. Building already keeps its progress on the thing (`Blueprint.work_left`). DESIGN.md §6b: progress lives on the thing, and idle costs nothing.

## What

- `Work { done, total, designation, side }` on the worked thing, saved in an optional `engine:work` section. `designation` says which harvest the progress belongs to (None for a build), so gathering after a half-chop starts fresh.
- `Job::Harvest` and `Job::Deconstruct` drop `work` and add to the target's `Work`. `Blueprint` drops `work` and `work_left`; `Work` is inserted when the plan is placed. An old save's blueprint progress is carried over on load.
- `World::worksites`: entity to the last tick it was worked, not saved. The map is touched when a site starts or stops being worked, never on progress, so the client can cache idle sites.
- Material deliveries touch the map, so a cached plan shows them.
- `stage(w, e) -> u8`, 0..=8: the larger of work done and hp lost.

## Budget

No new per-tick work: the same integer is incremented, on the thing instead of the job. Two map touches per work session, and the worksite sweep is O(sites worked this tick).

## Acceptance criteria

- [ ] An interrupted chop resumes where it stopped, with another pawn
- [ ] Take-down progress survives the job ending
- [ ] Blueprint progress lives in `Work`, and an old save's plan keeps its progress
- [ ] `Work` survives a save and load, and the determinism test passes
- [ ] The map is touched when a site starts and stops being worked, and not on progress
- [ ] `stage()` reads work and hp
