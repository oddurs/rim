---
id: 464
uid: 3d4b8c40-ead0-4c5f-b5c5-2a7b56845491
title: 'Protect main: a ruleset that requires the queue''s checks, kept in the repo'
type: chore
status: backlog
milestone: proving-ground
depends_on:
- 446
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Problem

`main` has no branch protection and no rulesets, so a push that skips the queue lands anyway. Rulesets are free on a public repo. DESIGN.md §8a.

## Proposal

- `.github/rulesets/main.json`: no direct pushes or force pushes to `main`, pull requests required, the queue lane's checks required, Mergify allowed to merge.
- `scripts/task ruleset` applies it with `gh api` so the settings live in the repo and are reviewed like code.

## Acceptance criteria

- [ ] A direct `git push` to `main` is refused (shown)
- [ ] A PR merges through Mergify with the ruleset on (linked)
- [ ] `scripts/task ruleset` applied twice is a no-op the second time
