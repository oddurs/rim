---
id: 449
uid: 18e3e69f-c098-4ad1-8af9-0883f2181855
title: 'Balance across seeds: distributions over 200 seeds, and a check a balance PR must pass'
type: feature
status: backlog
milestone: proving-ground
depends_on:
- 476
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: tooling
area: tests
---

## Problem

Balance changes are argued from one seed, or from 200 seeds run by hand (#200, #206).

## Proposal

The balance bots write `--json` with p10, median and p90 of their measures and survival rates; a nightly run records them; a balance PR attaches the before and after, and a survival rate that drops beyond chance (two-proportion test, p < 0.01) is called out.

## Acceptance criteria

- [ ] A nightly run records the distributions (linked)
- [ ] A deliberately harsher change is flagged by the check (shown)
