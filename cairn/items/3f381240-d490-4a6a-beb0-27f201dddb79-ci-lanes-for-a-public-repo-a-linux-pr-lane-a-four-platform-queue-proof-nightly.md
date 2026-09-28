---
id: 3f381240-d490-4a6a-beb0-27f201dddb79
title: 'CI lanes for a public repo: a Linux PR lane, a four-platform queue proof, nightly'
type: chore
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
depends_on:
- 9b435cd8-40ed-4c0f-8a41-ef3b892811c2
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] A PR run starts only Linux jobs and finishes in under 7 minutes (run linked, timings recorded)
- [x] A queue-lane run (by `workflow_dispatch` on a branch) runs all four platforms and `agree` (run linked)
- [x] A docs-only PR runs the checks job alone, and a draft runs nothing (runs linked)
- [x] A nightly run by `workflow_dispatch` runs the full macOS and Windows suites (run linked)
- [x] Every job has a timeout; only PR runs cancel in progress
- [x] PR #138 closed with a pointer to this item

## 2026-09-27

Built: scripts/ci-plan picks the lane (pr, queue, nightly, main, none) from the event, and every event type was run through it locally. The PR lane is Linux only; the queue lane (a PR pushed while labelled full-ci, Mergify's mergify/merge-queue/* PRs, or workflow_dispatch lane=queue) adds macOS and Windows proof jobs (save round trip, crosscheck, FMA check on macOS), the ARM crosscheck with the save round trip and FMA, and agree; nightly (schedule, main) runs the proof plus full lint, test and mods on macOS and Windows, which also saves their caches. A push to main runs checks only. Every job has a timeout; only PR runs cancel in progress, and Mergify's queue PRs never do. The labeled trigger was removed after review: a run for a label would skip every job, and a skipped check passes a ruleset. rim-c2's queue script requires a SUCCESS agree on the readied sha and dispatches the queue lane itself. An independent review found 7 problems, all fixed. Criteria 1-4 need real runs and are ticked from this PR's own runs and the first dispatch and nightly after merge.

## 2026-09-28

Runs on 97c09a9e (base b591c0c3, before #290's autotest founder fix, so Client fails in all three with the known autotest.rs:97 panic):
- PR lane https://github.com/oddurs/rim/actions/runs/36379055603 : Plan, checks, Test (ubuntu), Sim, Client only; Proof, Nightly, ARM crosscheck and agree skipped. Plan start 05:00:19 to last job 05:07:35 = 7m16s (50 s of it waiting for a runner); Test job 6m21s on a cold PR cache.
- Queue lane https://github.com/oddurs/rim/actions/runs/36379984853 : Proof macOS 3m28s, Proof Windows 4m15s, ARM crosscheck 2m10s, Test ubuntu 5m12s, agree success.
- Nightly lane https://github.com/oddurs/rim/actions/runs/36379069658 : Nightly macOS 6m42s, Nightly Windows 11m29s, both proofs, ARM, agree success.
Every job has timeout-minutes (plan 5, checks 15, test 45, proof 30, crosscheck-arm 30, agree 5, sim 20, client 20, nightly 60), and cancel-in-progress is true only for pull_request events outside mergify/merge-queue/*. #138 closed with a pointer here. The checks job now also runs scripts/task scripts (#308's hook tests). Left for after merge: criterion 1 on a run with main's warm cache (this cold run took 7m16s wall, 16 s over), and criterion 3 from the cairn-only PR that closes this item, opened as a draft first.

## 2026-09-28

Merged as #287 (c35cb3cb). main's push run https://github.com/oddurs/rim/actions/runs/36455234599 ran Plan and checks only, 29 s. A nightly dispatched on main right after (https://github.com/oddurs/rim/actions/runs/36455392607) saves main's caches for PR runs.

## 2026-09-28

Criterion 3: the draft run https://github.com/oddurs/rim/actions/runs/36455454309 ran Plan only (6 s) and skipped every other job; the same PR marked ready, docs only, ran Plan and checks alone: https://github.com/oddurs/rim/actions/runs/36455520184 (attempt 2; attempt 1 was cancelled from outside), 32 s. Criterion 1 is not met: on warm caches a PR run took 17m19s (https://github.com/oddurs/rim/actions/runs/36458855344), with Client's autotest under xvfb about 14 minutes and Test 8m13s. Filed as 2ede9239-d2f9-49a9-b7fa-c862a82fb846; this item closes when that lands.

## 2026-09-28

Criterion 1: with 2ede9239 a PR run on warm caches starts only Linux jobs and takes 6m24s: https://github.com/oddurs/rim/actions/runs/36461782487 (Test 6m14s, Build/mods/crosscheck/sim 4m57s, Client 4m39s, Lint 1m45s, checks 19s).
