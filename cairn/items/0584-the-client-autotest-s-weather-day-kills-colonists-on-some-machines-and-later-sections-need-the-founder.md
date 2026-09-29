---
id: 584
uid: 259b6390-acc7-4320-9a17-3e6767ea83d9
title: The client autotest's weather day kills colonists on some machines, and later sections need the founder
type: bug
status: done
milestone: crafting
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p0
api: none
---

## What happens

Main's client autotest panics in the camera section ("pawn exists: NoSuchEntity"): the founder is dead. The weather section forces a storm and skips a day, with colonists out in it. Frames run the sim in real time, so how many ticks pass depends on the machine. On seed 7, Ines died inside the day skip (tick 28,412) and the founder during the pinned scenarios (30,153). Snow and mud (#277) made it likelier: with it reverted the run passed, but took another path.

## What should happen

The harness keeps its colonists alive through weather it forces. Later sections select, draft and follow the founder. Dying in an unsheltered storm day is fair in play.

## Acceptance criteria

- [x] The autotest runs to the end on seed 7
- [x] The weather section's own checks are unchanged

## 2026-09-28

The day skip runs pinned at 16° and dry. The storm check only needs the forecast's head, and the skip check only needs the tick count, so both stay as they were. T::keep_well refills every colonist's needs and hp after the skip, on each 100-tick step of the wait for night, and at the section's end. On c01d0380: 331 passed, 0 failed, seed 7, --background, at load 60-100.
