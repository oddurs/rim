---
id: 641
uid: 7a66328a-4c84-4828-80c8-0e9c798d5f6d
title: A quoted def kind nobody declared panics the mod loader
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: modding
---

## What

`modloader.rs` (~line 129) takes any top-level key that contains a colon as a mod-declared kind: `if key == "patch" || KINDS.contains(..) || key.contains(':') { key.clone() }`. The dotted form `[[magic.spell]]` checks `kinds.contains_key` and warns "unknown def kind … ignored". The quoted form `[["magic:spell"]]` skips that check, and the def reaches `let decl = &kinds[mod_kind];` (~line 315), a `BTreeMap` index that panics with "no entry found for key".

## How it fails

The game panics at load, instead of printing a warning or a load error, when a mod has:
- a typo in a quoted kind,
- a kind from an optional mod that isn't installed, or
- a built-in kind written with a prefix (`[["core:thing"]]`).

Found by the defs/modloader review sweep.

## Reproduce

In `tests/kinds.rs`: `load("kind-quoted", KIND, "[[\"magic:nope\"]]\nid = \"x\"\n", "")` panics.

## Acceptance

- [ ] A quoted kind nobody declared is warned about and ignored, as the dotted form is
- [ ] A test that fails before the fix and passes after
