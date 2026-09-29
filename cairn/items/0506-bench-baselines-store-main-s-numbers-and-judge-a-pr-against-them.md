---
id: 506
uid: 932f2bfd-3a53-4897-a943-d8cfba98c470
title: 'Bench baselines: store main''s numbers and judge a PR against them'
type: feature
status: backlog
milestone: proving-ground
depends_on:
- 467
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: tooling
area: perf
---

## Problem

The sim bench (2 ms) and render bench (4 ms) gate CI against absolute budgets with 3× and 1.5× slack, so a 40% regression can pass and a slow runner can fail.

## Proposal

- Both benches write `--json`.
- The queue lane stores main's numbers per runner type (an artifact or a `bench-data` branch); a PR's numbers are compared with the latest main on the same runner type, failing on a regression beyond the measured noise.
- A small trend page (Markdown table) regenerated nightly.

## Acceptance criteria

- [ ] A deliberate 30% slowdown in a system fails the comparison (shown)
- [ ] Ten runs of an unchanged main stay inside the tolerance (numbers recorded)
