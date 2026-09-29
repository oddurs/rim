---
id: 521
uid: c2e8ec82-0ff8-43c8-af5a-28f7be9df504
title: 'CLI hygiene: rim --help, unknown flags are errors, and the harnesses become rim sim subcommands'
type: chore
status: backlog
milestone: workbench
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: client
area: docs
---

## Problem

`rim` has no top-level `--help`, ignores unknown flags, and the headless harnesses (bench, balance, crosscheck, stone_age, year, duel) are cargo examples with hand-rolled flags and no machine-readable output.

## Proposal

`rim --help` lists every subcommand and flag; an unknown flag exits 2 with a suggestion; `rim sim bench|balance|crosscheck|year` wrap the examples with `--json`.

## Acceptance criteria

- [ ] `rim --bogus` exits 2 and names the flag (test)
- [ ] `rim sim balance --seeds 3 --json` prints valid JSON (test)
