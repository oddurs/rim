---
id: 6d81840b-2d4b-4f63-8c09-f09801df95d9
title: 'New colony setup: premise, storyteller, world, colonists, the set'
type: feature
status: backlog
milestone: rimos
depends_on:
- 13a7b680-8a61-4379-9b70-fa6843a45464
- 3e96a103-63d1-494f-8744-a2f9dbcadc35
- 47ec1fa0-6c5e-4511-86bd-3a1fe263d8c1
- 73751f4f-cd52-467a-9098-55d8033e4b4b
- e874ca2d-e8d5-4af2-af7f-59817a5a8da4
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: additive
effort: l
layer: core
area: ui
---

## Problem

New colony starts at once, with the launch time as the seed and no choices.

## Proposal

One stepped app hosts the pieces other items build:
- the premise (e874ca2d);
- the storyteller;
- the world: seed code (47ec1fa0), latitude, map size, starting season;
- the cast card (3e96a103);
- the set of mods (73751f4f).

Every step has a default, so Start is one click from the first step.

## Acceptance criteria

- [ ] Start with every default gives the same colony as today's New colony
- [ ] A typed seed code and latitude reach the new world (test)
