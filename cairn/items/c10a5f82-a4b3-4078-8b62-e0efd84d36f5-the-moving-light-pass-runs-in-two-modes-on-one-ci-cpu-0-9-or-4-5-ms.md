---
id: c10a5f82-a4b3-4078-8b62-e0efd84d36f5
title: The moving light pass runs in two modes on one CI CPU, 0.9 or 4-5 ms
type: perf
status: backlog
milestone: foundations
created: 2026-09-29
updated: 2026-09-29
priority: p2
api: none
effort: s
layer: client
area: perf
---

The CI render bench's `moving` view comes out at one of two world_ms on the same EPYC 7763 runner: about 0.9 ms or about 4-5 ms. Commit 0920c20b measured 0.89 (run 36517204597) and 4.12 (run 36518050170). Nearly all of it is the moving light pass's CPU: about 0.5 ms in the fast mode, 3.5-4.8 in the slow one. Other classes (Xeon 6973P, EPYC 9V74) sit at 2.4-2.8 ms.

It made #390 look like a 0.91 to 5.25 ms regression. Twenty-four render-bench artifacts (`views[].world_ms`, CPU in `machine`) showed it was noise. #390 touches no lighting code.

While the pass flips like this, one run can't gate it, and an A/B can't read it.

## Budget

A number that stays within 20% across runs of one class, so the budget gates (#404) can hold it.

## Measurement (before)

EPYC 7763, moving world_ms: 0.87, 0.89, 0.91, 0.89 in the fast mode; 3.89-5.26 in the slow one (11 runs).

## Approach

Find what the two modes differ in: llvmpipe thread count, a GL flush landing inside the pass, or the bake landing in a measured frame. Then pin it, or measure the pass so the difference drops out. If #405 (one lighting, flat) removes the moving pass, close this as done.

## Acceptance criteria

- [ ] Five runs of one class read the moving view within 20% of each other
- [ ] The budget gates can cap the moving view without flaking
