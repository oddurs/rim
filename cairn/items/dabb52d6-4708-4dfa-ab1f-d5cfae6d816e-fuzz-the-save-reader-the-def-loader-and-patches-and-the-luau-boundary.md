---
id: dabb52d6-4708-4dfa-ab1f-d5cfae6d816e
title: Fuzz the save reader, the def loader and patches, and the Luau boundary
type: feature
status: backlog
milestone: proving-ground
depends_on:
- 3f381240-d490-4a6a-beb0-27f201dddb79
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: m
layer: tooling
area: tests
---

## Problem

The save reader, the def loader and the Luau API take input from files and mods that rim doesn't control, and nothing throws malformed input at them.

## Proposal

`cargo-fuzz` targets for `savefile::read`, the def loader with patches, and the script API's argument decoding; the nightly lane runs each for 30 minutes; crashes are kept in a corpus that runs as regression tests.

## Acceptance criteria

- [ ] Three targets build and run nightly (linked)
- [ ] Any crash found is fixed with its input kept as a test, or filed as an item
