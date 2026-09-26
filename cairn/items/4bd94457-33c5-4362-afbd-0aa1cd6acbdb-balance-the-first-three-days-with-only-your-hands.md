---
id: 4bd94457-33c5-4362-afbd-0aa1cd6acbdb
title: 'Balance: the first three days with only your hands'
type: chore
status: doing
milestone: stone-age
assignee: Oddur Sigurdsson
claimed: 2026-09-25
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

- [x] Campfire and an enclosed bed before the first night in at least 90% of seeds 1 to 20
- [x] A flint tool by the end of day 2 in at least 80%
- [x] A felled tree by the end of day 3 in at least 80%
- [x] Cob walls (clay) by the end of day 4 in at least 60%
- [x] Numbers recorded, and densities set from them

## 2026-09-25

Measured on 2026-09-26, while landing clay (89f6d138):
- Nearest flint nodule from the start, seeds 1 to 10: 26, 40, 40, 45, 5, 10, 50, 46, 32 and 60 cells (4 to 17 nodules per map).
- Flint is gathered whole, so a map's supply is fixed.
- Crosscheck seed 1 with clay banks: the lone founder died on day 3, after making the first axe and chopping 60 wood, with no hut up.
The first days still need their pacing measured, not assumed.

## 2026-09-25

Correction to the note above: it was measured today, the 25th. The seed-1 founder was killed by wolves (hurt on day 2, dead on day 3), not by cold. Across crosscheck seeds 5, 6, 7, 9, 11 and 12, every colony is gone by day 45; seed 9 lasts longest, with 7 colonists at day 30. The crosscheck now defaults to seed 9. The storyteller's pressure on a stone-age colony belongs in this sweep.

## 2026-09-25

Bone (61c93a4e) opens the chopping gate without flint. A bare-handed hunt of two hares or one deer is enough for a bone hand axe, at 0.42 speed and 48 hp against flint's 0.6 and 60. Deer outnumber flint nodules by far. Weigh that in the sweep.

## 2026-09-25

Measured with crates/rim_sim/examples/stone_age.rs, seeds 1 to 20, 5 days (seeds 1 to 40 in brackets):
- campfire and an enclosed bed before the first night (14 h): 90% (92%)
- a flint tool by the end of day 2: 95% (98%)
- a felled tree by the end of day 3: 100% (100%)
- cob walls by the end of day 4: 95% (92%)

Before tuning: 0%, 65%, 0% and 50%.

What moved them:
- The bot plays like a player. It puts Build at 1 and Craft at 2 (find_work picks the nearest job at equal levels, so a default colonist gathered grass for 8,000 of day one's 11,667 ticks). It puts Chop at 2 once armed. It builds a 3x3 first hut on ground clearable by hand, and marks the nearest flint and stones.
- Oak branches went from 3 per 60 work to 5 per 45.
- Deadfall went from 4 branches to 6, and spawns on grass, dirt and rich soil at 3%, up from 2%.
- Clearing a planned thing now counts as build work (find_work).

Not tuned: storyteller pressure, since every sweep colony is gone by day 45.

## 2026-09-25

Correction from review:
- The content set these numbers, not the engine. With the clearing-as-build rule reverted and the new content kept, seeds 1 to 20 score 95/100/100/90. With the rule kept and the old content, shelter is 55%.
- The rule stays because it's what a player expects of 'build first', and DESIGN §4d documents it.
- The first target passes by one seed (18 of 20); the held-out seeds 21 to 40 score 95%.
- The sweep's 'felled' now counts a tree the bot marked that's gone, because a windfall drops wood. The quarry column is gone: the bot never quarries, and every stone it saw was a windfall.
- 'Before tuning' in the note above mixed the old bot with the old content: the content alone took shelter from 55% to 90%.

## 2026-09-25

After rebasing onto main (bone, feels-like and the inspector PRs), shelter before night one fell to 80%. Feels-like changes how a cold colonist spends day one, and the target was passing by one seed. Rather than flood the median map with more branches, primitive gains a grass pallet: a bed of 6 fibre at rest 1.3, against core's bed of 25 branches at 1.8. The sweep's first hut uses it. Now seeds 1 to 20: 90/100/100/90; seeds 1 to 40: 92/100/98/85. The first target is still exactly at 90% on seeds 1 to 20: fragile, and noted in DESIGN.
