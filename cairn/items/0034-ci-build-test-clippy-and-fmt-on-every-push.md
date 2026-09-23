---
id: 34
title: 'CI: build, test, clippy and fmt on every push'
type: chore
status: review
milestone: foundations
assignee: Oddur Sigurdsson
claimed: 2026-09-22
depends_on:
- 32
created: 2026-09-22
updated: 2026-09-22
priority: p2
api: none
effort: s
layer: tooling
area: tests
---

## Why

Keep the engine honest automatically.

## Acceptance criteria

- [ ] Workflow runs on macOS and Linux
- [x] Determinism test is part of it

## 2026-09-22

Added .github/workflows/ci.yml: fmt; clippy -D warnings + tests + 3-day headless soak on ubuntu and macOS; cairn check + ROADMAP.md freshness (cairn pinned 0.2.1). Added rustfmt.toml (width 120) and fixed all clippy warnings. Every step passes locally. Not yet run on GitHub: the repo has no remote. Close once the first run is green.
