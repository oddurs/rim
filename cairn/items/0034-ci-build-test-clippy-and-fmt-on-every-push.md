---
id: b1fabfa8-7a11-47cc-b8da-295d3d0b60b6
title: 'CI: build, test, clippy and fmt on every push'
type: chore
status: done
milestone: foundations
assignee: Oddur Sigurdsson
depends_on:
- c7e9ab80-43d1-4121-8e75-e2dcb9bf3340
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p2
api: none
effort: s
layer: tooling
area: tests
---

## Why

Keep the engine honest automatically.

## Acceptance criteria

- [x] Workflow runs on macOS and Linux
- [x] Determinism test is part of it

## 2026-09-22

Added .github/workflows/ci.yml: fmt; clippy -D warnings + tests + 3-day headless soak on ubuntu and macOS; cairn check + ROADMAP.md freshness (cairn pinned 0.2.1). Added rustfmt.toml (width 120) and fixed all clippy warnings. Every step passes locally. Not yet run on GitHub: the repo has no remote. Close once the first run is green.

## 2026-09-22

First green run: https://github.com/oddurs/rim/actions/runs/35813162619 — fmt, roadmap, and tests on ubuntu + macos. The 3-day soak produced state hash 7e7a47f8e642f43c on Linux x86_64, macOS ARM, and locally: cross-platform determinism holds. Toolchain pinned to 1.98.1 after stable's new some_filter lint broke the first run.
