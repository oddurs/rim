---
id: 473
uid: 4796c539-2458-428b-9918-a7fa0bce5b6b
title: The depth ruler misses an alert whose first subject is on another level
type: bug
status: done
milestone: depth
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: additive
effort: s
layer: core
area: ui
---

## Why

Main went red after #261: the view's autotest hurts a colonist at −1 and expects the ruler to show an alert there, but core:hurt names only its first hurt colonist as its subject, and a surface colonist was hurt too. The ruler counted the alert on the surface alone, which is also what a player would see: trouble below goes unmarked.

## What

- An alert may name every subject it is about (`subjects`); core's hurt and idle alerts do.
- The ruler counts an alert on each level one of its subjects is on.

## Acceptance criteria

- [x] The autotest's ruler check holds with a colonist hurt on the surface as well

## 2026-09-27

Main went red again on both ruler-alert checks after later merges. The alerts panel re-checks at most every 0.25 s of client time, and the section waited 12 frames (0.2 s) after hurting colonists, so whether it saw the new alerts depended on where earlier sections left that phase. It waits 30 frames now (0.5 s), past one period.
