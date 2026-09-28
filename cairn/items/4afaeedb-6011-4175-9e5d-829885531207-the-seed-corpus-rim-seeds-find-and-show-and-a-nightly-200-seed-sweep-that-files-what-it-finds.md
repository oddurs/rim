---
id: 4afaeedb-6011-4175-9e5d-829885531207
title: The seed corpus, rim seeds find and show, and a nightly 200-seed sweep that files what it finds
type: feature
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
depends_on:
- 3f381240-d490-4a6a-beb0-27f201dddb79
- 7dcb8a90-e24a-4547-9f2e-2259ee83bf4b
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: m
layer: tooling
area: tests
---

## Problem

Maps worth testing on are found by luck and lost when a test changes, and the balance, soak and autotest sweeps over many seeds run only by hand. DESIGN.md §7b.

## Proposal

- `tests/seeds.toml`: `[[seed]]` entries with `id`, `seed`, `why`, optional `item`. Seeded property tests read it.
- `rim seeds find --where <predicate> --n N` runs mapgen alone and prints matching seeds; `rim seeds show <seed> --out map.png` renders a map.
- The nightly lane runs 200 seeds derived from the date through the soak and the autotest scenes; each failure opens a cairn item (through a PR) with its repro line, and the fixed seed joins the corpus.

## Acceptance criteria

- [x] `rim seeds find --where "river near start" --n 3` prints three seeds whose maps have one (checked by eye with `show`)
- [x] A nightly run by `workflow_dispatch` completes 200 seeds and reports the date seed (linked)
- [x] A deliberately broken seed in a nightly run produces an item with a working repro line (shown)

## 2026-09-28

Built: rim_sim::seeds (the corpus, questions about the land near the start, soak, the night's seeds from its date) and rim seeds find | show | soak | sweep in the client binary. Maps here have lakes, not rivers (water comes from elevation bands), so a question names a terrain id, a terrain tag or a thing id: 'water near start', 'no water within 40 of start', 'oak within 6 of start'. The corpus lives at crates/rim_sim/tests/seeds.toml (three seeds so far) and tests/corpus.rs plays each for a day and round-trips its save; the property tests don't read it. Criterion 1: 'rim seeds find --where "water near start" --n 3' printed seeds 1, 5 and 6 (water 1, 3 and 3 cells from the start); rim seeds show rendered each (start ringed in red) and each has a lake at the start's edge. Actions can't open a PR here (can_approve_pull_request_reviews false), so a sweep failure becomes a cairn item in the sweep-items artifact and the run summary, not a PR; turning on 'Allow GitHub Actions to create and approve pull requests' would let it open one. The sweep plays the soak only, not the autotest scenes: an autotest run takes four minutes, so 200 of them don't fit a night.

## 2026-09-28

Criteria 2 and 3, from a nightly dispatched on a throwaway branch (deleted) that planted a panic at tick 5000 on the night's first seed: https://github.com/oddurs/rim/actions/runs/36478356011. The sweep job played 200 seeds of 20260928 in about a minute, reported the date seed ('the first is 2162083801786920'), failed with 199 of 200, and filed 90ce8e35 into the sweep-items artifact with the repro 'rim seeds soak --seed 2162083801786920 --days 2'. Run as printed on the planted build it failed the same way (tick 5000); without the plant it passed. An earlier run (36475811797) found the step went green through tee without pipefail; the step now runs under bash (-eo pipefail).
