---
id: c24ea9b8-24df-4c76-a29d-e6d106b1c560
title: 'main.rs down to wiring: modules for render, input and settings, and dead code gone'
type: chore
status: doing
milestone: bare-metal
assignee: calm-forest
created: 2026-09-28
updated: 2026-09-29
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

main.rs is 2,659 lines of render passes, input, settings and state. Every render change conflicts in it, and dead paths from the fast build-out hide in it.

## What

Move render(), input and settings into their own modules, and delete unused code and dead helpers across rim_client (cargo's unused warnings plus a search for items with no callers).

## Acceptance criteria

- [ ] main.rs is under 600 lines
- [ ] rim_client is smaller in lines at the end than at the start, with the count in the PR

## 2026-09-28

Baseline on b90b069e: main.rs 2,686 lines, rim_client 19,836. A search for functions and constants with no callers found none outside tests and FFI bindings, so the line savings will come from consolidating duplicated helpers (alpha, shade and disc exist in several files), not from deleting uncalled code. Step 1 is settings into settings.rs; render() follows once lucky-harbor's render clock change (RawInput dt, app.now) has merged, since both touch the level fade.

## 2026-09-29

PAUSED: done: render.rs (#361) and settings.rs (#349) moved out; input.rs, actions.rs and tools.rs split on draft #391 (main.rs 2,203 → 731), plus the dead Action::ToggleDraft/CenterSelected and App.dragged. Left: re-run the split on a fresh main (don't rebase; the scripts and steps are in #391's description), gate, merge. Under 600 comes when green-forest's leave-to-title moves main()'s App build and loop into play.rs (agreed). Next step: re-run #391's scripts on main after play.rs lands. Branch refactor/c24ea9b8-input-module.
