---
id: 8b2e05db-9ea0-4d67-b62b-13fb8c9f4ae2
title: 'Work choice: an urgency term, so shelter beats nearby grass'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: none
effort: m
layer: engine
area: ai
---

## Why

DESIGN §4d says that within a priority level "an integer score picks: distance, urgency, skill fit". `find_work` scores distance only. The stone-age sweep (4bd94457) measured the cost: at the default priorities, a lone colonist spent 8,000 of day one's 11,667 ticks gathering grass beside them, while the hut that night needed went unbuilt. A player can answer it by setting Build to 1, but the default colonist should know that a planned shelter before dark beats another armful of fibre.

## What

- An urgency term in `find_work`'s score within a level. Sources: a blueprint for an enclosing wall or a bed with night coming and no shelter, food with hunger low, a threat in the colony.
- The sweep (`examples/stone_age.rs`) run at default priorities as the measure.

## Acceptance criteria

- [x] At default priorities, the sweep's shelter-before-the-first-night share is within 10 points of the build-first bot's
- [x] Nothing walks across the map for one pebble: distance still dominates between equally urgent jobs

## 2026-09-26

Built in choose_work: within a level the key is the distance plus CALM (1<<24) for work that isn't urgent, so an urgent job at a level beats any calm one, and distance still decides between equally urgent ones; a better priority level still beats urgency. Urgent: a plan for a wall, a door (blocks or door) or a bed while the colony has no built bed in an enclosed room (World::has_shelter), and a plan for something that comforts a need (ThingDef.comforts: it emits a positive amount into a field a Field-satisfied need reads, or that field's terms read; core's campfire warms temperature, which feels_like reads) while the colony has none built (World::has_comfort); and clearing the ground for either (Planned). Both checks run at most once per choice and only when a build candidate asks. No content ids in the engine. find_work refreshes rooms before choosing. Measured (examples/stone_age.rs, new --defaults flag leaves every priority at its default), 20 seeds x 5 days: campfire and an enclosed bed before the first night, defaults 45% before -> 100% after; the build-first bot 90% -> 90%. Shelter alone lifted defaults to 60%; the campfire was the rest. balance example, 60 seeds x 7 days: lost 3 vs 3, runs with a death 9 vs 8, froze 39.6 vs 40.0 h. (On main that harness no longer finishes a hut on day 1 on any seed: its bot builds wooden walls, and chopping needs an axe with primitive loaded. Not this change.) Food with hunger low and a threat in the colony, the item's other sources, are left: needs already put food first, and threats have the draft.

## 2026-09-26

Review fix: find_work no longer calls map.ensure_rooms(). A rebuild mid-tick renumbers rooms under the fields' room values (the reviewer measured a warm hut reading 11.97° instead of 23.02° after a mid-tick rebuild and one more wall), so has_shelter reads the rooms as built at the start of the step, at most a tick stale. Regression test: choosing work never rebuilds rooms mid-tick (urgency.rs), which fails with the call. Re-measured after: defaults 100%, bot 90%.
