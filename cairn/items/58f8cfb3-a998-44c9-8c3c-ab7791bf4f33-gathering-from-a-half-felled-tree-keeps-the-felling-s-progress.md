---
id: 58f8cfb3-a998-44c9-8c3c-ab7791bf4f33
title: Gathering from a half-felled tree keeps the felling's progress
type: bug
status: done
milestone: building
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: none
effort: s
layer: engine
area: sim
pillar:
- plugin-first
---

## What happens

`Work` holds progress for one designation. The primitive mod gives the oak a gather harvest beside core's chop. Gather branches from a tree someone has half chopped, and the chop's count is replaced by the gather's; the next chop starts over (a14a5ff6 noted it).

## Fix

`Work` keeps the displaced count (`other`): starting a different designation's work shelves the current one, and coming back to it restores it. One shelf: a thing has at most a few harvests, and two in turn is the case that happens. Saved additively, remapped on load like `designation`.

## Acceptance criteria

- [x] A gather between two stretches of chopping leaves the chop's progress where it was
- [x] The shelved count survives a save
