---
id: 109
uid: 2c03427e-d62c-4c83-9f19-3f9b4e03ef64
title: 'Cooking: raw food into meals'
type: content
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
effort: s
layer: core
area: needs
---

## Why

Better food, better colony.

## Acceptance criteria

- [x] Campfire as first bench

## 2026-09-25

Cooking becomes recipes in the crafting plugin (049e2f73) at a station tagged crafting:fire. The stone age puts that tag on core:campfire, and its clay pot (e3846c47) is the vessel a stew asks for. The stove stays a later station.

## 2026-09-27

What's built:
- Crafting owns the fire station now. Primitive's patch putting crafting:fire on core:campfire moved into mods/crafting/defs/cooking.toml, so cooking works with core and crafting alone.
- A Cook work type (crafting:cook): crafting skill, priority 3, order 44, auto per_person 2 and weight 3, weighed like food work as calm-harbor asked.
- Roast meat: two raw meat make one, nutrition 0.3 against 0.16 raw, 180 work.
- Primitive's stew: 1 raw meat and 2 berries make 2 stew at 0.3 each, against 0.32 raw, 300 work.

The vessel: stew requires = ["boiling"], a new core-vocabulary tool tag, and primitive's pot became a tool (tags boiling, wear 1, so a pot lasts 40 stews). A vessel held rather than consumed fits the tool mechanism: no engine change, no pot eaten per bowl, and the stall reason says 'needs a boiling tool'.

The stove stays out, as noted on 2026-09-25. Tagging it crafting:fire later is one patch.

Tests (tests/cooking.rs): a colonist in Hand roasts meat at a campfire under core and crafting alone; a stew waits with no pot, boils once one is put down, and the pot survives; and the meals feed more than their raw inputs.
