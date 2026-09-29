---
id: 422
uid: e641fbdb-1f13-4fe1-b382-c37623228885
title: 'story plugin: threads and beats with conditions, premise binding and claims'
type: feature
status: backlog
milestone: story
depends_on:
- 310
- 384
- 390
- 406
created: 2026-09-26
updated: 2026-09-27
priority: p0
api: additive
effort: l
layer: plugin
area: storyteller
pillar:
- plugin-first
- growth
---

## Why

Threads are how stories stack (DESIGN.md §4g).

## What

`mods/story` declares `thread` and `beat` kinds. A beat has conditions (era,
day, an event and its data, a relation threshold), ordering (`after`), an
offer (an incident and weight) and a text key. Conditions are checked daily
and on the events a thread names.

## Acceptance criteria

- [ ] Thread and beat kinds, with premise binding and claims
- [ ] Conditions evaluated daily and on named events, never per tick
- [ ] Beat state saved in the plugin's data; fired beats become memories
- [ ] `rim test` helpers: assert a beat fired, by day, across seeds

## 2026-09-27

Beats offer into core's incident registry, not into a particular pacer: a77aec3a splits the storyteller into the registry (shared vocabulary) and a claimed pacer that spends the tension budget. Whatever pacer holds the storyteller claim sees offers in its pool.

## 2026-09-27

Superseding the earlier note: the storyteller is a singleton kind with a replaceable pacer (a77aec3a), not a claim. Beats still offer into core's incident registry; whichever pacer the one [[storyteller]] names sees them in its pool.
