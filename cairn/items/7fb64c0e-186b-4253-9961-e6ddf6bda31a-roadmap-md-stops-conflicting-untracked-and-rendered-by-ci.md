---
id: 7fb64c0e-186b-4253-9961-e6ddf6bda31a
title: 'ROADMAP.md stops conflicting: untracked, and rendered by CI'
type: chore
status: review
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-28
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
- [ ] The `roadmap` artifact from main's run after the second merge lists both items (run linked)
- [x] ROADMAP.md is untracked and ignored, and `scripts/task roadmap` passes without it

## 2026-09-27

Decision: keep ROADMAP.md tracked on main, rendered by a workflow on push to main as github-actions[bot]. It stays readable on GitHub, which is where people look. Pieces: drop the after-* render hooks from cairn.toml and the render in post-merge (else every cairn command dirties ROADMAP.md and agents commit it); CI 'checks' runs cairn check --strict plus 'cairn render -o /dev/null' and fails a PR whose three-dot diff touches ROADMAP.md; scripts/task roadmap does the same. Main has no ruleset today (checked 2026-09-27), so the bot push works now; the ruleset item must list GitHub Actions as a bypass actor. A GITHUB_TOKEN push starts no workflow, so the render commit costs no CI. Built after the lanes PR lands, because both rewrite ci.yml.

## 2026-09-28

Built on the lanes branch (#287), which rewrites ci.yml. scripts/task roadmap: cairn check --strict, a render to stdout, and a refusal when the branch's own commits change ROADMAP.md (base HEAD^1 on a PR's merge commit in CI, the merge base with origin/main locally), with the exact git checkout to drop it. cairn.toml loses its after-* render hooks and post-merge stops rendering, so cairn commands no longer dirty ROADMAP.md. .github/workflows/roadmap.yml renders on pushes to main that touch cairn/ or cairn.toml, as github-actions[bot], retrying on top when a merge lands between its pull and push. Every open PR that commits ROADMAP.md is refused once this lands until it drops the change: one git checkout. The criteria need real merges and are ticked after it lands.

## 2026-09-28

The user chose the alternative (via rim-c2, 2026-09-28): ROADMAP.md is untracked, not rendered by a bot on main, because main advances only through a merged PR. So: /ROADMAP.md in .gitignore and removed from the index; its merge=cairn attribute gone; the roadmap workflow dropped; the checks job renders main's and uploads it as the 'roadmap' artifact (90 days); scripts/task roadmap is cairn check --strict plus a render to stdout; README, CLAUDE.md and DESIGN §8a say so. The refuse-a-ROADMAP-change check went too: an untracked file can't be changed in a branch.
