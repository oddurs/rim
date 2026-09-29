---
id: 615
uid: 4a11812f-a04e-4dc8-abbe-1c7657eda483
title: scripts/task runs rim and the crosscheck from CARGO_TARGET_DIR
type: bug
status: done
milestone: proving-ground
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
effort: s
layer: tooling
area: tests
---

## Problem

`scripts/task mods` and `crosscheck` run `target/release/rim` and `target/release/examples/crosscheck` by path, so they exit 127 for anyone whose `CARGO_TARGET_DIR` points elsewhere, and `sim` and `autotest` write their reports into a `target/` that may not exist then (reported by quiet-field, 2026-09-28).

## Acceptance criteria

- [x] With CARGO_TARGET_DIR set, `mods` and `crosscheck` run the binaries built there (a test with stub binaries)
- [x] Without it, they run `target/release/...` as before
- [x] `sim` and `autotest` still write their reports under the repo's `target/`

## 2026-09-28

scripts/task runs rim and the crosscheck from ${CARGO_TARGET_DIR:-target}/release, and sim and autotest make the repo's target/ before writing their reports there (CI uploads target/climate-report.txt). scripts/test-task stubs the binaries in a throwaway CARGO_TARGET_DIR and a cargo on PATH: its three cases pass, and against the old script all three fail (mods and crosscheck exit 127, sim exits 1). The gate's own mods and crosscheck steps run the default target/ path. scripts/task scripts runs test-task beside test-pre-push.
