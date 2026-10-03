---
id: 724
uid: a6bb6ddb-c61d-4c64-8008-c89244ed73fa
title: A budgets.toml change runs the render bench in its PR
type: chore
status: done
milestone: bare-metal
created: 2026-10-02
updated: 2026-10-02
closed_at: 2026-10-02
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Problem

scripts/ci-plan ran the render bench in a PR only for crates/rim_client/ or Cargo.lock, so a PR changing only budgets.toml merged without its new caps checked; the next main push was the first to apply them (#447, found by rim-c2).

## Acceptance criteria

- [x] A PR that changes budgets.toml runs the render bench, covered by a case in scripts/test-ci-plan
