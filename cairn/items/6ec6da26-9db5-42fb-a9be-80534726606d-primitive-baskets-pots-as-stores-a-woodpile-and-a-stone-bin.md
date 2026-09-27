---
id: 6ec6da26-9db5-42fb-a9be-80534726606d
title: 'primitive: baskets, pots as stores, a woodpile and a stone bin'
type: content
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 282efef8-78c1-4d2d-ad5d-4a3dd1246589
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: none
effort: s
layer: plugin
area: building
---

## Why

The first rungs of the storage ladder (DESIGN.md §4f). Pots exist and do nothing.

## What

- Basket: 6 fibre or 2 cordage at a crafting spot; walkable; 2 slots; small things only.
- Pot: a placed pot holds one stack of food.
- Woodpile: 4 branches, 2×1, 2 slots at 3×, wood, branches and fuel only.
- Stone bin: 12 stone, 2 slots at 2×, stone and ore only.
- Numbers tuned with the stone-age sweep; the day-one targets in §4e must still hold.

## Acceptance criteria

- [x] Each store takes what it should and refuses the rest (mod test)
- [x] `cargo run --release -p rim_sim --example stone_age` targets still pass

## 2026-09-26

Basket (6 fibre; the 'or 2 cordage' in the plan can't be said, since a build has one cost), storage pot (built from one fired pot: an item can't be a store, so the placed pot is a building), woodpile (2x1, wood and fuel three deep), stone bin (12 stone, mineral things two deep). Added the 'mineral' tag (core stone, primitive stones; iron's ore takes it) so the bin names no other mod's items. stone_age sweep after: campfire and enclosed bed 95% (target 90), flint tool by day 2 100%, felled tree by day 3 100%, cob walls by day 4 90%. New defs before wild.toml shift the spawn hash (keyed by def index), which changed seed 2's map; shelter's an_enclosed_room_has_no_wind searched for open ground only along four axes and found none, so it now searches whole rings.

## 2026-09-26

Correction to the note above: the crosscheck scenario also failed with the storage defs in storage.toml (seed 1 lost reachable flint, so no hand axe by day 5). The file is now yard.toml, sorting after wild.toml, which leaves every wild thing's def index and so every map as it was; the shelter test change is reverted. The underlying fragility is filed as c2b1b623 (spawns seeded by def index).

## 2026-09-26

stone_age on the final branch (yard.toml, current main): campfire and enclosed bed 100% (target 90), flint tool by day 2 100% (80), felled tree by day 3 100% (80), cob walls by day 4 70% (60). The earlier 90% for cob walls was on an older main.
