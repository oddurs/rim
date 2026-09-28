---
id: b7f5cde1-a8c4-4d7d-b93d-bdf0ba0d523b
title: 'Mod index: a git repo of mod entries, checked by CI'
type: feature
status: backlog
milestone: platform
depends_on:
- be5845a1-bb0d-4c13-afa7-ac5d87f62d5d
- eb2c9422-7ed6-4060-806c-6d1cee5a0ba1
created: 2026-09-23
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Discovery without a closed store: adding a mod is a pull request (DESIGN.md §10, decided in 0120).

## Acceptance criteria

- [ ] Index repo layout: one TOML per mod id (repo URL, description, tags)
- [ ] Index CI on each PR: unique id, SPDX `license` present, `rim check` and `rim test` pass on the latest tag
- [ ] Mod ids are first come, first served; renames keep an alias
- [ ] Published as a static JSON snapshot the game can fetch
- [ ] Each release is pinned by content hash, so a moved tag can't change what a lockfile installs
- [ ] A `compat/` directory takes third-party rules (`breaks`, `load_after` hints, known-broken versions) by pull request; the loader names the source of each rule it applies

## 2026-09-27

Decided 2026-09-27 (modding review, PR #246): pin releases by content hash (Modrinth pins SHA-512; the fractureiser malware spread through popular modpacks), and give third parties a compat/ directory (RimSort's community rules and SMAPI's compatibility list exist because authors' metadata is never complete). The crater's results land in compat/results/ (f2a251f6).
