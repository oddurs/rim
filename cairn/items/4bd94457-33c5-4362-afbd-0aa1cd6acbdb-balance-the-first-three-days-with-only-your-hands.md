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
