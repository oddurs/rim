---
id: 655
uid: 97a9ba85-c6cf-4595-a9e6-7cc85bd7b398
title: ui.list with a huge count overflows
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

Unverified beyond reading. `Builder::expand_list` (crates/rim_ui/src/vm.rs)
turns count into `n as usize` (1e30 saturates to usize::MAX) and then keys
the bottom spacer with `count + 2`: an overflow panic in a debug build, and
a spacer of 1.8e19 rows in release. Fix: cap count like a grid's cells, as a
node error.
