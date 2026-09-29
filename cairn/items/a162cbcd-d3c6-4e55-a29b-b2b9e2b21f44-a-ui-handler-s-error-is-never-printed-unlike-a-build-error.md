---
id: a162cbcd-d3c6-4e55-a29b-b2b9e2b21f44
title: A UI handler's error is never printed, unlike a build error
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: ui
---

## What

`UiVm::call_with`, `call_handler` and `call_when` (crates/rim_ui/src/vm.rs)
push a failed handler's error onto `vm.errors` as `handler: <first line>`
and do nothing else. A build error, by contrast, is printed
(`eprintln!("rim_ui: {e}")` in `run_build`), named by its mod, and shown in
an error box in place.

## How it fails

An `on_click`, `on_change`, `on_key`, `on_drag`, a grid's paint or wheel, a
binding's run or `when`, or the context handler that throws: the click does
nothing, and the only trace is a line in the F3 profiler's warning list.
Nothing reaches the terminal a modder runs the game from.

## Fix

One place records a handler's error: kept once and printed once, as build
errors are.
