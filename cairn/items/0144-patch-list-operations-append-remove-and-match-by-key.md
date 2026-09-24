---
id: 51a9c5c6-1136-44b9-98be-d72f8c986b0c
title: 'Patch list operations: append, remove and match by key'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-23
updated: 2026-09-23
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

- [ ] `append = { butcher = [...] }` and `remove = { needs = ["core:rest"] }`
- [ ] Lists of tables can be matched by a key field (`match = { thing = "core:raw_meat" }`) and edited in place
- [ ] Two mods appending to the same list is not a conflict; two mods setting the same matched element is
- [ ] Conflict warnings and the compatibility report show list edits
