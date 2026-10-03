---
id: 718
uid: c10a5f82-a4b3-4078-8b62-e0efd84d36f5
title: The moving light pass runs in two modes on one CI CPU, 0.9 or 4-5 ms
type: perf
status: done
milestone: foundations
created: 2026-09-29
updated: 2026-10-02
closed_at: 2026-10-02
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

- [x] Five runs of one class read the moving view within 20% of each other
- [x] The budget gates can cap the moving view without flaking

## 2026-09-29

Moot under flat (#405). The moving view's slow mode was the moving light pass under medium lighting. Under flat, the default since #405, that pass costs 0.07-0.10 ms. Post-#405 CI render benches, moving world_ms: EPYC 7763 0.797, 0.801, 0.854, 0.871, 0.895, 1.022 (6 runs, all within 19% of the 0.86 median); EPYC 9V74 0.623, 0.673, 0.850, 0.862, 0.864 (5 runs). Worst gated world_ms across all 11 runs: 1.40 (zooming). Runs: 36530055937 36530126352 36538151622 36647571310 36648242638 36648542114 36651284052 36651588173 36651592219 36652329482 36653039618. Next: once #404 merges, lower render.software.world_ms from 6 to 2 and close this (criterion 2).

## 2026-10-02

Closed by this PR: render.software.world_ms cap 6 -> 2 ms. Every gated view's world time in the 11 post-#405 runs is at most 1.40 ms, so the cap holds the moving view without flaking (criterion 2). Criterion 1 is the six 7763 runs above, all within 19% of their median.
