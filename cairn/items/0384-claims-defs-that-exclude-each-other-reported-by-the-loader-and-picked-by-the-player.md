---
id: 384
uid: 96db8898-cf55-4517-acd7-260c05fe9d08
title: 'Claims: defs that exclude each other, reported by the loader and picked by the player'
type: feature
status: backlog
milestone: story
depends_on:
- 540
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
- [ ] Claims resolve by the contested-slot ladder (dbb92ebe): a compatibility mod, the pick, then load order, labelled; a claim never blocks a new game

## 2026-09-27

First non-story user: the storyteller itself (a77aec3a). Core's [[storyteller]] def claims "storyteller", and a replacement pacer mod claims it too, so claims must work for any def kind, not only threads.

## 2026-09-27

Decided 2026-09-27 (modding review, PR #246): a claim is one kind of contested slot, and every contested slot resolves by one ladder (dbb92ebe): a compatibility mod that depends on both contenders, then the player's pick, then load order, labelled. The old criterion that an unpicked claim blocks a new game is replaced: Content Patcher's Exclusive loads show where blocking leads (neither side applies), and a player with forty mods shouldn't face a questionnaire. The earlier note naming the storyteller as the first non-story user is superseded: the storyteller is a singleton kind now (a77aec3a), because with a claim, core's own storyteller would make every replacement a question.
