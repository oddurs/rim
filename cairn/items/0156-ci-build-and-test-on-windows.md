---
id: 156
title: 'CI: build and test on Windows'
type: chore
status: backlog
milestone: shelter
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: tooling
area: tests
---

## Why

Windows is a target platform (0135), and the stack should build there (Rust, macroquad, hecs, Luau via mlua), but CI only tests `ubuntu-latest` and `macos-latest`. Windows can break without anyone noticing.

## Acceptance criteria

- [ ] `windows-latest` added to the test matrix in `.github/workflows/ci.yml`
- [ ] Clippy, `cargo test` (including determinism) and the headless soak pass on Windows
- [ ] Any Windows-only fixes (paths, line endings in mod files) recorded as notes here

## 2026-09-23

From 0163: also verify the UI resolves the system font on Windows (C:\Windows\Fonts\segoeui.ttf) and renders text; add a Windows run of the rim_ui test suite.
