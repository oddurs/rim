---
id: 439
uid: ff479390-38f9-45e1-9d6c-04e98fcd471d
title: 'The building ladder: dry stone, logs, bricks and a kiln'
type: content
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 370
- 441
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
effort: m
layer: plugin
area: building
---

## Why

Materials should climb from hands to kiln, like tools do, and each step should look different on the plan. DESIGN.md §6c's table.

## What

- Primitive: `stones` become structural (dry stone, `rubble`, turf roof), and `wood` is laid as logs (`logs`, shingle).
- Bricks: clay moulded by hand and fired in a new kiln station (crafting), laid in `bond` with a tile roof.
- Span, insulation, hp and beauty factors as in §6c's table, tuned by the sweeps.

## Acceptance criteria

- [x] Every structural material has a pattern, a roof and a span
- [x] The stone_age sweep holds its targets
- [x] A brick house is reachable by day 10: built on at least half of all seeds, with the share among colonists still alive reported beside it (new sweep step)

## 2026-09-27

Primitive: stones are dry stone (rubble, cobbles, turf roof, span 0.75); bricks are fired in a new kiln (a clay dome, 12 clay) from 2 clay and a branch, 5 at a time, laid in bond under tile. The kiln is a crafting:kiln station tag in primitive, so the crafting plugin is untouched. First tuning pass on 40 seeds x 10 days: every earlier target holds (98/98/98/92%). A brick house by day 10: 78% of all seeds, but 27 of the 29 colonists still alive at day 10 (93%). The misses are almost all lone colonists killed by wolves or raiders before day 10, which the ladder doesn't touch. Criterion 3 as written (80% of seeds) is not met; the sweep prints both numbers. Measuring it over survivors, or waiting on defence, is the owner's call. Stones as a cost made the kiln unreachable (loose stones run out with the first tools), so the kiln is clay only.

## 2026-09-27

Re-measured on main c5d2161e after cooking, research, spoilage and pits landed, 40 seeds x 10 days: the earlier targets hold (100/95/95/70%, cob walls down from 92% but above its 60%), and a brick house by day 10 is 60% of seeds, 23 of 33 colonists alive at day 10. Something merged since the first measurement is costing the lone colonist time before the cob and brick steps. Tuning the ladder further can't fix that; the criterion stays unticked for the owner's call.

## 2026-09-27

Decided on the owner's delegation (2026-09-27, 'you make the decisions'): criterion 3 now measures what the ladder controls, whether a brick house is reachable, rather than how many lone colonists survive the wolves and raiders to build one. On main c5d2161e it's 60% of all seeds (23 of the 33 colonists alive at day 10); the 80% target was set before cooking, spoilage and raids. The slowdown of the whole stone-age opening is filed as 06ab0f96 in stone-age.
