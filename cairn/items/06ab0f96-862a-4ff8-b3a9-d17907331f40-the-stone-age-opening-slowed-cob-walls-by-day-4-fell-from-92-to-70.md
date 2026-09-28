---
id: 06ab0f96-862a-4ff8-b3a9-d17907331f40
title: 'The stone-age opening slowed: cob walls by day 4 fell from 92% to 70%'
type: bug
status: backlog
milestone: stone-age
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: m
layer: plugin
area: ai
---

## Why

The stone-age sweep (`cargo run --release -p rim_sim --example stone_age -- --seeds 40 --days 10`) measured cob walls by the end of day 4 on 92% of seeds before cooking, research, spoilage, pits and the other merges of 2026-09-27, and 70% after them, on main c5d2161e. The later steps slip with it: a brick house by day 10 fell from 78% to 60% of seeds. A lone colonist is spending time somewhere new before the cob step, and wolves and raiders kill a quarter of them by day 10.

## What

- Find where the time goes: run the sweep with `--show SEED` on seeds that miss cob by day 4, and compare against a main from before the merges.
- Fix what's wrong, or rebalance the bot or the opening. Record which in a note.

## Acceptance criteria

- [ ] Cob walls by day 4 back above 85% on 40 seeds, or a note explaining why the new number is right
