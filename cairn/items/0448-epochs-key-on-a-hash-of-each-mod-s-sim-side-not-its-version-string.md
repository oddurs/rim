---
id: 448
uid: 173de74c-0bdd-41fa-98f0-201825aaca68
title: Epochs key on a hash of each mod's sim side, not its version string
type: feature
status: backlog
milestone: platform
depends_on:
- 542
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: engine
area: save
pillar:
- determinism
---

## Why

`Epoch::mods` is every loaded mod's (id, version), and `SaveFile::load`
starts a new epoch when that list changes (`lock_of` in `savefile.rs`). The
version is typed by hand and covers everything in the mod. Three failures:

1. Adding a theme mod starts an epoch and drops the unreplayed tail, for a
   mod the sim never reads.
2. Editing a def without bumping the version keeps the epoch, so the log
   replays under different code until a checkpoint hash disagrees.
3. Bumping core's version for a UI-only fix starts an epoch in every save.

Paradox computes its checksum over gameplay folders only. Factorio's CRC
covered only Lua that ran, so a `require` gated by a setting made
mismatched settings look like mismatched mods. The lesson: hash inputs, not
execution, and only the inputs the simulation reads. DESIGN.md §7a,
"Tension: what counts as a change of code?"

## What

- An epoch records, per **sim-side** mod (e4b96647), `(id, version, sim_hash)`.
- `sim_hash` covers:
  - the `mod.toml` fields the loader uses for the sim (`id`, `api`,
    `depends`, `optional`, `load_after`), with `version` excluded, so a
    version bump with identical content changes nothing;
  - every file under `defs/` and `scripts/`, by relative path and bytes, in
    sorted order.
- The epoch also records colony option values (fe54d733) and sim picks
  (dbb92ebe) once those land; a change to either is a change of code.
- The hash uses the crate's existing deterministic hash (`snapshot::hash_bytes`).
- The load report names the input that differs: "core: defs/buildings.toml
  changed", "wildlife_plus added".
- Epochs written before this change have no hashes and compare by version,
  as today.
- Presentation fields in defs (`look`, `color`, `label`) stay in the hash for
  now. Excluding them waits until no sim path reads text (DESIGN.md §4g).

The lockfile (0152) hashes every file of every mod, for integrity. That is a
different job from this one.

## Acceptance criteria

- [ ] Editing a def without a version bump starts an epoch, and the report names the file (test)
- [ ] Bumping a version with identical sim content starts no epoch (test)
- [ ] Adding or changing a client-side mod starts no epoch and loses no ticks (test)
- [ ] A save whose epochs predate hashes loads and compares by version (test)
- [ ] The determinism and save round-trip tests pass unchanged
- [ ] DESIGN.md §7a matches
