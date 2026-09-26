---
id: ff479390-38f9-45e1-9d6c-04e98fcd471d
title: 'The building ladder: dry stone, logs, bricks and a kiln'
type: content
status: backlog
milestone: houses
depends_on:
- 7c53ec62-85bb-4752-8bed-9b1271d0eef3
- fff4fb42-0b68-454c-b5ff-204609564b6a
created: 2026-09-26
updated: 2026-09-26
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

- [ ] Every structural material has a pattern, a roof and a span
- [ ] The stone_age sweep holds its targets
- [ ] A brick house is reachable by day 10 on 80% of seeds (new sweep step)
