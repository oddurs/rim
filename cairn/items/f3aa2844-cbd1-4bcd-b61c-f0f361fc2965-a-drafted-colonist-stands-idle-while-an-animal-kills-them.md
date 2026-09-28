---
id: f3aa2844-cbd1-4bcd-b61c-f0f361fc2965
title: A drafted colonist stands idle while an animal kills them
type: bug
status: backlog
milestone: defense
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: s
layer: engine
area: combat
---

## Problem

A drafted colonist with no order stays `Idle` while a wolf attacks them from the next cell, and dies. Found on the random-streams branch (#285) in `water_depth::colonists_leave_rising_water_and_nobody_drowns`: Tarn, drafted, 100 hp at tick 7500, is attacked by a `core:wolf` from tick ~8000 at (34,33,0) and dies at 8630 without a swing. Undrafted, the flee rules would move them; drafted, nothing responds. RimWorld's drafted pawns return fire at what attacks them, and players expect that.

## Proposal

- A drafted pawn with no job fights back at an adjacent attacker (melee), and doesn't pursue beyond reach: drafting keeps them where the player put them.
- Undrafted behaviour is unchanged.

## Acceptance criteria

- [ ] A test: a drafted colonist next to an attacking wolf swings at it within a few ticks and doesn't leave their cell
- [ ] A test: an undrafted colonist's response to the same wolf is unchanged
- [ ] The water_depth test's scenario, run with the wolf, ends with the wolf driven off or dead, not Tarn
