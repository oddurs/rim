---
id: 531
uid: d35b101b-276e-4203-944c-81dae700b1cc
title: 'migrates_from and declarative renames: a fork adopts a removed mod''s things'
type: feature
status: backlog
milestone: platform
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: additive
effort: m
layer: engine
area: save
pillar:
- plugin-first
- determinism
---

## Why

Mods are git repos, so forks will be common: a mod goes quiet, someone
carries it on under a new id. Today, loading a save without the original
drops its things (`restore_noting`, "dropped 3 × boars:boar") and its
script data is parked. A mod can't rename one of its own defs without the
same loss.

Factorio's `migrates_from` moves a removed mod's storage to its successor,
and its JSON migrations rename prototypes, recorded per save so each
applies once.

## What

- `mod.toml` gains `migrates_from = ["wildlife_plus"]`. When a save has
  `wildlife_plus` and not the successor, and the successor is loaded:
  - the old mod's script data moves to the successor, and its
    `rim.on_migrate` hook sees the old mod's id and version;
  - things of the old mod's defs are kept if a rename maps them.
- Declarative renames, in `mod.toml`:

```toml
[[rename]]
from = "wildlife_plus:boar"
to = "wildlife_redux:boar"
```

  A mod may rename its own defs the same way.
- Renames apply at an epoch boundary, when the snapshot is restored, and
  the epoch records which were applied, so each applies once.
- The load report lists what was carried over and what still dropped.

## Acceptance criteria

- [ ] A successor with `migrates_from` inherits the old mod's script data and renamed things from a save (test)
- [ ] A mod renaming its own def keeps every existing thing of it (test)
- [ ] A rename applies once: loading the new epoch again changes nothing (test)
- [ ] The load report lists what was carried over and what dropped
- [ ] docs/modding/scripting.md "Upgrading saved data" covers `migrates_from` and `[[rename]]`
