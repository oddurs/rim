---
id: 554
uid: fe1c23b7-da1b-4c51-b14d-a7deea184ebe
title: Bug capture and rim repro
type: feature
status: backlog
milestone: workbench
depends_on:
- 518
- 526
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: m
layer: client
area: save
---

## Problem

A bug report depends on someone finding and sending the right save, and `--load` writes into the file it opened. DESIGN.md §11a. The 1.0 crash bundle (8a8deb4a) needs the same thing.

## Proposal

F8 writes a `.rimbug` bundle: the save, mod lock, commit, seed, the last in-game day of commands, a screenshot and the session log. `rim repro <file>` opens it read-only at its tick. `--load` stops appending to the file it opened.

## Acceptance criteria

- [ ] A bundle made at tick N opens at tick N with the same state hash (test)
- [ ] Loading a save doesn't change the file (test)

## 2026-09-27

Bug bundles should embed the sim sides of their mods, so a report reproduces without the index (ead42976).
