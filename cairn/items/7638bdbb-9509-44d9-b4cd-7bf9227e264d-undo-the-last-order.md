---
id: 7638bdbb-9509-44d9-b4cd-7bf9227e264d
title: Undo the last order
type: feature
status: review
milestone: pointer
assignee: Oddur Sigurdsson
claimed: 2026-09-26
depends_on:
- 9aa55d96-c35e-4378-bfcc-c5882bcbc3b7
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: additive
effort: m
layer: engine
area: ui
---

## Why

A right-click order has no way back once it's given; a wrong one costs work, or a wall.

## What

- Each player order leaves a toast for five seconds ("Gunnar will deconstruct the wall · Undo"); Cmd/Ctrl+Z undoes the newest.
- Undo is a command: it puts the pawn back on its previous job and releases the order's reservations, so a replay stays exact.

## Acceptance criteria

- [x] An order's toast undoes it within five seconds
- [x] Cmd/Ctrl+Z undoes the newest order
- [x] Undo goes through a Command and the determinism test passes

## 2026-09-26

Undo stops the ordered job if the pawn is still on it (by the job's target, or the walk's cell) and takes back a deconstruct mark only if the order put it there; it doesn't try to restore the previous job, whose target may be gone. The toast forces a UI rebuild through the client hash, so it shows the frame the order is given.
