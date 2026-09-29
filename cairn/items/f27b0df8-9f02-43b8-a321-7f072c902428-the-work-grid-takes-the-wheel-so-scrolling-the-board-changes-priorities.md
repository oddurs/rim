---
id: f27b0df8-9f02-43b8-a321-7f072c902428
title: The Work grid takes the wheel, so scrolling the board changes priorities
type: bug
status: backlog
milestone: rimos
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: s
layer: core
area: ui
api: none
---

## Problem

`on_wheel` on the priority grid takes the wheel over any cell (crates/rim_ui/src/lib.rs:895-913, mods/core/ui/work.luau:497). Scrolling the board with a trackpad nudges priorities instead of scrolling.

## Proposal

The wheel scrolls. Changing a priority with the wheel needs a modifier held, or goes.

## Acceptance criteria

- [ ] A test: a wheel event over a priority cell scrolls the board and changes no priority
