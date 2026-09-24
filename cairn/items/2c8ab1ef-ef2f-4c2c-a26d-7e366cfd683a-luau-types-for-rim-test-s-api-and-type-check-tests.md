---
id: 2c8ab1ef-ef2f-4c2c-a26d-7e366cfd683a
title: Luau types for rim test's API, and type-check tests/
type: feature
status: backlog
milestone: sdk
created: 2026-09-24
updated: 2026-09-24
priority: p3
api: additive
layer: tooling
area: modding
effort: s
---

## Why

Test files use `test`, `t.world`, `t.expect` and world methods that luau-lsp doesn't know, so editors can't complete them and `scripts/check-luau.sh` skips `mods/*/tests`.

## Acceptance criteria

- [ ] The test API is declared once in `modtest.rs`, and `types/test.d.luau` is generated from it with a drift test, like `rim` and `ui`
- [ ] `scripts/check-luau.sh` checks `mods/*/tests` and the shipped tests pass
