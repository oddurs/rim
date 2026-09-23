---
id: 149
title: 'rim: one binary to play, check, test and pack mods'
type: feature
status: backlog
milestone: sdk
depends_on:
- 146
- 156
- 159
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: client
area: modding
pillar:
- plugin-first
---

## Why

The modder's whole toolchain should be one download, the same binary players run and CI uses. Today the client binary is already named `rim` (`crates/rim_client/Cargo.toml`), so a separate `rim` CLI would collide with it.

## Decision

One binary. `rim` with no arguments starts the game; everything else is a subcommand. Players and modders install the same thing, and CI runs exactly what players run. Subcommands that don't need a window (`check`, `test`, `pack`) must not open one or need a GPU, so they run on headless CI machines.

## Acceptance criteria

- [ ] `rim` with no arguments starts the game as today; existing client flags (`--seed`, `--autotest`) keep working
- [ ] `rim check [path]` loads a mod with its dependencies and reports errors, conflicts, deprecations
- [ ] `rim test [path]` runs its tests (0146)
- [ ] `rim run [path]` starts the game with the mod in dev mode
- [ ] `rim pack` builds a release archive plus its content hash
- [ ] `check`, `test` and `pack` run with no display (tested under CI without Xvfb)
- [ ] Prebuilt for macOS, Windows and Linux on every engine release
