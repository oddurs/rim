---
id: 6a6a4828-3f5d-46ca-8e29-5cd569791f20
title: Autotest seeds from the clock, so CI rolls dice
type: bug
status: done
milestone: shelter
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: none
layer: tooling
area: tests
effort: s
---

## What happens

## What should happen

## Reproduction

Seed:
Mods:
Tick:

1.
## Why

`rim --autotest` with no `--seed` falls back to the wall clock, so every CI
run tests a different map. Some maps do not satisfy the test's assumptions
and the run fails through no fault of the change under test.

Measured on `main` at 18224e9, seeds 9-44: six of thirty-six fail.

- `dragging designates trees (0)` on seeds 20, 26, 29, 40, 41 — the drag box
  is `home ± 8`, and on those maps it holds no oak.
- `the warrior chopped and built walls (0)` on seeds 18, 20, 26, 29, 40, 41 —
  downstream of the same thing, or of no wood being reachable in time.

Roughly one run in six is red for reasons unrelated to the diff, which
trains everyone to re-run a red CI instead of reading it.

## Acceptance criteria

- [x] The autotest runs on a fixed seed unless one is given
- [x] Checks that need a tree, or open ground, find one or skip explicitly
- [x] A seed sweep in CI, or a documented command, covers map variation
- [x] Seeds 9-44 pass

## 2026-09-23

Found while adding right-click orders (0071). Confirmed pre-existing: the same failures reproduce on main at 18224e9, and the 0071 branch passes every seed tested (1-8, 18, 20, 26, 29, 40, 41) on the new checks. Sweep: for s in $(seq 9 44); do ./target/release/rim --seed $s --autotest /tmp/at; done

## 2026-09-24

rim --autotest without --seed now uses seed 7 (play still gets a new world). The chop drag is centred on the tree nearest the colonist rather than a fixed home±8 box, the build phase runs until walls stand (up to 12,000 ticks), and the campfire check picks open ground outdoors (seed 37 put it inside the hut, where temperature is the room's own value). scripts/autotest-sweep.sh FIRST LAST covers map variation (in the README). Sweep 9-44: all 36 pass, including 18, 20, 26, 29, 40 and 41, which failed before.
