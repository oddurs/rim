---
id: ba8253df-9f73-4196-87ac-2924e3143627
title: 'Pits and bridges: trenches that raiders have to bridge'
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- acd85584-7f3d-4348-8d56-5242a0bdb620
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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
- Movement classes (climbers that drop a level) are split out to 3fea3b3d.

## Acceptance criteria

- [x] A raid against a colony ringed by a trench bridges it and arrives (scene test)
- [x] The colony's own drawbridge lets colonists out and keeps raiders off
- [x] Balance: 40 seeds with a bot that trenches by day 5. Raid outcomes before and after are recorded here, and bridging speed is set from them.
- [x] Determinism test passes

## 2026-09-27

Split movement classes into 3fea3b3d: a second region layer per class is its own engine change, and nothing in core needs it for trenches to work.

## 2026-09-27

Balance (examples/balance: 40 seeds, 8 days, 3 colonists, --tools since the bot crafts none, a raid forced on day 6 through the storyteller). Without a trench: raiders got inside the hut's radius in 30/40 runs, 32 colonist deaths from the raid day on, 0 colonies lost. With --trench 3 (a pit ring 5 cells out and a drawbridge before the door; whole in 28/40 runs by a mean day 3.15, the drawbridge up in 34/40): bridge work 120 → raiders inside 20/40 (15/28 with the ring whole), 13 deaths, 2 lost; 360 → 15/40 (11/28), 14 deaths, 1 lost; 1080 → 16/40 (13/28), 14 deaths, 1 lost. About 0.6 bridges stand at the end of a run. At 20,000 ticks a day even 1080 ticks is 1.3 hours against a raid that lasts about a day, so on a one-cell trench the speed hardly matters: the trench's worth is that raiders stop to bridge where colonists can meet them, and raid-day deaths fall by about 60%. Set: raiders bridge at the bridge's own work (120), no multiplier. The lost colonies with a trench starved or froze shut in when the drawbridge never stood (6/40 runs); that is the bot's, a player would see it.
