---
id: 0fedb3d8-f2b2-49a9-9b60-685a3292de25
key: proving-ground
title: Proving ground
type: milestone
status: planned
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
---

Seeds, tests, CI and the merge queue, so that every change is checked the same way on a laptop and in CI, every failure can be brought back with one line, and a PR lands without anyone rebasing it by hand. Design: DESIGN.md §7b, §8a.

## Goal

`scripts/task check` is the whole gate, and CI runs it. A PR push gets a Linux lane in about five minutes; adding the `queue` label hands the PR to Mergify, which tests it on top of main on four platforms and merges it. No test in the suite asserts wall-clock time. Every test failure prints a one-line repro, a nightly sweep runs 200 seeds and files what it finds, and random draws come from a stream per purpose, so a change in one system no longer reshuffles every other.

## Order

1. **The gate:** `scripts/task` and the pre-push hook. Everything else calls it.
2. **Trust the suite:** the wall-clock asserts become work counts.
3. **CI lanes**, then **the merge queue on Mergify**, then **required checks** on main.
4. **Seeds:** streams per purpose, a seed per test with the nightly shift and repro line, then the corpus and nightly sweep.
5. **Deeper checks:** visual regression, property tests, bench baselines, replay bisect, then fuzzing and balance across seeds.

## What this waits on elsewhere

- Mergify has to be installed on the repo by its owner (a GitHub App install); the queue item can be built and dry-run only after that.
- rim-c2 runs today's scripted queue. The Mergify item retires it with rim-c2's agreement, after a dry run.
- PR #138 (c97f1924, only on its branch) is superseded by the CI lanes item, which keeps its change detection.

## Not in this milestone

- Moving the repo to an organization, Blacksmith, and private-repo billing: the repo stays public (decided 2026-09-27).
- The Mac mini runner: later, filed without a milestone.
- In-game developer tools: the Workbench milestone.

## Where it sits

First. Every other milestone lands through this gate and this queue, so each day it waits costs rebases, cancelled runs and flakes. Due date: not set, since agents only read `due`.
