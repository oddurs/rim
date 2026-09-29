---
id: 716
uid: af2c2733-fa35-4065-8b47-d03a6ea9d99f
title: The Work window gets a redesign pass
type: spike
status: backlog
milestone: rimos
depends_on:
- 566
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: m
layer: core
area: ui
api: none
---

## Problem

Playing, the user said the whole Work UI needs a lot of work. Today it's an 816×860 sheet (mods/core/ui/work.luau:848-878). From the top it holds a stance bar, an orders panel and Board/Roles/Person tabs, then the lens. The Board lens is an Auto hint, a Focus line, a brush bar, a header grid, then rows of role, name, a 56×20 priority grid and job. Priorities are painted by click-drag, the wheel or keys.

## Proposal

A design pass before code, under RimOS: what the player needs to see and change at a glance, what can go, and what becomes its own app (0e773a3e). The result is a plain mock-up and the items that come from it. The scrolling fixes are separate items.

## Acceptance criteria

- [ ] A mock-up the user approves, with the items it needs filed
