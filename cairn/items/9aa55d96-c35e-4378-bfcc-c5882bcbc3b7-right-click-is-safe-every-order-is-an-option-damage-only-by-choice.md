---
id: 9aa55d96-c35e-4378-bfcc-c5882bcbc3b7
title: 'Right-click is safe: every order is an option, damage only by choice'
type: bug
status: review
milestone: pointer
assignee: Oddur Sigurdsson
claimed: 2026-09-26
depends_on:
- 3b1726ff-ded5-45f6-968f-cb0a5e6a2577
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: ui
---

## Why

With a colonist selected, right-clicking anything the colony built resolves to Deconstruct and runs at once (`order.rs`, `fixture`). Right-click is also "go here", so a click a cell off takes down a wall, a bed or the campfire.

## What

- `order::options(w, pawn, cell, on)` returns every order for a spot, each flagged `damaging` and with a short reason when it can't run. `resolve` returns the first safe one; damaging orders are only ever options. Read-only in the sim.
- The client decides a right-click on release: a tool in hand drops; moving 6 px is a drag; a 350 ms hold opens the orders menu; otherwise the first safe order runs, and with nothing safe the menu opens instead.
- The map's orders are the first provider of the context menu (`view.orders()`, `act.order(i)`), for one colonist or a group.
- The cursor hint shows the default before the click, and "hold for more" when there are other orders.

## Acceptance criteria

- [x] Right-clicking your own wall with a colonist selected orders nothing and opens the menu
- [x] The menu's Deconstruct issues the deconstruct order
- [x] Open ground still means go here; a tool in hand still drops
- [x] Holding right-click opens every order for the spot

## 2026-09-26

order::options lists every order with a stable key and a damaging flag; resolve takes the first safe; Command::Order gained pick (serde default, old logs load). The floor case was the likeliest accident: right-clicking your own floor to walk there deconstructed it. Felling an unmarked tree, which a plain click used to do, is now damaging too; a tree marked for chopping still chops on a plain click.
