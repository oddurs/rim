---
id: 2ede9239-d2f9-49a9-b7fa-c862a82fb846
title: 'The PR lane in under 7 minutes: the client autotest takes 14 under xvfb'
type: perf
status: backlog
milestone: proving-ground
created: 2026-09-28
updated: 2026-09-28
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

- [ ] A breakdown of the autotest's time by section, recorded here
- [ ] A PR run on warm caches finishes in under 7 minutes (run linked), with every check that ran before still running in the PR or queue lane
