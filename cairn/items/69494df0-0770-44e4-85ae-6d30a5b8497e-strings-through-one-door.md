---
id: 69494df0-0770-44e4-85ae-6d30a5b8497e
title: Strings through one door
type: chore
status: backlog
milestone: colony
created: 2026-09-24
updated: 2026-09-24
priority: p2
api: additive
effort: s
layer: core
area: ui
---


## Why

Every label is a literal in Luau. Localisation is not on the roadmap and
is not built here; routing text through one function costs an hour now
and a week in a year.

## What

- `t("core:toolbar.cancel")` returns the key's default for now; core's
  UI scripts use it for every user-visible string.
- The engine exposes `ui.t`; a language file can back it later.

## Acceptance criteria

- [ ] Core's UI has no user-visible string literal outside `t(...)`
- [ ] A test lists every key core uses
