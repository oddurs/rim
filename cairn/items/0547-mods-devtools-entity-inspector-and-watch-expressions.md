---
id: 547
uid: ea920745-7885-4c42-9491-51b852f3b872
title: 'mods/devtools: entity inspector and watch expressions'
type: feature
status: backlog
milestone: workbench
depends_on:
- 488
- 490
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: plugin
area: ui
---

## Problem

The player inspector shows curated fields. A developer can't see an entity's components, job step, reservations or path, or stop the game when something becomes true. DESIGN.md §11a.

## Proposal

A dev inspector for any entity: raw components (serialized), the job and its step, reservations, the path drawn on the map, the commands and events that touched it this session, and watch expressions that pause the game when true. Several can be pinned.

## Acceptance criteria

- [ ] Autotest: select a hauler, see its job step and path; a watch on a need pauses when it crosses the value

## 2026-09-27

Ruled 2026-09-27 (DESIGN.md §11a): the workbench's tools are a first-party plugin, mods/devtools, loaded only with --dev. Build this item's UI there, on the dev API (6ae1513a-d8d0-4744-90e5-386d6d1d1921); anything the engine must add goes in that item.
