---
id: 47
title: 'Client: selection, draft, move and attack orders'
type: feature
status: done
milestone: castaway
depends_on:
- 29
- 45
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: none
effort: m
layer: client
area: ui
---

## Why

Direct control for fights.

## Acceptance criteria

- [x] Click to select, R to draft, right-click to move or attack

## 2026-09-23

Verified by rim --autotest (crates/rim_client/src/autotest.rs), which drives the real client through the same Actions keyboard and mouse produce, checks state and saves screenshots; 56/56 checks pass, screenshots reviewed by eye. Runs in CI on macOS. Click selects, R drafts/undrafts, right-click moves (walks there) and right-click on an enemy attacks (the hit lands); right-click with a tool drops the tool; Escape clears.
