---
id: c8598bbd-6102-4a6b-ab54-17342a239f6c
title: Mod packaging and distribution
type: spike
status: done
milestone: platform
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
api: none
effort: m
layer: tooling
area: modding
---

## Question

How do players get mods?

## Options

- Steam Workshop
- Own index + git URLs
- Both

## Decision

Own index plus git URLs first; a Workshop mirror later, fed from the same index (DESIGN.md §10). A mod is a git repo and a release is a tag. The index is a git repo of per-mod TOML entries, added by pull request and checked by CI. Players download release archives over HTTPS and never need git. Installs are pinned in a modlist lockfile.

Follow-up work: 0151 (versioned dependencies), 0152 (lockfile), 0153 (index), 0154 (rim add), 0155 (mod crater).


## Acceptance criteria

- [x] Decision recorded in DESIGN.md
