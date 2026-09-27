---
id: 7fb64c0e-186b-4253-9961-e6ddf6bda31a
title: 'ROADMAP.md stops conflicting: render it on main, not in every PR'
type: chore
status: backlog
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: s
layer: tooling
area: tests
---

## Problem

Almost every PR runs `cairn render`, and CI's `cairn check --render --strict` fails a PR whose ROADMAP.md is stale. So every merge makes every other open PR conflict on ROADMAP.md, and it's the main source of rebase churn (rim-c2, 2026-09-27). A merge queue would hit the same conflicts and eject PRs for a generated file.

## Proposal

- PRs stop committing ROADMAP.md changes; CI checks the items (`cairn check --strict`) and that they render, without comparing the file.
- A workflow on push to main (after the queue merges) renders ROADMAP.md and commits it only when it changed, as a bot allowed by the ruleset, batching rapid merges with `concurrency`.
- `scripts/task roadmap` and the pre-push hook check the items, not the rendered file; CLAUDE.md and the cairn block say not to commit ROADMAP.md.
- Alternative if a bot commit on main is unwanted: stop tracking ROADMAP.md and publish it from CI. Decide in this item.

## Acceptance criteria

- [ ] Two PRs that each add a cairn item merge one after the other with no conflict and no rebase (linked)
- [ ] After the second merge, main's ROADMAP.md lists both items (commit linked)
- [ ] A PR that edits ROADMAP.md by hand is told to stop by CI
