---
id: cf957b98-8425-4e44-a80b-52d625ce9f16
title: The crosscheck's seed 5 loses its colony after mining's worldgen
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

From mining (#298, 49fe0d72) on, `crosscheck` ends with "day 5: the colony is gone" on its default seed 5. That fails the ARM crosscheck job and the agree job that depends on it.

## What should happen

The crosscheck runs a seed whose colony lives all 60 days, as the example's comment asks, provided mining didn't make the opening harsher for everyone.

## Acceptance criteria

- [x] Mining didn't make the opening harsher (seed survey and the stone-age sweep, before and after)
- [x] The crosscheck runs 60 days on its default seed

## 2026-09-28

Seeds 1 to 12, 10 days each, before (5a2f5477) and after (b0528faa):
- before, 4, 5, 6, 9, 10, 11 and 12 lived; after, the same set without 5;
- 1, 2 and 8 fail the same way both times (no flint for a hand axe), and 3 and 7 stall on building both times;
- the stone-age sweep is identical: 100%, 100%, 100%, 85%.
So mining changed seed 5's world, not the opening. After mining, seeds 4, 6, 9, 10, 11 and 12 all run 60 days. The default is now 4, the busiest (68 pawns, 167 messages).
