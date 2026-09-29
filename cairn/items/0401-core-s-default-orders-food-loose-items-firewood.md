---
id: 401
uid: bbc59ec3-10da-4c12-8e05-45f20dac8c75
title: 'Core''s default orders: food, loose items, firewood'
type: content
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 385
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: m
layer: core
area: needs
---

## Why

Auto reacts to waiting work; the colony also needs to react to what it is short of. DESIGN.md §4d: core ships three standing orders, on by default, so a new player's colony answers hunger, clutter and winter without being told.

## What

- `mods/core/scripts/readings.luau`, hourly: `core:food_days` (food in stores over what the colonists eat a day, from `count_items` and the food need's rate), `core:loose_items` (items outside a store that takes them), `core:fuel_days` (fuel over what fires burn a day, in autumn only; 99 otherwise).
- Three `[[priority_rule]]`s:
  - Food is low: `food_days` below 5 until 8; Harvest and Hunt −1.
  - Loose items piling up: `loose_items` above 30 until 10; Haul −1.
  - Firewood for winter: `fuel_days` below 10 until 15; Chop −1.
- News on `rule_started` and `rule_stopped`: "Food is low: 3.4 days left. Harvest and Hunt one level sooner until there are 8 days." with "Show on the board" and "Switch this order off".
- A balance pass: the survival harness with and without the orders.

## Acceptance criteria

- [x] Each reading matches a hand count in a fixture colony (test)
- [x] Each order starts and stops at its marks in a scripted scenario (test)
- [x] The survival harness loses no more colonies with the orders on than off, recorded in DESIGN.md §4d

## 2026-09-26

The storage work (DESIGN §4f) is adding a stock ledger: rim.stock(thing | {tag} | {category}, "stored" | "loose"), O(1). Read core:loose_items from rim.stock(..., "loose") instead of scanning items.

## 2026-09-26

Firewood for winter became wood before winter: core's fires burn nothing yet, so there's no fuel to count. It holds in autumn under 60 wood, until 120. Loose items count every item outside a store that takes it, so with no stockpile everything is loose; the Haul shift then does nothing, since there's no haul work without a stockpile. A switched-off order now keeps tracking its reading but posts no news and bumps no evaluation.
