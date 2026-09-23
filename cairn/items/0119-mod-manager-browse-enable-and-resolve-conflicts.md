---
id: 119
title: 'Mod manager: browse, enable and resolve conflicts'
type: feature
status: backlog
milestone: platform
depends_on:
- 152
- 154
created: 2026-09-22
updated: 2026-09-23
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
