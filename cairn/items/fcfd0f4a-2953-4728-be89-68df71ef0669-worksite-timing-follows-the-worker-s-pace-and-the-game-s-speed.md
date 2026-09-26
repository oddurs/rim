---
id: fcfd0f4a-2953-4728-be89-68df71ef0669
title: Worksite timing follows the worker's pace and the game's speed
type: bug
status: done
milestone: building
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: none
effort: s
layer: client
area: render
pillar:
- plugin-first
---

## What happens

Skills (#103) and tools (#101) made work per tick vary: `work_on` takes the pawn's `amount`, and a tool shortens the total. The worksite client (#97) still assumes one work a tick:

- A worker's wind-up is timed as if the next strike were `every - done % every` ticks away, so a skilled worker lunges early, or not at all.
- The readout's seconds are `(total - done) / 60`: wrong for a fast or slow worker, and wrong at 2× or 3× speed.

## Fix

The client measures each site's pace from how much its `done` moved since it last looked, and times the wind-up by it. The readout divides by pace and game speed, and says "paused" when the game is.

## Acceptance criteria

- [x] The wind-up is timed from the observed pace, and a test at 2 work a tick shows it
- [x] The readout's time left accounts for pace and game speed
