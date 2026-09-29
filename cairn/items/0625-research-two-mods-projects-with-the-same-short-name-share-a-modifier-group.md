---
id: 625
uid: 62658665-3d9f-4fd8-adba-c2d4c2017a2b
title: 'Research: two mods'' projects with the same short name share a modifier group'
type: bug
status: backlog
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
effort: s
layer: plugin
area: modding
---

## Why

mods/research/scripts/research.luau keys a project's modifier group by short(def.id). Projects a:smithing and b:smithing both switch the group smithing, and research:smithing, so finishing one unlocks the other's buildables.

## Acceptance criteria

- [ ] Two projects with the same short name in different mods unlock only their own modifiers (test that fails before the fix)
