---
id: c2579dbc-dfcf-418d-994f-187289e86387
title: 'Random streams per purpose: a new draw in one system stops reshuffling the rest'
type: feature
status: doing
milestone: proving-ground
assignee: Oddur Sigurdsson
claimed: 2026-09-27
created: 2026-09-27
updated: 2026-09-28
priority: p0
api: additive
effort: m
layer: engine
area: sim
---

## Problem

The world has one RNG and 29 draw sites in seven files share it (terms 6, systems 5, script 5, ai 4, world 3, mapgen 3, map 3). A new draw anywhere changes every roll after it: the map's animals, a raid's target, and what every mod's `rim.random()` returns. Tests written against a seed's behaviour break for no reason (#207, #211), and installing one mod changes another's dice. DESIGN.md §7b.

## Proposal

- `rng.rs`: `World::stream(name)` returns a stream derived as `Rng::new(mix(seed ^ hash_str(name)))`, created on first use and kept in a sorted map. Streams: `spawns`, `sim`, `ai`, `weather`, `story`, and `mod:<id>` for each mod's `rim.random()` and `rim.random_int()`.
- A counter-based draw for per-entity decisions: `draw(stream, entity, tick, k)`, a hash, independent of system order. Use it where a system iterates entities and draws.
- Every stream's state is saved and loaded with the snapshot; bump the save format.
- `docs/modding/scripting.md` says each mod has its own stream.

## Acceptance criteria

- [x] A test: adding a mod that calls `rim.random()` every tick changes nothing else in a 3-day run on core (same spawns, same AI outcomes, same state hash outside that mod's data)
- [x] A test: an extra draw in the `ai` stream leaves the map's spawns identical
- [ ] Save, load and continue matches never saving (the existing test, still passing), and the crosscheck agrees on all four platforms
- [x] The save format version is bumped and an old save reports a version change, not a divergence
- [x] Determinism test passes

## 2026-09-27

Streams: spawns (mapgen, plant spread, new pawns' names, skills and first think), sim (systems' rounding, as counter-based draws by pawn, need and tick in the needs loop), ai (wander, flee, idle delay, melee damage; the AI functions don't have the entity, so a stateful stream, not counter draws), and mod:<id> for rim.random, rim.random_int, rim.edge_cell and rim.near_cell (the calling mod's, by calling_mod). Snapshot format 6 saves each open stream's state; a format-5 save's single rng is ignored and its streams open fresh from the seed. state_hash folds every stream. Tests: a mod drawing every tick changes no pawn or thing in a day on core; extra AI draws leave plant spread identical while changing the game; streams are independent and restore exactly. Seed-coupled tests this exposed, fixed rather than re-seeded: why::the_first_in_line_takes_it (a spawned pawn drew next_think 0 and took the tree during the step), board::cells_show_the_rules_and_say_why (the founder drew no construction skill), engine::forty_colonists... (clicked a row built just outside the view). Criterion 4: the format is bumped; 'an old save reports a version change, not a divergence' needs the epoch's engine version to be the commit, which is cec3efdf's criterion 2 (today it's always 0.1.0). Criterion 3 is ticked when CI's agree passes on the PR.

## 2026-09-27

After an independent review: (1) an old save is now another engine. The epoch's engine id is the package version plus the snapshot format ('0.1.0+s6'), so a format-5 save opens a new epoch on load ('the mods or the engine changed') and its old epoch refuses to replay rather than diverging (unit test a_save_from_before_streams_is_another_engine). That meets criterion 4 before cec3efdf's commit-as-version. (2) Isolation: a new pawn's name, skills and first think are drawn by the id it's about to get, and AI rolls by pawn, tick and draw index, so a mod that spawns people or one more animal moves nobody else's rolls (test a_mod_spawning_people_leaves_plant_spread_alone). (3) A format-5 save opens its spawns and mods' streams from its old rng state, not the game's opening rolls. (4) The forty-colonists test clicked a row at its unscrolled place and never really tested the click; it now scrolls back to the top and must click one. rim.random draws from the stream of the mod whose code calls it, so a mod calling weather's helper draws from weather's stream, by design.

## 2026-09-27

People (5d09b04e): appearance (8469a7ff) is the first per-entity counter-based draw outside the sim's systems: picks rolled at spawn, keyed by the pawn.

## 2026-09-28

Rebased onto b591c0c3: apparel wear (#267) rounded with the shared rng in pawn order; it now draws from SIM keyed by the garment and tick. Fire (#270) draws through rim.random, so from mod:fire, which a format-5 load opens from the old rng like every other mod's.

## 2026-09-28

Rebased onto #283: water_depth's rising-water test lost its drafted pawn to a core:wolf on dry ground at tick 8630, after the water had done its work; the new rolls brought the wolf. With quiet-field (the test's author), the test now despawns Wild and Hostile pawns before each step, as animals.rs's isolated() does; its assertions are unchanged. The drafted pawn not defending itself is filed as f3aa2844.
