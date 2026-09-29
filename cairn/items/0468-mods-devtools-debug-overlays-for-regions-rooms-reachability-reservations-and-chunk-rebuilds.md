---
id: 468
uid: 3fb6c173-5c15-46cc-8ef3-b7fd5915620c
title: 'mods/devtools: debug overlays for regions, rooms, reachability, reservations and chunk rebuilds'
type: feature
status: backlog
milestone: workbench
depends_on:
- 490
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: plugin
area: render
---

## Problem

The O overlays show four fields and storage; the sim's own structures can't be seen. DESIGN.md §11a.

## Proposal

With `--dev`, the O cycle adds regions, rooms, reachability from the selection, reservations, and chunk rebuilds (a cell flashes when its chunk rebuilds); field overlays gain a legend and range.

## Acceptance criteria

- [ ] Each overlay has an autotest shot, looked at

## 2026-09-27

Ruled 2026-09-27 (DESIGN.md §11a): the workbench's tools are a first-party plugin, mods/devtools, loaded only with --dev. Build this item's UI there, on the dev API (6ae1513a-d8d0-4744-90e5-386d6d1d1921); anything the engine must add goes in that item.
