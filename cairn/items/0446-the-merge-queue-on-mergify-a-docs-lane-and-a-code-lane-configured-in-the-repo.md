---
id: 446
uid: 10e53a78-6c57-47b5-914f-551cc0422a34
title: 'The merge queue on Mergify: a docs lane and a code lane, configured in the repo'
type: chore
status: backlog
milestone: proving-ground
depends_on:
- 467
- 496
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: tooling
area: tests
---

## Problem

The queue is a Claude session (rim-c2) and a script, run by messages: "ready #N sha", then main moves, then a rebase and a fresh CI run, again. #213, a docs-only PR, went round four times. A real queue tests each PR on top of main and the PRs ahead of it and merges in order by itself. DESIGN.md §8a.

## Proposal

- `.mergify.yml`: a `docs` queue (label `queue` and `docs-only`; merge on the checks job; batches of 5) and a `code` queue (label `queue`; merge on the queue lane's jobs including `agree`; batches of 3, `checks_timeout` 40 minutes); squash merges; a `pull_request_rules` entry that queues on the `queue` label and skips drafts.
- CI's queue lane triggers on Mergify's queue branches.
- The labels `queue` and `docs-only` exist; `docs-only` is set only when the diff touches only `cairn/` and Markdown.
- A dry run on throwaway PRs before any real PR uses it; then rim-c2's script is retired with rim-c2's agreement, and CLAUDE.md and the agents' memory describe the new way.
- Needs the repo owner to install the Mergify GitHub App on `oddurs/rim`.

## Acceptance criteria

- [ ] A throwaway code PR labelled `queue` is tested on top of main and merges without a manual rebase (linked)
- [ ] A throwaway PR with a failing check leaves the queue with a comment naming the check (linked)
- [ ] Two docs-only PRs labelled together merge in one batch through the docs lane (linked)
- [ ] rim-c2 agrees and stops running its script; CLAUDE.md says how to queue a PR

## 2026-09-27

From rim-c2, who ran the scripted queue: the script merged only when (1) the PR's head was the sha readied, (2) origin/main was an ancestor of it, (3) every check on the sha was complete and green, at least 5 of them, (4) the whole workflow run for the sha concluded success, since 'Every platform reaches the same state' appears only after the test jobs and the rollup can read green too early, and (5) GitHub reported it mergeable. It halted if main's latest run failed. Keep that: merge_conditions list every required job by name, agree included, and a ruleset requires them, so testing on the exact merged result is enforced, not advisory. Two mergers would race: rim-c2 stops its script only after the dry run works.

## 2026-09-27

Evidence for a queue that tests the merged result: #262 and #270 each passed CI alone, then merged in one burst with #273's no_clocks guard, and main went red on the combination (2026-09-27). A queue that runs the suite on main+PR, one at a time or speculatively, catches that before merge.
