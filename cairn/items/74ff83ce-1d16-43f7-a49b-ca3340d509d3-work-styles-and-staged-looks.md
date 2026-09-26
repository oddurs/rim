---
id: 74ff83ce-1d16-43f7-a49b-ca3340d509d3
title: Work styles and staged looks
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
depends_on:
- a14a5ff6-adad-488c-9810-8a02f6409b7c
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: additive
effort: m
layer: client
area: render
pillar:
- plugin-first
---

## Why

Work on a cell should read at a glance: a wall rising, a rock cracking, a tree leaning (DESIGN.md §6b). The look is data, so how a job shows must be too.

## What

- `[[work_style]]` defs: `id`, `every`, `strike`, `wear`, `exit`, from fixed sets (strike: shake chips dust shed; wear: grow cracks lean; exit: fall crumble pop). A designation names a style; a harvest or build may override it. Core ships chopping, mining, building and dismantling.
- `grow = [from, to]` on look layers: clipped from the bottom inside the window, absent before, whole after. Unset, layer i of n takes [i/n, (i+1)/n].
- Unfinished things draw see-through with a hatch; a thing looks solid when it blocks.
- `cracks` and `lean` oriented by `Work.side`; damage uses the thing's take-down wear.
- Rule 3 at load: a blocking thing may not be taken down with `grow`.
- docs/modding/looks.md documents all of it.

## Acceptance criteria

- [x] `[[work_style]]` loads, validates and patches like other defs
- [x] A wall rises through its grow windows and turns solid when built
- [x] Mining cracks from the worked side; a damaged wall cracks by hp
- [x] A blocking thing with a `grow` take-down is a load error naming the def
- [x] looks.md documents styles and `grow`
