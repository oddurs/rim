---
id: ca22f222-1b4c-4705-b106-afbb8a282b43
title: 'Bills pick their ingredients: one material per order, and a filter'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 01691032-1f99-475a-b964-6e3f4149fe22
created: 2026-09-25
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: additive
effort: m
layer: plugin
area: building
---

## Why

A recipe's tag input (`knappable`) takes whatever lies nearest, and a work order doesn't care whether its pieces match. A hand axe can be knapped from one flint and one bone, and it comes out made of whichever was brought first. So a flint axe can cost half its flint, or a bone axe can use up a flint. The player can't steer it: bills have no ingredient filter, and bone dropped at a kill near the spot beats flint in a stockpile.

## What

- **An order that takes a tag input by several** takes one material for all of them. Once the first piece is in, the rest must match.
- **A bill's ingredient filter** says which things may fill a tag input (flint only, or anything knappable), with the bills panel showing it.
- **The stall reason names the shortage**: "1 of 2 flint" rather than counting every knappable.

## Acceptance criteria

- [x] A hand axe never mixes flint and bone
- [x] A bill can be told to use flint only
- [x] A bill waiting for a matching second piece says so

## 2026-09-26

Uses the shared filter from 01691032 (DESIGN.md §4f) for its ingredient filter, so it waits for that item.

## 2026-09-27

Engine versus plugin. The engine holds the mechanism. Need gains an optional Filter (the shared one from §4f), and Need::fits checks the thing or tag, the filter, and the one-material lock (same def and made_of as the first piece in) on each stack. The order fetch goes through a new ai::nearest_stack_where, which also asks each stack about its material and condition; nearest_item_where is that with no stack check. The first piece is fetched only from a def with stock.on_map at or above what's missing, so the lock can't strand an order on a lone nearest flint. The lock applies to thing needs too (it only matters where made_of differs). rim.order's needs report match (the def the rest must be) and coming (a colonist is on the way with some), so the plugin's stall reason doesn't flicker while the last piece is carried. The work board's Why names the matched thing: 'No flint to bring'. The plugin holds the policy. A bill has filters[input] = { allows = {...} }; all allowed is stored as no filter, so a thing a mod adds later is taken. The last allowed thing can't be turned off. Changing a filter takes the bill's order down, like pausing. Stall reasons count the allowed thing with the most, not the tag's total: '1 of 2 flint'. An order up that waits for a match says '1 of 2 flint'. The panel has a toggle per thing for each tag input with more than one thing. Tests: crafting.rs has the kit chip/bit tests (one material; told which thing, including the refused last toggle; waiting says so) plus the real primitive hand axe with 1 flint and 3 bone; rim_ui bills.rs covers the toggles and the command they send. Left open: the panel edits only the allows set. The filter's refuses (materials) and hp range work in the engine and in post_order, but no bill UI sets them yet.

## 2026-09-27

Changed: one material is opt-in per need (Need.alike, alike = true in post_order), not the engine's rule for every need. The engine's work_orders test mixes chip and slab by tag on purpose, and cooking will want a stew of any meat. The crafting plugin asks for alike on every input unless the recipe input says mix = true; a mixed input's stall reason counts the whole tag. Engine callers that don't say alike behave as before.
