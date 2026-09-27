---
id: 7f26e2e3-89ee-4a50-b80c-5fea90522e58
title: 'Mod manager: browse, enable and resolve conflicts'
type: feature
status: backlog
milestone: platform
depends_on:
- 73751f4f-cd52-467a-9098-55d8033e4b4b
- 76a0bc45-13c1-4725-b5ce-47957109ac41
- 9e979a26-5dc0-4122-b62d-fc48f14d8488
created: 2026-09-22
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: client
area: ui
pillar:
- plugin-first
---

## Why

Players manage mods in-game. There is no manual load order (DESIGN.md §10): order comes from manifests, and conflicts are resolved per field.

## Acceptance criteria

- [ ] Browse the index, install, update and remove (uses the rim add code)
- [ ] Shows the derived load order read-only, with why each mod is where it is
- [ ] Shows patch conflicts; the player picks a winner per field, saved in the lockfile
- [ ] Import and export a modlist lockfile as a modpack

## 2026-09-27

Builds on 73751f4f (the default game is a set), which makes the New colony screen and its Change list of installed mods. This item adds the index, the read-only load order with reasons, and per-field conflict picks to that list rather than building a second mod screen. Sim options (fe54d733) are edited on the New colony screen; after a colony starts, the mod manager shows them read-only and edits only UI options.

## 2026-09-27

Decided 2026-09-27 (modding review, PR #246): the mod manager is where contested slots (dbb92ebe) are picked. Sim picks go to the lockfile, client picks (UI ids, theme tokens) to the player's settings. It lists client-side mods (e4b96647) apart as the player's own, edits player options, and shows colony options (fe54d733) read-only once a colony has started.
