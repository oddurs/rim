---
id: 3f381240-d490-4a6a-beb0-27f201dddb79
title: 'CI lanes for a public repo: a Linux PR lane, a four-platform queue proof, nightly'
type: chore
status: backlog
milestone: proving-ground
depends_on:
- 9b435cd8-40ed-4c0f-8a41-ef3b892811c2
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: tooling
area: tests
---

## Problem

Every PR push runs seven jobs on Linux, macOS, Windows and ARM and cancels its previous run, main included. With about 240 runs a day and 20 concurrent jobs (5 macOS) on a public repo, runs queue for runners and a fifth are thrown away. No job has a timeout, the toolchain version is written five times, and the client and sim jobs recompile what the test job built. PR #138 (c97f1924) proposed a cut for billing reasons when the repo was private. DESIGN.md §8a.

## Proposal

- One `plan` job (from #138): decides the lane and whether anything but `cairn/` and Markdown changed; drafts run nothing.
- **PR lane** (`pull_request`, cancellable): Linux x64 only: checks, clippy and nextest in two partitions, the client autotest and render bench. One release build, shared by the jobs as an artifact.
- **Queue lane** (Mergify's queue branches, and `workflow_dispatch`; never cancelled): the PR lane plus the crosscheck and save round trip on Linux ARM, Windows and macOS, and `agree`.
- **Nightly** (`schedule` on main): full nextest on macOS and Windows, and a place for the sweep, fuzzing and bench trend items to add jobs.
- A push to main runs nothing heavy. `timeout-minutes` on every job. The toolchain comes from `rust-toolchain.toml`. Every job calls `scripts/task` verbs.
- Close PR #138 with a note pointing here.

## Acceptance criteria

- [ ] A PR run starts only Linux jobs and finishes in under 7 minutes (run linked, timings recorded)
- [ ] A queue-lane run (by `workflow_dispatch` on a branch) runs all four platforms and `agree` (run linked)
- [ ] A docs-only PR runs the checks job alone, and a draft runs nothing (runs linked)
- [ ] A nightly run by `workflow_dispatch` runs the full macOS and Windows suites (run linked)
- [ ] Every job has a timeout; only PR runs cancel in progress
- [ ] PR #138 closed with a pointer to this item
