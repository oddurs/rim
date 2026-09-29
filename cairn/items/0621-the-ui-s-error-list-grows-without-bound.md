---
id: 621
uid: 58832b28-b481-44ce-b587-927d600fc840
title: The UI's error list grows without bound
type: perf
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

`UiVm::errors` (crates/rim_ui/src/vm.rs) keeps every distinct error, checked
with `Vec::contains`. A handler whose message varies (a position, a count)
adds a line every frame it fails: memory grows, the check is O(n) per
error, and the F3 warning list becomes unreadable. Fix: cap it (keep the
first N and count the rest).
