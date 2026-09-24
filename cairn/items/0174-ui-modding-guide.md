---
id: 174
title: UI modding guide
type: docs
status: done
milestone: interface
depends_on:
- 82
- 173
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p2
api: none
effort: s
layer: tooling
area: docs
---

## Why

The UI is only moddable if modders can learn it in an afternoon.

## What

A chapter in the modding guide (0086): components, `view` and `act`, tokens and themes, the four mod operations, devtools, hot reload, with the wildlife_plus example walked through.

## Acceptance criteria

- [x] Guide chapter covering components, view/act, tokens, operations, devtools and hot reload
- [x] Every code sample in the guide is taken from a mod that loads in CI

## 2026-09-23

docs/modding/ui.md. Test guide_samples_run loads every Luau block in the guide as a mod beside core and fails on any error (checked: a token typo in a sample fails it).
