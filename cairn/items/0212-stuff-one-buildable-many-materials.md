---
id: 763d8d64-9bee-459a-87c4-1ec543644f38
title: 'Stuff: one buildable, many materials'
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: building
---

## Why

A "wooden wall" and a "stone wall" are two unrelated defs that happen to
look alike. Six materials times ten buildables is sixty defs, and a mod
adding one material has to write ten of them.

## What

- `build.stuff = { category = "structural", count = 25 }` replaces a fixed
  `cost` for buildables that can be made of anything. `cost` stays for
  things with a fixed recipe.
- `[[thing]]` gains `stuff = { categories = [...] }` on items, so wood
  declares itself structural rather than walls listing every wood.
- The material is chosen when the blueprint is placed and travels on the
  `Command`, so replays and lockstep still work.
- `MadeOf(DefId)` on the blueprint and the finished thing.
- Delivery and refund resolve against the chosen material, not a fixed
  cost list.

## Acceptance criteria

- [x] One `wall` def buildable in wood or stone
- [x] A mod adding an item with a `stuff` category makes it buildable with
      no other change
- [x] Cancel refunds the material actually delivered
- [x] Determinism holds: the choice is in the command log

## 2026-09-23

Stopped mid-item at the user's request. Work is on branch feat/0212-stuff (pushed, no PR). State: schema done (BuildDef.stuff, ThingDef.stuff with categories+factors, load-time validation that a buildable has exactly one of cost/stuff); Blueprint now owns its resolved cost so nothing downstream cares whether it came from a recipe or a choice; MadeOf lands on blueprint and finished thing; Command::Build carries the chosen material so replays rebuild the same wall; Cancel refunds what was delivered of what it was made of; spawn_fixture_of is the material-aware constructor. Core's wall_wood and wall_stone are now one 'wall' taking 5 of anything structural; wood and stone declare categories and factors. Client picks the material the colony has most of as a stopgap until 0215.

KNOWN FAILURE, not yet fixed: tests/fields.rs cold_colonists_go_to_the_fire. Passes on main, fails on the branch ('ended up somewhere warm: 6 deg'). No dangling wall_stone references -- the cause is that deleting a thing def shifts every later DefId, which changes what mapgen and plant spread produce for the same seed, so that seed-sensitive test lands the colonist somewhere colder. Two honest options: re-pin the test's expectations, or make it find the fire rather than assume the geometry. Worth deciding deliberately, because any future def collapse will do this again -- it is an argument for that test not keying on a seeded world.

## 2026-09-23

Resumed and finished. The fields.rs failure was not a DefId problem after all: cold_colonists_go_to_the_fire asserted the pawn's temperature at a fixed tick 1500, and on main the pawn was still ~50 ticks short of finishing its Comfort job. The def collapse shifted mapgen so the fire landed west instead of east and the pawn started 330 warmth higher, finishing early and wandering off to a cold cell before the assert. The test now asserts what it means -- sought warmth, stood somewhere >= 11 degrees while warming, and peaked above half -- so it no longer depends on a race. Blueprint now owns its resolved cost, so order.rs, ai.rs, command.rs, rim_ui and the client all stopped reading cost off the def; that also fixed the blueprint hover text and progress bar for stuff walls, which would otherwise have shown an empty cost. Six tests in tests/stuff.rs, the last of which is the sprint's whole thesis: mods/marble as one item def builds a wall with no engine change.
