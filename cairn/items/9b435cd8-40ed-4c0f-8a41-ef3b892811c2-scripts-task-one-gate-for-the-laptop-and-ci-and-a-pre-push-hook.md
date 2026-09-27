---
id: 9b435cd8-40ed-4c0f-8a41-ef3b892811c2
title: 'scripts/task: one gate for the laptop and CI, and a pre-push hook'
type: chore
status: doing
milestone: proving-ground
assignee: Oddur Sigurdsson
claimed: 2026-09-27
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: s
layer: tooling
area: tests
---

## Problem

What "green" means isn't written anywhere that runs. CLAUDE.md says `cargo test -p rim_sim`; README lists four commands; CI runs about ten: `cargo fmt --check`, the Luau type check, `cairn check --render --strict`, clippy with `-D warnings` over every target, nextest over the workspace in release, `check-no-fma.sh`, `rim test`, `rim check --strict`, the crosscheck, the sim runs, and the client autotest and render bench. So a PR finds out in CI, and each fix costs a full run. DESIGN.md §8a.

## Proposal

- `scripts/task` (bash, no dependencies) with verbs `fmt`, `fmt:check`, `lint`, `test`, `build`, `check`, plus `autotest` and `bench` for the client. `check` runs, in order and stopping at the first failure: `fmt:check`, the Luau check, `cairn check --render --strict`, `lint`, `test` (nextest over the workspace in release, then `rim test` and `rim check --strict`), `check-no-fma.sh` where it applies.
- `scripts/hooks/pre-push` runs `scripts/task check`; `scripts/task hooks` sets `core.hooksPath` to `scripts/hooks` (git hooks are per clone, so each worktree's clone installs once).
- CLAUDE.md and README point at `scripts/task check` as the gate.
- CI's jobs call these verbs in the CI lanes item; this item doesn't change `ci.yml`.

## Acceptance criteria

- [ ] `scripts/task check` on a clean main passes, and on a branch with a deliberate fmt error, clippy warning and failing test fails at the first, printing which verb failed (each shown)
- [ ] Every command CI's `checks` and `test` jobs run on Linux is run by `check` (a side-by-side list in this item)
- [ ] The pre-push hook blocks a push of a branch that fails `check`, and `scripts/task hooks` installs it
- [ ] CLAUDE.md and README name `scripts/task check` as the gate
