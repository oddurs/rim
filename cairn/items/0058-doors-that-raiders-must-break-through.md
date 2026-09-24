---
id: 531096ee-8681-4d09-804d-fc11899c2eec
title: Doors that raiders must break through
type: feature
status: done
milestone: shelter
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
priority: p1
api: none
effort: m
layer: engine
area: building
pillar:
- survival
---

## Why

Walls mean nothing if raiders walk through doors.

## Acceptance criteria

- [x] Doors are faction-owned; hostiles attack them

## 2026-09-23

Doors carry an Owner(Faction), set when the colony finishes building one. Passability is per faction: Map keeps one region layer per faction (Faction::ALL), differing only where an owned door stands, so can_reach_for stays O(1) and A* skips doors it cannot open. A hostile that cannot path to a colonist looks for the door whose neighbours touch both its own region and the target's -- that is exactly the door worth breaking -- and takes Job::Breach until it falls. Found and fixed a latent bug while testing: set_fixture only marked regions dirty when *blocking* changed, so removing a door left the region layers stale; with faction-aware doors a door is itself a barrier, so a door change now dirties regions too. Perf note for 0097 (incremental region updates): ensure_regions now runs one flood fill per faction instead of one, and it only runs when walls or doors change.
