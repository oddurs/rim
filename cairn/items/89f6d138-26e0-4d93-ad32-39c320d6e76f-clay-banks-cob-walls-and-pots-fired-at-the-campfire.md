---
id: 89f6d138-26e0-4d93-ad32-39c320d6e76f
title: 'Clay: banks, cob walls, and pots fired at the campfire'
type: content
status: backlog
milestone: stone-age
depends_on:
- 4675019b-c017-47e7-b148-b12e4be58bda
created: 2026-09-25
updated: 2026-09-25
priority: p1
api: none
effort: m
layer: plugin
area: building
---

## Why

Split from e3846c47. After night one, clay is the upgrade. Cob is warmer than branches and doesn't burn, and a fired pot is the vessel cooking (2c03427e) will ask for.

## What

- **Clay banks** on marsh are dug with a `digging` tool: 4 clay, not destroyed, and they regrow in 4 days.
- **Clay** is structural stuff (cob): hp 0.8, work 1.2, insulation 1.6, flammability 0.
- **`core:campfire`** gains the station tag `crafting:fire` by patch.
- **A clay pot** is fired there from 3 clay, with 300 work.

## Acceptance criteria

- [ ] Clay is dug with a digging stick, and not with bare hands
- [ ] Cob walls build from clay and hold heat better than branch walls
- [ ] A pot is fired at a campfire from a bill
