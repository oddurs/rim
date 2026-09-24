---
id: ee231dac-c777-46b8-a9f5-0f1c2228f803
title: 'Dependency configuration: Luau sandbox and determinism, upgrades, text and rendering'
type: chore
status: done
milestone: shelter
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: breaking
effort: m
layer: tooling
area: perf
---

Research and apply the optimal configuration for every main library. Reasoning and measurements: docs/engineering/dependencies.md. Merged as PR #19.

## 2026-09-24

PR #19. Luau: mlua 0.12 (Luau 736); the arm64 Luau VM had 41 fused multiply-adds (incl. the % operator), now built with -ffp-contract=off and checked in CI; sim VM sandboxed (deterministic libs only, escape hatches removed, read-only libraries, safe envs, compiler level 2, 256 MB limit, a counted step budget), script-heavy work 22.9 -> 13.1 ms; UI VM with a 250 ms per-call deadline and no memory limit (measured 45% slower rebuilds with one). hecs 0.11, toml 1.1, def errors name the key path and the patching mod. taffy flexbox only with one reused tree; a cached, preloaded system font list (33 -> 2 ms, ~220 ms cold); shape-run cache; 1x hinting. Renderer: macroquad's own Conf, terrain as one texture, cheaper circles: 155 -> 66 draw calls zoomed out. Build profiles (line tables, dist with fat LTO, profiling), CI on the pinned toolchain. api=breaking: removed Luau globals (collectgarbage, loadstring, getfenv...) and read-only libraries.
