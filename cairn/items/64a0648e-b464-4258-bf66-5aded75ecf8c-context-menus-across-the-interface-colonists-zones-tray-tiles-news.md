---
id: 64a0648e-b464-4258-bf66-5aded75ecf8c
title: 'Context menus across the interface: colonists, zones, tray tiles, news'
type: feature
status: backlog
milestone: pointer
depends_on:
- 3b1726ff-ded5-45f6-968f-cb0a5e6a2577
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: core
area: ui
---

## Why

Once the map has a context menu, every other thing you can point at should answer a right-click the same way, so the interface has one grammar.

## What

- Colonist (people column, name labels, inspector): select, centre, draft, work priorities…
- Zone (Zones tray, the map inside a zone): allow all, forbid all, edit…, delete (damaging).
- Tray tile: place, find similar.
- News line and alert: go to, copy, dismiss.
- Each is a provider plus a `menu` subject on its node; no surface draws its own menu.

## Acceptance criteria

- [ ] Right-clicking a colonist row, a stockpile row, a tray tile and a news line each opens the shared menu with core's rows
- [ ] Damaging rows are last and never primary
