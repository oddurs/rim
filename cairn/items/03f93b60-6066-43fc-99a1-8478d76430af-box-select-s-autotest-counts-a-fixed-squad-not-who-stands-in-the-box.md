---
id: 03f93b60-6066-43fc-99a1-8478d76430af
title: Box select's autotest counts a fixed squad, not who stands in the box
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

Main's CI on 1577cd4e fails one autotest check from #288: "a select drag's box previews who it picks (4 rings, '5 × 5 · 4 colonists')". The check expects exactly the three colonists it set up (the founder and two it spawns), but since #290 refills the colony after the weather section, another colonist can be standing inside the 5×5 box.

## Proposal

Count whoever `boxed_colonists` finds in the box, and require the three set up to be among them.

## Acceptance criteria

- [x] The check compares the preview's rings and chip with the colonists actually in the box, and still fails if any of the three set up is missing
- [x] The autotest passes on current main
