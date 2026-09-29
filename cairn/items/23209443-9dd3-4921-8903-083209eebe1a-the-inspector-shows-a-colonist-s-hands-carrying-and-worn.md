---
id: 23209443-9dd3-4921-8903-083209eebe1a
title: The inspector shows a colonist's Hands, Carrying and Worn
type: feature
status: backlog
milestone: carrying
depends_on:
- 3c8833c4-214c-4da2-ae5e-b02769b71039
created: 2026-09-29
updated: 2026-09-29
priority: p2
effort: s
layer: core
area: ui
api: additive
---

## Problem

The UI's pawn table has only a `hand` label and a `carrying` string (crates/rim_ui/src/vm.rs:2541-2547). Worn garments aren't exposed (5e92cd70).

## Proposal

Three rows in the colonist inspector: Hands, Carrying and Worn. A chosen thing shows a pin, and right-clicking a row gives its drop or take-off order. Doc: https://claude.ai/code/artifact/349db585-8e04-44a3-a729-f2163bd8d5f4.

## Acceptance criteria

- [ ] An autotest shot of a colonist holding a hammerstone, carrying flint and wearing a hide wrap
