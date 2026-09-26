---
id: 6ec6da26-9db5-42fb-a9be-80534726606d
title: 'primitive: baskets, pots as stores, a woodpile and a stone bin'
type: content
status: backlog
milestone: crafting
depends_on:
- 282efef8-78c1-4d2d-ad5d-4a3dd1246589
created: 2026-09-26
updated: 2026-09-26
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

- [ ] Each store takes what it should and refuses the rest (mod test)
- [ ] `cargo run --release -p rim_sim --example stone_age` targets still pass
