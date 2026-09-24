---
id: 2c8ab1ef-ef2f-4c2c-a26d-7e366cfd683a
title: Luau types for rim test's API, and type-check tests/
type: feature
status: backlog
milestone: sdk
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: additive
layer: tooling
area: modding
effort: s
---

## Why

Mechanisms 1-5 of the UI plan add three node kinds and nine functions. A
UI mod that names a view function that does not exist should be told at
load, not at first hover.

## What

- Generate `ui.d.luau`, `view.d.luau` and `act.d.luau` from the Rust
  registrations in the build.
- Version the UI surface as the sim API is (`ui_api` in `mod.toml`);
  `rim check` refuses a mod that names a call the version lacks.

## Acceptance criteria

- [ ] The generated types match the registrations, checked in CI
- [ ] A mod naming a missing view call fails `rim check` with its name
