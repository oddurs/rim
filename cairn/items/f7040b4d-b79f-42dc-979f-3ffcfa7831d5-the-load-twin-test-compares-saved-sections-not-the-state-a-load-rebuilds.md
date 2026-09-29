---
id: f7040b4d-b79f-42dc-979f-3ffcfa7831d5
title: The load twin test compares saved sections, not the state a load rebuilds
type: chore
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
layer: tooling
area: tests
---

## Why

This review found six save/load divergences, all in state a load rebuilds rather than reads:
- worn garments on the item layer
- water cost, footing and rising
- the ground's value below the surface
- the rules' clock
- stale cover
- replay at a log's own tick

None was caught, because the twin tests compare what they can see:
- `snapshot.rs::carrying_on_after_a_load_matches_never_saving` compares saved sections.
- `corpus.rs` and `seeds::soak` check that save → load → save gives the same bytes.
- `state_hash` covers neither the map layers nor stock, water, `fields.layers[*].below` or `rules.on`.

Anything derived and rebuilt on load is invisible to all of them.

## What

A twin check over derived state: save at a tick that is on no interval (not a multiple of 20, 60, 250 or 5000), load, then compare live and loaded:
- right after the load and after one step: the map's layers (item, fixture, floor, water cost, footing, cover), stock and store index, fields (ambient, below, rooms), water basins, rules, regions and rooms;
- then run both for a while and compare `Snapshot::capture`.

Run it over the seed corpus with a colony that digs, floods, wears apparel and builds walls, in a test and in the nightly sweep.

## Acceptance

- [ ] The check exists and fails on each of the six bugs above (revert one fix to see it fail)
- [ ] It runs in the nightly seed sweep, and a short version in `scripts/task test`
