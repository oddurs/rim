---
id: 607
uid: 43a3b850-7840-4857-99b4-871c7445df82
title: A def can name its plural, so counted labels read right
type: feature
status: backlog
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: additive
effort: s
layer: engine
area: modding
---

## Problem

The overlays count things in the pointer hint: 'Chop · 4 oak trees', '16 of 18 walls', 'Hunt · 2 deers'. overlay::plural builds the plural with English suffix rules (s, es, ies), so an irregular label reads wrong: deer becomes 'deers'. It's a known gap from Chalkline's designate (#314), where the PR listed it for review.

## Proposal

Thing and creature defs take an optional `plural` (`plural = "deer"`). overlay::plural uses it when set and falls back to the suffix rules. It's an additive change to the defs schema, documented in the modding docs beside `label`.

## Acceptance criteria

- [ ] A def with `plural` set is counted with it, and one without keeps the suffix rules (unit test)
- [ ] core's deer sets `plural = "deer"`, and the hunt hint over two deer reads 'Hunt · 2 deer' (autotest or unit test)
- [ ] The modding docs list `plural` with the other def fields
