---
id: 632
uid: 6d81840b-2d4b-4f63-8c09-f09801df95d9
title: 'New colony setup: premise, storyteller, world, colonists, the set'
type: feature
status: backlog
milestone: rimos
depends_on:
- 570
- 465
- 474
- 493
- 424
created: 2026-09-28
updated: 2026-09-29
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

## 2026-09-29

PAUSED: not started; calm-forest's when work resumes. Seam agreed with green-forest (DESIGN §11b, docs/modding/ui.md): kind="screen" windows; title-layer acts new_colony/load/continue/delete_save/open/quit and views saves/starts/settings/loading. The engine delivery is green-forest's; the client side (main.rs, title.rs, play.rs) is calm-forest's. Waits on green-forest's title-layer delivery and leave-to-title (play.rs). Rule from the user: no per-frame cost when idle, a fast first frame, settings only in rare cases (#388 trimmed afc3e3b0).
