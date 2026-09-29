---
id: 507
uid: 9a743404-27ad-4e80-ba29-2e8c0d10e1c2
title: Migrate rim to cairn format 5, bumping CI's pin in the same commit
type: chore
status: doing
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-29
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Problem

rim's items are cairn format 4, and CI pins cairn at rev c7d4230. A cairn 1.0.0-alpha.1 dev build (format 5) installed globally on 2026-09-27 refused to write in rim until rim-c2 reinstalled the pinned rev. The migration has to happen once, deliberately, with the pin moving in the same commit.

## Proposal

When the merge queue is empty: `cairn migrate` on a fresh main, bump the rev in `ci.yml` and in `scripts/task` in the same commit, and tell every agent (through the queue owner) to reinstall.

## Acceptance criteria

- [x] One PR: the migrated items, the new pin in CI and in scripts/task, and ROADMAP.md rendered by the new version
- [ ] `cairn check --render --strict` passes locally and in CI on the new version
- [ ] Every open PR's owner is told to rebase and reinstall, and the queue owner confirms

## 2026-09-29

Migrated with 1.0.0-alpha.1 (68ac154) on a frozen main: 722 items, 221 numbers restored, 553 references renumbered. ROADMAP.md is untracked now, so criterion 1's render is the local one and CI's artifact. cairn migrate's preflight reads its own cairn/items/.lock as an uncommitted change, so it needed --allow-dirty; .gitignore now ignores the lock. Branches cut before this carry their item edits over with scripts/cairn-port.
