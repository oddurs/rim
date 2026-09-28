---
id: aa9610a0-c8b2-4352-af66-1ef3aa13e69c
title: scripts/task runs cairn at the pinned rev, installed into the project
type: chore
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
depends_on:
- 9b435cd8-40ed-4c0f-8a41-ef3b892811c2
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] With a different cairn first on PATH, `scripts/task roadmap` uses the pinned one (shown)
- [x] CI installs cairn at the rev read from scripts/task

## 2026-09-28

scripts/task names the rev once (CAIRN_REV=c7d4230) and runs cairn from ${XDG_CACHE_HOME:-~/.cache}/rim/cairn-<rev>, installing it there on first use (31 s here). Not .tools/ in the project, as proposed: every agent worktree would build its own. The checks job reads the rev from scripts/task, caches that directory under cairn-root-<rev>, and main's roadmap render runs the same binary. Shown: with a stub cairn that exits 9 first on PATH, scripts/task roadmap printed 'cairn c7d4230 (~/.cache/rim/cairn-c7d4230/bin/cairn)' and passed.

## 2026-09-28

CI: the PR run https://github.com/oddurs/rim/actions/runs/36466052808 missed the new cache key (cairn-root-c7d4230), and scripts/task roadmap installed cairn c7d4230 in 40 s, ran it from /home/runner/.cache/rim/cairn-c7d4230/bin/cairn and passed (570 items).
