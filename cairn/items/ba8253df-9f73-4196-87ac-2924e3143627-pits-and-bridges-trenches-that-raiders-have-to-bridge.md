---
id: ba8253df-9f73-4196-87ac-2924e3143627
title: 'Pits and bridges: trenches that raiders have to bridge'
type: feature
status: backlog
milestone: depth
depends_on:
- acd85584-7f3d-4348-8d56-5242a0bdb620
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: combat
pillar:
- wealth-gravity
---

## Why

A trench is the stone-age wall: you can dig one before you can build stone. But a closed ring of air that nothing can answer makes raids pointless, which is §1's failure again (DESIGN.md §6d).

## What

- Air is impassable at its level, and nothing climbs out of a pit. Items dropped on air fall to the level below.
- **Bridges:** a floor laid over air, with hp. A raider who can't reach anything carries planks and bridges a cell at a time, as raiders break doors today. Defenders can knock a bridge down.
- **Drawbridge:** an owned floor over air, passable to its owner and air to everyone else. It uses the owned-door rule through `fix_owner`.
- Trench cells count toward defensive strength in the raid budget, like walls.
- **Movement classes:** a `[[movement]]` def (`drop = 1`) whose region layer treats a one-level drop as passable. Region layers go from per faction to per faction and class. Core ships no climbers; the class exists for mods.

## Acceptance criteria

- [ ] A raid against a colony ringed by a trench bridges it and arrives (scene test)
- [ ] The colony's own drawbridge lets colonists out and keeps raiders off
- [ ] Balance: 40 seeds with a bot that trenches by day 5. Raid outcomes before and after are recorded here, and bridging speed is set from them.
- [ ] A fixture mod's climber crosses a one-level drop; the extra flood fill's cost is recorded here
- [ ] Determinism test passes
