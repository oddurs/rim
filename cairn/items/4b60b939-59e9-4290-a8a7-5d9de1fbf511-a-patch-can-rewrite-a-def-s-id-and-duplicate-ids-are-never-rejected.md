---
id: 4b60b939-59e9-4290-a8a7-5d9de1fbf511
title: A patch can rewrite a def's id, and duplicate ids are never rejected
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: engine
area: modding
---

## What

`apply_patch` → `merge` (modloader.rs ~551-553, ~697-730) accepts any key, `id` included. The loader's `index` keeps the old id, but deserialization reads the new one. `DefDb::finalize` (defs.rs ~2322) then does `index.insert(("thing", d.id.clone()), i)`, which silently overwrites a duplicate.

## How it fails

- `set = { id = "core:floor" }` on `thing/core:wall` makes two things with one id; lookups reach only the later one, and a save maps both to the first.
- An id without a prefix makes every bare reference in that def fail with a confusing `unknown designation ':chop'`.

## Reproduce

Not yet run; verified by reading. A tests/patches.rs two-mod setup patching `id`; it should be a load error.

## Fix

Reject `id` in a patch's `set`, and have `finalize` refuse duplicate ids.

## Acceptance

- [ ] Both are load errors that name the def
- [ ] A test that fails before the fix and passes after
