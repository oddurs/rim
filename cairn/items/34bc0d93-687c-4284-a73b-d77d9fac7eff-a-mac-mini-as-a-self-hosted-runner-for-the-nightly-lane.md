---
id: 34bc0d93-687c-4284-a73b-d77d9fac7eff
title: A Mac mini as a self-hosted runner for the nightly lane
type: chore
status: backlog
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: none
effort: s
layer: tooling
area: tests
---

## Problem

Nightly work (200 seeds, fuzzing, the full macOS suite) competes with PRs for GitHub's 20 concurrent jobs. The owner's Mac mini could run it with a warm build cache. Deferred by the owner on 2026-09-27.

## Proposal

A self-hosted runner used only by `schedule` (which runs main's code, never a PR's, so it's safe on a public repo), several runner instances with their own work directories and a shared sccache, and a label switch back to GitHub-hosted runners when it's offline.

## Acceptance criteria

- [ ] A nightly run completes on the mini (linked), and with the mini offline the switch sends it to GitHub-hosted runners
