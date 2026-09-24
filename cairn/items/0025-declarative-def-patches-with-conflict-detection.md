---
id: 69816584-120b-45fa-b00c-aa229a9c0359
title: Declarative def patches with conflict detection
type: feature
status: done
milestone: foundations
depends_on:
- 4504c561-95b3-4d62-8691-3a129260a3c9
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Most mods change existing content. Patches must compose, and two mods setting one field must be reported rather than silently resolved.

## Acceptance criteria

- [x] [[patch]] target = "kind/id" with set = {...} deep-merges
- [x] remove = true deletes a def
- [x] Conflicting leaf writes from two mods produce a warning naming both
- [x] Patching a missing optional target warns and continues
