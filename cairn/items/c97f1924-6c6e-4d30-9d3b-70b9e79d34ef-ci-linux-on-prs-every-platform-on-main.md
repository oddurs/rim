---
id: c97f1924-6c6e-4d30-9d3b-70b9e79d34ef
title: 'CI: Linux on PRs, every platform on main'
type: chore
status: review
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Why

The repository is private, so GitHub Actions bills minutes past the free quota. Every PR run tested Linux, macOS (billed at 10×), Windows (2×) and ARM Linux. With several agents opening PRs, a day's runs used up the month's quota, and every job has since been refused.

## What

- PRs run the Linux jobs only: checks, tests, sim runs and the client.
- macOS, Windows and ARM, and the job that compares platforms, run on pushes to main.
- A `full-ci` label runs the whole matrix on a PR that needs it (a determinism-sensitive sim change).
- Draft PRs run nothing.
- A PR that only changes cairn items or Markdown runs the quick checks alone.

## Acceptance criteria

- [ ] A PR's run has no macOS, Windows or ARM job unless it carries `full-ci`
- [ ] A push to main runs every platform and compares them
- [ ] A cairn- or docs-only PR runs only the checks job

## 2026-09-26

The criteria are ticked on the workflow's logic (actionlint clean; the change detection exercised locally under bash). They can only be watched passing once billing lets jobs start.

## 2026-09-26

Unticked: the criteria are observable only once billing lets jobs start; tick them from the first runs.
