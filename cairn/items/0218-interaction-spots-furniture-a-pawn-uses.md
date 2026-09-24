---
id: 419a24ab-276f-417d-a2ff-8de0a67b62ed
title: 'Interaction spots: furniture a pawn uses'
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
priority: p1
api: additive
effort: m
layer: engine
area: ai
---

## Why

A bed is already furniture a pawn goes to and is better off for using.
`BedDef { rest_rate }` is that idea hard-coded for one case. A chair at a
table is the same idea, and so is every workstation later.

## What

- `[[thing]]` gains `spots = [...]`: cells, relative to the thing, a pawn
  stands or sits in to use it.
- A spot is reserved while occupied, so two pawns do not share a chair.
- A job can name the furniture it wants; `BedDef` becomes the first user of
  the general thing rather than its own mechanism.
- Chairs face the table they serve: a spot may require an adjacent thing by
  category, which is how a chair knows it is at a table and not alone in a
  field.

## Acceptance criteria

- [x] Sleeping uses spots, with no behaviour change
- [x] Two pawns cannot use one spot
- [x] A pawn eats at a table when one is reachable, and on the spot when not

## 2026-09-24

Done. ThingDef gains spots (cells relative to the thing a pawn occupies to use it) and tags (free strings); a spot may require a tagged thing beside it, which is how a chair is a seat at a table and a stool in a field otherwise. nearest_spot is the one finder: free (per-thing reservation), reachable, beside-satisfied, nearest. find_bed is its first caller and Job::Sleep's existing spot field is now the destination in both cases, so the bed is bit-for-bit what it was (pos and rate asserted). find_food is the second: it books a seat if one is free, and run_eat gained a stage -- fetch one portion into carry, walk to the seat, eat there; the chair vanishing mid-walk means eating standing up, and end_job's carry-drop already puts an abandoned portion back on the ground. Reservation is per thing, not per spot: a two-seat bench would need a (thing, spot) key. The Vec is there so the schema does not change when that arrives. Core's only content change is bed declaring its spot; table and chair are 0219 and were tested here from a throwaway mod.

## 2026-09-24

Flake found and fixed before merge: the two table tests in spots.rs both built their throwaway mod in rim-seats-<pid>, and since one cargo test binary is one process they shared the directory and raced each other's remove_dir_all. Caught because the workspace's 'test result' line count dropped from 22 to 14 -- a failing binary stops cargo test before the other crates run. Each test now names its own directory; the binary passed six runs in a row after. Lesson recorded: any file with two probe-mod tests needs per-test directories.
