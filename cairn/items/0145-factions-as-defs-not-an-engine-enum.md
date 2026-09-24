---
id: 978be4f3-0b80-4b7f-a036-126009306617
title: Factions as defs, not an engine enum
type: feature
status: backlog
milestone: plugin-api
depends_on:
- be8174f0-ff41-44fe-b788-2ffff60e0d19
created: 2026-09-23
updated: 2026-09-24
priority: p0
api: breaking
effort: m
layer: engine
area: sim
pillar:
- plugin-first
---

## Why

`Faction` is a hard-coded enum (wild, player, hostile) that `rim.spawn_pawn` parses from strings. A trade, tribe or rival-town mod can't add a faction, which breaks "engine verbs, plugin nouns" (DESIGN.md §6, §10). Doing it now keeps the later factions work (0113) additive instead of breaking.

## Acceptance criteria

- [ ] `[[faction]]` defs with id, label, colour and a default stance toward others
- [ ] core defines `core:player`, `core:wild`, `core:hostile`; engine code refers to none of them by name except through a def flag (e.g. `player = true`)
- [ ] Hostility checks in AI read the stance table
- [ ] Determinism test still passes

## 2026-09-24

Reachability regions are one layer per Faction, indexed by the enum (map.rs). When factions become defs, the layers become one per door key: what differs between them is only which owned doors are open. See DESIGN.md §6a.
