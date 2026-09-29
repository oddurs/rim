---
id: 2fbd4815-48e7-455d-b716-812a3492df37
title: A metamethod can re-enter the world through a binding's table argument
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: scripting
---

## What

`with_world` hands a binding `&mut World`, and several bindings then read the Lua tables they were given (`store_put` ~1879, `post_order` ~2012-2033, `stock` ~1943, `count_items` ~1918). An `__index` metamethod on such a table can call another `rim.*` function, which makes a second live `&mut World` from the same pointer. That is undefined behaviour under stacked borrows, so the SAFETY comment at script.rs ~34 doesn't hold.

## How it fails

In practice it's small (a nested `rim.remove(site)` makes an `insert_one` fail silently), but it's unsound.

## Reproduce

Not yet run; verified by reading. A table with an `__index` that calls `rim.remove`, passed to `rim.post_order`.

## Fix

Read every table argument into Rust values before calling `with_world`, or refuse re-entry while a world borrow is live.

## Acceptance

- [ ] Re-entering the world from inside a binding is impossible or a script error
- [ ] A test that fails before the fix and passes after
