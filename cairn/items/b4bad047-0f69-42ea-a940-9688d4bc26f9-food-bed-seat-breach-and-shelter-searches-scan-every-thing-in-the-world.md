---
id: b4bad047-0f69-42ea-a940-9688d4bc26f9
title: Food, bed, seat, breach and shelter searches scan every thing in the world
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
layer: engine
area: ai
---

## What

Each of these walks every `Thing` (ECS query) per call:
- `find_food` (ai.rs ~483), per hungry colonist think: every thing, to find food items and forageable plants;
- `nearest_spot` (~559), per sleep or seated meal: every thing, for beds and seats;
- `nearest_breach` (~264), per walled-out raider think: every owned thing;
- `World::has_shelter` and `has_comfort` (world.rs ~1293, ~1386): every thing, once per work choice when a build's urgency is asked.

## Why it matters

Each is O(things) per call, so O(pawns × things) per round of thinking. A grown colony has tens of thousands of things (plants, rocks stood up, stacks, walls).

## Direction

- Food items: the stock's chunk holdings already exist.
- Forageable plants, beds and seats: small indexes by def and chunk, kept on spawn and despawn.
- Shelter and comfort: a flag worked out when rooms or those things change.
- Owned blockers: by chunk, for breach.

## Acceptance

- [ ] None of these scans every thing per call
- [ ] Same answers as today (brute-force comparison tests)
- [ ] The scaling bench shows the colonist think cost flat in world thing count
