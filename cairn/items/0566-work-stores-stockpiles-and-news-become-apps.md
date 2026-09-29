---
id: 566
uid: 0e773a3e-d7e7-4814-8de0-5edce457b088
title: Work, Stores, Stockpiles and News become apps
type: feature
status: backlog
milestone: rimos
depends_on:
- 586
- 644
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: core
area: ui
---

## Problem

The big screens are sheets: fixed in the centre, one at a time, never beside each other.

## Proposal

They become apps that open maximised in the free area the first time, then remember their place. `screens.add` keeps working for mods and makes an app.

## Acceptance criteria

- [ ] Work and Stores can be open side by side, snapped left and right (autotest)
- [ ] Their keys still toggle them; research's screen works unchanged
