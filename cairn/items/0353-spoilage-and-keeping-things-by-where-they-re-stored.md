---
id: 353
uid: 63811969-055f-4aed-a5e9-a2584a3a6c5d
title: Spoilage, and keeping things by where they're stored
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 322
- 186
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p3
api: additive
effort: m
layer: engine
area: needs
---

## Why

Storage keeps things (DESIGN.md §4f), but nothing spoils or gets wet yet, so `shelter` and a pot's keeping do nothing. Waits for its first readers: ground wetness (0187) and cooking.

## What

- Food with a spoil rate; a store's `keeps` factor (terms, so a mod can make cold storage from `temperature`); `shelter` keeps contents dry.

## Acceptance criteria

- [x] Berries in a pot outlast berries on the ground (test)

## 2026-09-27

Built as §4f's Spoiling and keeping:
- Items: spoil = { days, rate } on thing defs. rate is terms read at the stack's cell, and no terms is 1.
- Stores: store.keeps is terms read at the container's cell, dividing the rate (floor 0.1). A shelter store evaluates the item's rate with sky = 0, through Fields::eval_sheltered. So rain terms written against input = sky stop at a roof or a lid, and the engine names no field.
- Condition: hp is the state, so filters, tokens and saves need nothing new. World::set_stack_hp takes a stack out of the ledger and store index at the old hp and puts it back at the new; at 0 it goes through take_from_stack. The part of a point lost below one hp is a Spoiling { lost } component, saved as an optional engine:spoiling section (no format bump) and in the state hash.
- Pass: systems::spoil runs every 250 ticks and works out a quarter of the stacks (entity id % 4), stacks only (loose or contained). Changes apply in id order.
Content:
- berries 6 days, raw meat 2.5, each rate = 1 + warmth (0° −0.8, 25° +1) + rain (up to +2, times sky);
- primitive's storage pot keeps 2.5 and timber's granary 3; the basket keeps 1 and doesn't shelter.
Tests (tests/spoilage.rs):
- potted berries outlast loose ones, which rot away, and the ledger's count drops by 20;
- rain spoils loose and open-box berries but not a shelter box's;
- a save keeps hp and the remainder, with an equal hash;
- cost: 4000 berry stacks, 1.4 ms a pass (0.0056 ms a tick), measured at load average 72 on a busy machine.
Needs reconciling: Fields::eval_at here is the same function as the plants branch's (e1be8ebd); keep one on rebase. Built on stock fields (d77d9e1f, #231) for input = sky.
Open: cooking's meals (#236) should get spoil data when both land. A fresh and a spoiled stack merging keep the target's hp, as merges did before.
