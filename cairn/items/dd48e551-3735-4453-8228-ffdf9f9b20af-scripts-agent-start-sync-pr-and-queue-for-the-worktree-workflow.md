---
id: dd48e551-3735-4453-8228-ffdf9f9b20af
title: 'scripts/agent: start, sync, pr and queue for the worktree workflow'
type: chore
status: backlog
milestone: proving-ground
depends_on:
- 10e53a78-6c57-47b5-914f-551cc0422a34
- 9b435cd8-40ed-4c0f-8a41-ef3b892811c2
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Problem

Agents start worktrees, name branches, rebase, open PRs and hand them to the queue by hand, each a little differently. The steps are the same every time.

## Proposal

`scripts/agent` with verbs `doctor start check commit pr sync done list`, plus `queue`:
- `start <type>/<id8>-<slug>` makes the worktree from a fresh `origin/main` under `../.worktrees/rim/`.
- `sync` fetches and rebases, re-renders the roadmap when that's the only conflict, and says when code conflicted.
- `pr` runs `scripts/task check`, pushes and opens the PR.
- `queue` labels the PR `queue`, and `docs-only` when the diff touches only `cairn/` and Markdown.
- `done` removes the worktree and branch after the merge.

## Acceptance criteria

- [ ] A full round trip on a throwaway item: start, commit, pr, queue, merged by Mergify, done (linked)
- [ ] `sync` on a branch whose only conflict is ROADMAP.md resolves it by rendering (shown)
- [ ] `queue` sets `docs-only` only for a cairn-and-Markdown diff (both cases shown)
