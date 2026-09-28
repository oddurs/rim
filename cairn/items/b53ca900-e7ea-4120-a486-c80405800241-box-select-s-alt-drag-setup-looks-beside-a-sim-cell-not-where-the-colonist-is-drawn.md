---
id: b53ca900-e7ea-4120-a486-c80405800241
title: Box select's Alt-drag setup looks beside a sim cell, not where the colonist is drawn
type: bug
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: s
layer: client
area: tests
---

## Problem

The autotest's several-selected section (#288) can panic with "a cell beside one colonist and no other". To set up the Alt-drag, it takes the third colonist's sim cell and looks for a neighbouring cell where a two-cell box holds only them. But a box goes by where a colonist is drawn, which differs from the sim cell while they're mid-step. Since #290 refills the colony, others can be standing beside them too.

## Proposal

Search every boxed colonist and each of their four neighbouring cells, going by drawn cells, and take the first pair where the two-cell box holds that colonist alone.

## Acceptance criteria

- [x] The setup finds its colonist and cell from drawn positions over every boxed colonist
- [x] The autotest passes on current main
