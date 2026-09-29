---
id: 441
uid: fff4fb42-0b68-454c-b5ff-204609564b6a
title: A material names the tool it is built with
type: feature
status: done
milestone: houses
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
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

- [x] An ashlar wall waits until a maul exists, then gets built (test)
- [x] The stone_age sweep is unchanged: day-one huts are wattle

## 2026-09-26

Built on #165's build.requires rather than waiting for builds to become orders (e7c4a3f6): a blueprint's needed tools are the def's build.requires_r joined with its material's stuff.requires_r.

## 2026-09-26

Primitive's pounding tool is the hammerstone, so the test waits for that. stone_age --seeds 8 --days 4 prints byte-identical output with and without the patch: the bot builds with branches and clay.

## 2026-09-26

Dropped the chopping rule for logs from primitive: wood only comes from felling, which already takes an axe, so the rule added nothing, and it stranded windfall wood and broke six tests that build wood walls with primitive on. Stone keeps its pounding rule.
