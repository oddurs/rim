---
id: 6f7f4f63-a333-4dbc-8ad9-d258bea603fe
title: pre-push accepts a commit stacked on a branch already pushed, judging only its own diff
type: chore
status: done
milestone: proving-ground
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
effort: s
layer: tooling
area: tests
---

## Problem

scripts/hooks/pre-push judges a pushed commit by its diff from the merge base with origin/main. A commit stacked on another unmerged branch carries that branch's diff too. So a docs-only commit on top of a code branch is refused unless the whole gate has run on it, and a clean rebase of a stacked branch never matches its recorded patch-id once the lower branch merges under another sha. Both happened on #287 (2026-09-28).

## Proposal

- Diff from the nearest ancestor already on the remote (`git merge-base` against the remote-tracking refs, or the parent when `git branch -r --contains` finds it), not from origin/main.
- Keep origin/main as the fallback when nothing closer is on the remote.

## Acceptance criteria

- [x] A docs-only commit pushed on top of an already-pushed code branch runs only `scripts/task roadmap`
- [x] A code commit on the same stack is still refused without a recorded pass
- [x] A branch rebased after its lower branch merged under a new sha is accepted by patch-id

## 2026-09-28

The hook now judges the commits the remote doesn't have, from the newest commit below them that it does. A rebase is accepted commit by commit, by patch id with ROADMAP.md aside: every pushed commit matches one the check covered, and every covered one is pushed or on main, so a dropped commit needs the check again. scripts/test-pre-push builds throwaway repos with a bare remote and covers seven cases; against the old hook the stacked docs commit and the stacked rebase fail, and the other five pass. scripts/task gains a 'scripts' verb for it, run by check. CI doesn't run it yet: the lanes PR (#287) rewrites the checks job, so the step goes in after it lands.

## 2026-09-28

An independent review found no high issues and six real ones, all fixed and tested: patch ids now verbatim (an amend changing whitespace inside a string had matched); the docs-only diff uses --no-renames (a guide moved out of docs/modding/ had passed as Markdown); an upper branch rebased onto its updated, pushed lower branch is accepted (covered commits the remote already has no longer need matching); a root commit diffs against the empty tree instead of killing the hook silently; with no origin/main the rebase path is skipped; the origin/main patch ids are computed only when a checked commit is missing from the push. scripts/test-pre-push now has 13 cases; the old hook fails 5. Known and left: a docs branch stacked on pushed code that merges origin/main in is refused (the old hook too); a checked commit dropped from a rebase goes unchecked if the remote already has it, and CI catches that.
