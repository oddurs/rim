---
id: 51a9c5c6-1136-44b9-98be-d72f8c986b0c
title: 'Patch list operations: append, remove and match by key'
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Setting a list in a `[[patch]]` replaces the whole list (`merge` only deep-merges tables). Adding one butcher yield or one spawn terrain means copying and owning the list, which then conflicts with every other mod that did the same.

## Acceptance criteria

- [x] `append = { butcher = [...] }` and `remove = { needs = ["core:rest"] }`
- [x] Lists of tables can be matched by a key field (`match = { thing = "core:raw_meat" }`) and edited in place
- [x] Two mods appending to the same list is not a conflict; two mods setting the same matched element is
- [ ] Conflict warnings and the compatibility report show list edits

## 2026-09-24

append/remove take tables shaped like the def down to the lists; remove matches scalars by equality and tables by a subset of keys (so it doubles as remove-by-key). [[patch.edit]] has list (dotted path), match and set; set merges into every matching element and records set_by under list[key=value], so two mods setting the same matched field conflict like any field. Order within a patch: set, edit, remove, append. Unknown patch keys are now errors (a typo like 'sett' used to be ignored). Criterion 4: conflict warnings cover list edits (a whole-list set after another mod's edit is reported); the compatibility report doesn't exist yet, so that half is noted on 0123 rather than ticked here. Docs: docs/modding/patches.md, the first modder guide to patching, with its samples loaded by a test.
