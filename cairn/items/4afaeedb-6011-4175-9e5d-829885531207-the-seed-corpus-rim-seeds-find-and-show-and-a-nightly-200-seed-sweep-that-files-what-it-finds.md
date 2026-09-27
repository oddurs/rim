---
id: 4afaeedb-6011-4175-9e5d-829885531207
title: The seed corpus, rim seeds find and show, and a nightly 200-seed sweep that files what it finds
type: feature
status: backlog
milestone: proving-ground
depends_on:
- 3f381240-d490-4a6a-beb0-27f201dddb79
- 7dcb8a90-e24a-4547-9f2e-2259ee83bf4b
created: 2026-09-27
updated: 2026-09-27
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

- [ ] `rim seeds find --where "river near start" --n 3` prints three seeds whose maps have one (checked by eye with `show`)
- [ ] A nightly run by `workflow_dispatch` completes 200 seeds and reports the date seed (linked)
- [ ] A deliberately broken seed in a nightly run produces an item with a working repro line (shown)
