---
id: 596
uid: 346145f1-60cd-4ed9-a53c-d711ec8afbdf
title: The filters test predates mining's rock blocks, and main is red
type: bug
status: done
milestone: crafting
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p0
api: none
---

## What happens

`filters::every_item_lands_in_the_categories_it_should` fails on main after mining (#298). Its list of what the materials category holds directly doesn't include the new chalk and limestone blocks. My checks on #298 ran mining, strata, harvests and the guards, but not filters.

## What should happen

The blocks are structural stuff, as stone is, so landing in materials is intended. The test's expected list gains them.

## Acceptance criteria

- [x] The filters test passes with the rock blocks in materials
- [x] All of rim_sim's tests pass

## 2026-09-28

The expected materials list gains core:chalk_blocks and core:limestone_blocks. They're structural stuff, as stone is, so landing in materials is intended. All of rim_sim on 11e7c373 plus this commit: 593 of 593, with nextest --no-fail-fast.
