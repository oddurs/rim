---
id: 537ef322-6f7c-4e05-aa5b-1540f727de2f
title: 'Structured logging: tracing with per-mod targets, a session log file and a log panel'
type: feature
status: backlog
milestone: workbench
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Problem

Diagnostics go to `eprintln!`, the message feed and F3's warnings, with no levels, no file and no filter; `rim check` prints only the first line of a script error. DESIGN.md §11a.

## Proposal

`tracing` with a target per subsystem and per mod, a log file per session under the data directory, `RIM_LOG` to set levels, a dev log panel with filters, and whole errors in `rim check`.

## Acceptance criteria

- [ ] A script error appears with its traceback in the file, the panel and `rim check` (shown)
- [ ] `RIM_LOG=mod:weather=debug` shows only that mod's debug lines

## 2026-09-27

The log itself (tracing, the file, RIM_LOG) is engine and client work in this item; the log panel is part of mods/devtools (DESIGN.md §11a), reading the log through the dev API.
