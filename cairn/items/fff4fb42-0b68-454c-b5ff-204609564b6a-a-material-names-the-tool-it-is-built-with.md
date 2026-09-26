---
id: fff4fb42-0b68-454c-b5ff-204609564b6a
title: A material names the tool it is built with
type: feature
status: backlog
milestone: houses
depends_on:
- e7c4a3f6-99b4-4587-b53e-b40cb0ea96a0
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: s
layer: engine
area: building
---

## Why

Gathering climbs a tool ladder (§4e), but building doesn't: a naked founder lays ashlar bare-handed. Minecraft ties a pickaxe tier to each block material. DESIGN.md §6c ties a tool to each building material.

## What

- `stuff.requires = ["pounding"]`: anything built of this material needs a worker holding a tool with those tags. This goes through orders' `requires` once builds are orders.
- Primitive sets logs to `chopping` and ashlar to `pounding`. Wattle, cob, dry stone and brick build by hand.
- A plan that no tool in the colony can build says why, like a bill.

## Acceptance criteria

- [ ] An ashlar wall waits until a maul exists, then gets built (test)
- [ ] The stone_age sweep is unchanged: day-one huts are wattle
