---
id: 4bd94457-33c5-4362-afbd-0aa1cd6acbdb
title: 'Balance: the first three days with only your hands'
type: chore
status: backlog
milestone: stone-age
depends_on:
- e3846c47-f425-46f5-8e96-fec056074052
created: 2026-09-25
updated: 2026-09-25
priority: p0
api: none
effort: m
layer: plugin
area: tests
---

## Why

The stone age makes the first days harder on purpose, and "four walls before the second night" (§4) has to stay true. d447889b balanced core's first three days, and this does the same with `mods/primitive` on. It's measured, not guessed.

## What

- **A seed sweep** (`scripts/autotest-sweep.sh` or a headless example) of a lone naked colonist with core, weather and primitive, and no player input beyond a scripted plan: gather, build a branch shelter and campfire, find flint, knap, fell.
- **Record per seed:**
  - the tick of the first campfire, the enclosed room and the bed;
  - the first flint and the first hand axe;
  - the first felled tree and the first quarried stone;
  - the first clay and the first cob wall.
- Tune densities, yields, work and wear until the targets hold. Record the numbers in the item.

## Acceptance criteria

- [ ] Campfire and an enclosed bed before the first night in at least 90% of seeds 1 to 20
- [ ] A flint tool by the end of day 2 in at least 80%
- [ ] A felled tree by the end of day 3 in at least 80%
- [ ] Cob walls (clay) by the end of day 4 in at least 60%
- [ ] Numbers recorded, and densities set from them

## 2026-09-25

Measured on 2026-09-26, while landing clay (89f6d138):
- Nearest flint nodule from the start, seeds 1 to 10: 26, 40, 40, 45, 5, 10, 50, 46, 32 and 60 cells (4 to 17 nodules per map).
- Flint is gathered whole, so a map's supply is fixed.
- Crosscheck seed 1 with clay banks: the lone founder died on day 3, after making the first axe and chopping 60 wood, with no hut up.
The first days still need their pacing measured, not assumed.

## 2026-09-25

Correction to the note above: it was measured today, the 25th. The seed-1 founder was killed by wolves (hurt on day 2, dead on day 3), not by cold. Across crosscheck seeds 5, 6, 7, 9, 11 and 12, every colony is gone by day 45; seed 9 lasts longest, with 7 colonists at day 30. The crosscheck now defaults to seed 9. The storyteller's pressure on a stone-age colony belongs in this sweep.
