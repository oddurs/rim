---
id: 3c8833c4-214c-4da2-ae5e-b02769b71039
title: Equip, wear, haul and drop as right-click orders, with a chosen mark
type: feature
status: backlog
milestone: carrying
depends_on:
- 7691ffc6-817f-4209-a958-eff4e4e3aa3f
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: l
layer: engine
area: ai
api: additive
---

## Problem

A colonist has three slots (hands: one tool; carried: one stack; worn: one per layer), but only the colonist fills them. There is no order to equip, wear, pick up, haul or drop (crates/rim_sim/src/order.rs:62-103 offers attack, hunt, build, haul to a blueprint, harvest, deconstruct, eat and go here). The user wants to right-click the hammerstone to equip it.

## Proposal

The design is in the Carrying and equipping doc: https://claude.ai/code/artifact/349db585-8e04-44a3-a729-f2163bd8d5f4.

- New `order::options` entries, read from def fields and never content ids: Equip (a `tool` def), Wear (an `apparel` def), Haul to storage (any item), and Drop and Take off on the pawn's own slots.
- `Command::Order` re-reads its options, so no new `Command` variant is needed.
- A chosen mark on the pawn, for hands and each worn layer. `tool_for` and `dress` respect it. It's saved; old saves load with nothing chosen. The save changes, so it takes the full cross-platform run.
- Scripts and the UI can read hands, carried, worn and chosen (additive api and ui_api).

It waits on the user's answers to the doc's open questions.

## Acceptance criteria

- [ ] A test: with a colonist selected, right-click a hammerstone, then Equip; the colonist holds it, chosen, and work doesn't swap it out against the user's rule
- [ ] Wear, haul to storage, drop and take off each have a test
- [ ] Tick time at 250² with 50 colonists is unchanged on the scaling bench, and the menu builds in under 0.1 ms

## 2026-09-29

The user settled the doc's questions on 2026-09-29. A chosen tool and work that needs another: swap and return. Drafting still drops the carried stack. No pack in this item; packs are 571dca56-98bc-4eed-a067-ed2dab075441, later, in crafting. Weight is 7691ffc6-817f-4209-a958-eff4e4e3aa3f, which comes first.
