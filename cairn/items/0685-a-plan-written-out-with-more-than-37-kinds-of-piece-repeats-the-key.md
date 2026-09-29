---
id: 685
uid: e42f7f4a-8cf0-43b7-9064-a39abe4e21d7
title: A plan written out with more than 37 kinds of piece repeats the `?` key
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: building
---

## What

`plan::plan_text` (crates/rim_sim/src/plan.rs) gives each (thing, material, facing) a character: the label's first letter, else the first free one of `#a-z0-9`. Past 37 kinds, `unwrap_or('?')` hands every further kind `?`. The grid then can't tell those kinds apart, and the legend holds the key `"?"` more than once, which is invalid TOML.

## Reproduce

Verified by reading. Plan out a stretch with 38 distinct piece kinds (materials times facings get there quickly), then parse the text back: a TOML duplicate-key error.

## Direction

Draw from a larger pool (upper case letters and other printable ASCII), and refuse with a message rather than write a plan that won't load back.

## Acceptance

- [ ] `plan_text` never writes a duplicate key, and any text it writes loads back
- [ ] A test that fails before the fix and passes after
