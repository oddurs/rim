---
id: c24ea9b8-24df-4c76-a29d-e6d106b1c560
title: 'main.rs down to wiring: modules for render, input and settings, and dead code gone'
type: chore
status: backlog
milestone: bare-metal
assignee: calm-forest
created: 2026-09-28
updated: 2026-09-28
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
