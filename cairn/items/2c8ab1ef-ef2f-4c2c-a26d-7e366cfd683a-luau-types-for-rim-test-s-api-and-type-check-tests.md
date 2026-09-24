---
id: 2c8ab1ef-ef2f-4c2c-a26d-7e366cfd683a
title: Luau types for rim test's API, and type-check tests/
type: feature
status: done
milestone: sdk
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] The generated types match the registrations, checked in CI
- [x] A mod naming a missing view call fails `rim check` with its name

## 2026-09-24

Criterion 1 was already true: crates/rim_ui/src/api.rs is the one declaration, tests/api_types.rs checks it against the VM's registrations and against the checked-in types/ui.d.luau and docs/modding/api-ui.md, and CI runs it with the workspace tests. This item adds the version: UI_API_VERSION (0.1) in the engine, ui_api in mod.toml (the shipped mods declare it), and rim check refusing a mod that targets a version the engine lacks. Criterion 2 is a scanner, not a type checker: check.rs walks each ui/*.luau skipping comments and strings (long brackets and interpolated strings included) and reports every ui./act./view. member not in UI_API, with file, line and the nearest real name. It cannot see a member reached through a local alias, and it does not check argument types; luau-lsp against types/ui.d.luau (scripts/check-luau.sh) does that when installed.
