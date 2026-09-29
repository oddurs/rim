---
id: 591
uid: 2ede9239-d2f9-49a9-b7fa-c862a82fb846
title: 'The PR lane in under 7 minutes: the client autotest takes 14 under xvfb'
type: perf
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: m
layer: tooling
area: tests
---

## Problem

With main's caches warm, a PR run still takes 17m19s (https://github.com/oddurs/rim/actions/runs/36458855344, 2026-09-28). The Client job is the long pole at 16m47s: `cargo build --release -p rim_client` takes 1.6 min and the autotest under `xvfb-run` about 14. Test takes 8m13s, 5m37s of it `scripts/task test`, and Sim 1m45s. The lanes item (3f381240) wants a PR run under 7 minutes.

## Proposal

- Measure where the autotest's 14 minutes go (per section, from its log timestamps) before cutting anything.
- Then the likely levers, cheapest first:
  - Run the Client job on a PR only when it touches `crates/rim_client`, `crates/rim_ui`, `crates/rim_sim` or `mods/`, and always in the queue lane.
  - Shard the autotest across two or three runners by section.
  - Drop sleeps and fixed waits the autotest doesn't need under xvfb.
- The Test job: see if `scripts/task test`'s 5.6 minutes is build or run, and whether nextest's archive shared with Client saves a build.

## Acceptance criteria

- [x] A breakdown of the autotest's time by section, recorded here
- [x] A PR run on warm caches finishes in under 7 minutes (run linked), with every check that ran before still running in the PR or queue lane

## 2026-09-28

Breakdown, warm caches, run 36458855344: Client 16m47s = build 1m39s + autotest 4m14s + render bench 10m21s (1000 frames, 50-115 ms of software-GL submit each). Test 8m13s = lint 26s + compile 3m46s + run 1m50s + build/fma/mods/crosscheck ~1.5 min. Sim 1m45s. So the long poles are the render bench and Test's serial steps, not the autotest. Change: the render bench runs in the queue and nightly lanes, and on a PR only when it touches crates/rim_client/ or Cargo.lock (ci-plan's new render output; rim-c2 asked that a renderer PR not skip it); lint is its own job with its own cache; build, fma, mods and the Linux crosscheck move to the sim job, and agree takes Linux's crosscheck from there. scripts/test-ci-plan covers ten lane cases.

## 2026-09-28

Measured on this PR's own run, warm caches: https://github.com/oddurs/rim/actions/runs/36461782487, 6m24s from creation to the last job (was 17m19s). Test 6m14s (the long pole now), Build/mods/crosscheck/sim 4m57s, Client 4m39s (autotest, no bench: the PR doesn't touch the client), Lint 1m45s on a cold cache, checks 19s. Every check that ran before still runs: the render bench in the queue and nightly lanes and on client PRs, the rest in the PR lane.
