---
id: 508
uid: 9b435cd8-40ed-4c0f-8a41-ef3b892811c2
title: 'scripts/task: one gate for the laptop and CI, and a pre-push hook'
type: chore
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] `scripts/task check` on a clean main passes, and on a branch with a deliberate fmt error, clippy warning and failing test fails at the first, printing which verb failed (each shown)
- [x] Every command CI's `checks` and `test` jobs run on Linux is run by `check` (a side-by-side list in this item)
- [x] The pre-push hook blocks a push of a branch that fails `check`, and `scripts/task hooks` installs it
- [x] CLAUDE.md and README name `scripts/task check` as the gate

## 2026-09-27

What CI runs, and the verb that runs it locally. checks job: cargo fmt --check -> fmt:check; scripts/check-luau.sh with luau-lsp 1.70.0 -> types (the version is checked); cairn check --render --strict -> roadmap. test job on Linux: clippy --release --workspace --all-targets under RUSTFLAGS=-D warnings -> lint (clippy -- -D warnings: same coverage, no second build cache); nextest --release --workspace --profile ci -> test (--profile local: the same kill at 3 minutes, but stops at the first failure); cargo build --release --workspace --bins --examples -> build; check-no-fma.sh -> fma; rim test and rim check --strict -> mods; examples/crosscheck -> crosscheck (runs; CI's agree compares platforms). Not in check, by design: the sim job (scripts/task sim), the client job (scripts/task autotest, bench), and agree. Evidence: clean check passed on main (672 tests, 25 min cold under load); a formatting error, a clippy warning and a failing test each stopped check at fmt:check, lint and test, naming the step; the pre-push hook refused a badly formatted branch (never reached origin); a delete and a tag push skip it, a dirty tree or another commit is refused. An independent review found 8 issues (docs-only lane skipping tests that read docs/modding/*.md, the hook testing the tree instead of the pushed commit, deletes running the gate, crosscheck missing, no local timeout, no version pin, a latent pipefail, a stale comment); all fixed. cairn's version isn't pinned locally: it reports 1.0.0-alpha.1, CI builds rev c7d4230.
