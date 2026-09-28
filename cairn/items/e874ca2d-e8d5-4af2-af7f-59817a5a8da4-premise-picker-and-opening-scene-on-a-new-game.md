---
id: e874ca2d-e8d5-4af2-af7f-59817a5a8da4
title: Premise picker and opening scene on a new game
type: feature
status: backlog
milestone: story
depends_on:
- 96db8898-cf55-4517-acd7-260c05fe9d08
- bea51754-c7d0-4353-b763-1a6a11e6f383
created: 2026-09-26
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: client
area: ui
pillar:
- growth
---

## Acceptance criteria

- [ ] New game lists premises by mod, with the cast and a summary
- [ ] Threads that bind to another premise show as skipped, with the reason
- [ ] Claim conflicts are shown and resolved here
- [ ] The opening plays as a short scene before tick 0 and can be skipped

## 2026-09-27

The New colony screen is built by 73751f4f (the default game is a set), which leaves a Premise slot showing the castaway. This item fills that slot rather than building its own screen.
