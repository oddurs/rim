---
id: aa9610a0-c8b2-4352-af66-1ef3aa13e69c
title: scripts/task runs cairn at the pinned rev, installed into the project
type: chore
status: backlog
milestone: proving-ground
depends_on:
- 9b435cd8-40ed-4c0f-8a41-ef3b892811c2
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Problem

`scripts/task roadmap` runs whatever `cairn` is on PATH. A global install of another version broke every agent's writes on 2026-09-27, and a different version can pass or fail `roadmap` differently from CI.

## Proposal

`scripts/task` installs cairn at the rev CI pins with `cargo install --locked --git https://github.com/oddurs/cairn --rev <rev> --root .tools/cairn` (gitignored) on first use, and runs that binary. The rev is written once, in scripts/task, and CI reads it from there, as luau-lsp's version already is.

## Acceptance criteria

- [ ] With a different cairn first on PATH, `scripts/task roadmap` uses the pinned one (shown)
- [ ] CI installs cairn at the rev read from scripts/task
