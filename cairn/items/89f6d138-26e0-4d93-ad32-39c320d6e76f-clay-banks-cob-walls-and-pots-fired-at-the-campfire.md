---
id: 89f6d138-26e0-4d93-ad32-39c320d6e76f
title: 'Clay: banks, cob walls, and pots fired at the campfire'
type: content
status: done
milestone: stone-age
assignee: Oddur Sigurdsson
depends_on:
- 4675019b-c017-47e7-b148-b12e4be58bda
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
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

- [x] Clay is dug with a digging stick, and not with bare hands
- [x] Cob walls build from clay and hold heat better than branch walls
- [x] A pot is fired at a campfire from a bill

## 2026-09-25

Clay banks use core's gather designation with requires = ["digging"], so a bank says 'Needs a digging tool.' and the toolbar needs no new mark. The pot takes its material from the clay: a clay pot, at clay's hp. Cob is a wall made of clay (structural stuff), so there's no separate cob def. The warmth test checks the insulation stat the boundary divides by, not a room's temperature.

## 2026-09-25

With af17cc1d (#116), a building planned over a clay bank clears it at once: the bank's only harvest leaves it standing, and it doesn't block. Banks never come back. I accepted that on purpose: there are 72 to 335 reachable banks per map (19 on seed 8), and a wall over one is a player's choice. The crosscheck also moved to seed 9, since clay banks reshuffle every map and seed 1's founder died to wolves on day 3.
