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

## 2026-09-27

Decision: keep ROADMAP.md tracked on main, rendered by a workflow on push to main as github-actions[bot]. It stays readable on GitHub, which is where people look. Pieces: drop the after-* render hooks from cairn.toml and the render in post-merge (else every cairn command dirties ROADMAP.md and agents commit it); CI 'checks' runs cairn check --strict plus 'cairn render -o /dev/null' and fails a PR whose three-dot diff touches ROADMAP.md; scripts/task roadmap does the same. Main has no ruleset today (checked 2026-09-27), so the bot push works now; the ruleset item must list GitHub Actions as a bypass actor. A GITHUB_TOKEN push starts no workflow, so the render commit costs no CI. Built after the lanes PR lands, because both rewrite ci.yml.
