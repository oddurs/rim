---
id: f3aa2844-cbd1-4bcd-b61c-f0f361fc2965
title: A drafted colonist stands idle while an animal kills them
type: bug
status: done
milestone: defense
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] A test: a drafted colonist next to an attacking wolf swings at it within a few ticks and doesn't leave their cell
- [x] A test: an undrafted colonist's response to the same wolf is unchanged
- [x] The water_depth test's scenario, run with the wolf, ends with the wolf driven off or dead, not Tarn

## 2026-09-28

Criterion 3: the water_depth scenario's wolf came from #285's RNG streams and isn't reproducible on main, so the same situation is tested directly: tests/animals.rs a_drafted_colonist_drives_off_a_hunting_wolf (drafted founder, a wolf 5 cells off, 5000 ticks: the founder lives, the wolf is dead or gone). predator_hunts_nearby_people now asks the wolf whom it attacks, since the drafted founder consumes last_attacker when turning on it.

## 2026-09-28

The user confirmed the behaviour (2026-09-28): a drafted colonist defends itself when attacked but doesn't pursue. The fix ends a drafted pawn's self-defence attack as soon as the target is out of reach; only an ordered attack chases.
