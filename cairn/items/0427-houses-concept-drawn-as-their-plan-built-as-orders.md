---
id: 427
uid: eb469f44-ba7b-4319-8968-e99fa35c21bb
title: 'Houses concept: drawn as their plan, built as orders'
type: spike
status: done
milestone: houses
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: none
effort: m
layer: tooling
area: docs
---

## Question

How should buildings look and work, so that a house shows all its mechanics with abstract, minimal graphics and no sprites, and how do crafting and building fit together?

## Options

Weighed in DESIGN.md §6c: a picture or a plan; per-cell edges, hand-drawn tile sets or quarter joins; explicit facing or derived; a room size cap or a roof span; roles in scripts or data; two making mechanisms or one.

## Decision

DESIGN.md §6c. A house is drawn as its plan (four line weights, material as pattern, one light, three zoom stories), walls join in quarters from the 8-neighbour mask, openings orient from their run and face the room, a roof span replaces the 400-cell cap, rooms take roles from data, builds become orders with material tool gates and replace-in-place, and plans are text. The prototype is docs/engineering/houses-prototype.html. The implementation is the rest of the houses milestone.

## Acceptance criteria

- [x] Decision recorded in DESIGN.md
- [x] A working prototype of the look and the room rules
- [x] The milestone's items filed with dependencies
