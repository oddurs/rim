---
id: 4b60b939-59e9-4290-a8a7-5d9de1fbf511
title: A patch can rewrite a def's id, and duplicate ids are never rejected
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-28
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

- [x] Both are load errors that name the def
- [x] A test that fails before the fix and passes after

## 2026-09-28

finalize's duplicate check is not added: the loader already refuses a def defined twice (modloader.rs, 'is already defined by ... use a [[patch]]'), and a patch's set was the only way to make two defs share an id. With id refused there, no mod can reach finalize with a duplicate, so a second check would be dead code.

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/4b60b939-patch-id and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
