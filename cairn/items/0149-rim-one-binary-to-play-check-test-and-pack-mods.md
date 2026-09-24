---
id: be5845a1-bb0d-4c13-afa7-ac5d87f62d5d
title: 'rim: one binary to play, check, test and pack mods'
type: feature
status: backlog
milestone: sdk
depends_on:
- acaa16f4-f165-4ddf-a5aa-da860100abcb
- 48f92e6e-2187-461a-a98b-f20e36ae1325
- eb2c9422-7ed6-4060-806c-6d1cee5a0ba1
created: 2026-09-23
updated: 2026-09-24
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
- [x] `rim test [path]` runs its tests (0146)
- [ ] `rim run [path]` starts the game with the mod in dev mode
- [ ] `rim pack` builds a release archive plus its content hash
- [ ] `check`, `test` and `pack` run with no display (tested under CI without Xvfb)
- [ ] Prebuilt for macOS, Windows and Linux on every engine release

## 2026-09-24

rim test [path] landed with 0146 (#38?), dispatched before the window opens; check and pack can follow the same pattern in crates/rim_client/src/cli.rs.

## 2026-09-24

rim check [path] [--hours H] [--strict] landed: it loads each mod with only its dependency closure on a 96-cell map, plays 6 in-game hours, and reports load errors, load warnings (patch conflicts, no-match patches, pow lint) and script errors; exit 1 on an error, or on a warning with --strict. CI runs it on every OS with no display. Criterion 2 stays open for deprecations: they'll surface as load warnings once 63d2f10a adds them. check and test now both run headless in CI (criterion 6 still needs pack).
