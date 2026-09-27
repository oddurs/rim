---
id: 4f3e5d8d-a566-4520-802f-f03fd6d81ab5
title: 'mods/timber: planks, plank walls, crates, shelves, racks and a granary'
type: content
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 282efef8-78c1-4d2d-ad5d-4a3dd1246589
- 6ec6da26-9db5-42fb-a9be-80534726606d
- dce75339-9120-495d-9b3b-b4012251adac
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: m
layer: plugin
area: building
---

## Why

The middle of the ladder (DESIGN.md §4f): planks are the material that makes containers and better walls.

## What

- A new first-party plugin, depending on core, crafting and primitive.
- Planks: hewn from wood with a `chopping` tool at a crafting spot (slow), or sawn at a saw pit with a `sawing` tool (fast). A structural material with `span` (houses, §6c).
- Plank wall, crate (4 slots), shelf (2×1, 6 slots, shows its contents), tool rack (tools only) and granary (2×2, food).
- Relabels `core:wood` as "logs" by patch; the id stays.
- Plays without iron: nothing needs nails until iron patches them in.

## Acceptance criteria

- [x] A colony with timber and no iron can make planks, a plank wall and a crate (mod test)
- [x] The shelf draws what it holds
- [x] rim check --strict passes with and without iron installed

## 2026-09-26

Planks are hewn from logs with a chopping tool at a crafting spot (one to a log, 600 work); the saw pit and sawn planks (two to a log, 300 work, needs sawing) are iron's, because a saw pit in timber alone would be a station no one could use. Planks are structural and timber stuff (span 1.0 = logs' roof reach); the plank wall is 3 planks of the timber category with core wall's look, boundary and support (span 4). Crate 4 slots not bulky, shelf 2x1 6 slots display items, tool rack core:tools, granary 2x2 8 slots food x2 plus 10 clay. core:wood reads 'logs' by patch. Tests: timber alone hews planks and builds a plank wall and a crate; the client's contents() draws a shelf's three held things in slot order (draw.rs test).
