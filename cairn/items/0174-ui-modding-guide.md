---
id: ac5a569b-1ab9-4f92-8671-b6b9a8e5e3f6
title: UI modding guide
type: docs
status: done
milestone: interface
depends_on:
- c9b1406d-fa9a-4d48-88c4-5b66d8d2076c
- 959bba6d-ac66-47e9-9dad-09f005158cd3
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
