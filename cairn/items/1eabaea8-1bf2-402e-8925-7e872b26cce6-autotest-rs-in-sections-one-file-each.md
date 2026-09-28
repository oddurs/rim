---
id: 1eabaea8-1bf2-402e-8925-7e872b26cce6
title: autotest.rs in sections, one file each
type: chore
status: backlog
milestone: bare-metal
assignee: green-forest
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: none
effort: m
layer: client
area: tests
---

## Why

autotest.rs is 4,030 lines. Every client PR touches it, which makes it the top source of rebase conflicts, and a slow file to read and build.

## What

Split it into a module per section (lighting, UI, tools, depth and so on), sharing one harness. Delete checks that duplicate a unit test.

## Acceptance criteria

- [ ] No autotest file is over 800 lines
- [ ] The same checks pass, in the same count or fewer with each deletion noted
