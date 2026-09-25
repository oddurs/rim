---
id: f802c0b4-e8bb-4758-b357-a6d3f7f8749a
title: rim check fails a mod for being slow on a slow machine
type: bug
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## What happens

`rim check` puts "mod 'weather' is slow: its script calls take 1.41 ms on
average" among its warnings. That warning is wall-clock: it depends on the
machine, not the mod. `shipped_mods_check_clean` requires no warnings, so it
failed on a slow Windows CI runner (#90), and `--strict` would fail a mod the
same way.

## What should happen

Timing is reported, but apart from the warnings a mod can fix by changing
its files; neither the test nor `--strict` fails on it.

## Reproduction

Seed:
Mods: weather
Tick: the first six in-game hours

1. `rim check mods/weather --strict` on a machine slow enough that weather's
   hooks average over 0.5 ms.

## 2026-09-25

check_mod now splits the warnings pushed while the game runs (only Sim::check_mod_budgets, the wall-clock time budget) into CheckReport.slow; rim check prints them as notes and --strict ignores them. Test: a deliberately slow mod checks clean with a note.
