---
id: 7a66328a-4c84-4828-80c8-0e9c798d5f6d
title: A quoted def kind nobody declared panics the mod loader
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-28
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

- [x] A quoted kind nobody declared is warned about and ignored, as the dotted form is
- [x] A test that fails before the fix and passes after

## 2026-09-29

PAUSED at the merge freeze: the fix and its test are done, and the test was checked to fail before the fix and pass after. The full gate hasn't run yet. Next: rebase on main, run scripts/task check, and mark the PR ready. Branch fix/7a66328a-quoted-kind.
