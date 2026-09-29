---
id: 386
uid: 98b8ec4a-eff8-4a52-afbe-d64c61820856
title: The orders panel, and Focus on the HUD
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 385
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: s
layer: core
area: ui
---

## Why

A standing order that acts silently feels like a bug, and a stance left on in spring is a trap. DESIGN.md §4d: orders are listed with their live readings and switch off in one click; a non-Normal Focus stays lit on the HUD.

## What

- An "Orders · 1 on" button beside the stance bar opens a panel: each reading rule with its label, what it does, its live reading ("on · 3.4 days" or "waiting · 7.0 days") and a switch sending `SetRuleEnabled`.
- The HUD shows the stance chip when it isn't the first stance, and the count of orders on.
- The board's focus line: which stance and orders hold and how many cells they move.

## Acceptance criteria

- [x] Switching an order off in the panel stops it moving any cell (UI test)
- [x] The HUD chip appears for Siege and not for Normal (autotest)
