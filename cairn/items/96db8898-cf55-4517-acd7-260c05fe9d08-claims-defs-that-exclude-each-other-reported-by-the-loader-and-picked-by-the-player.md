---
id: 96db8898-cf55-4517-acd7-260c05fe9d08
title: 'Claims: defs that exclude each other, reported by the loader and picked by the player'
type: feature
status: backlog
milestone: story
created: 2026-09-26
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Two story threads that both pace the early raids can't both run. The loader
should say so, as it does for patch conflicts, and never pick silently (DESIGN.md
§4g, stacking).

## Acceptance criteria

- [ ] Any def may declare `claims = [...]`
- [ ] Two enabled defs claiming a tag are reported as a conflict with both names
- [ ] The player picks one; the pick is kept in the save (the modlist lockfile takes it when it lands)
- [ ] With no pick, the conflict blocks a new game rather than choosing

## 2026-09-27

First non-story user: the storyteller itself (a77aec3a). Core's [[storyteller]] def claims "storyteller", and a replacement pacer mod claims it too, so claims must work for any def kind, not only threads.
