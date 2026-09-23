# RIM — Design

A 2D colony sim. You begin as one naked warrior with nothing. You end as a town
that the world has noticed. The game is a shell, and everything you play is a
plugin, including the base game.

This document records decisions and the arguments behind them. Each section is
written as a tension: the strongest case on each side, then a ruling. When a
ruling turns out wrong, change the ruling and keep the argument.

---

## 1. The one-sentence game

> **Everything you build makes you visible, and visibility brings the world to you: settlers, traders, beasts and raiders.**

There is no difficulty slider and no scenario picker, so the game needs one
rule that produces its own difficulty curve. That rule is **Wealth is gravity.**
Every event in the game is something being pulled toward your colony.

### Tension: wealth-scaled threats punish building

- **For pure wealth scaling:** it's simple, readable and proven. RimWorld does it.
- **Against:** RimWorld players learn to *avoid* building nice things, burn
  wealth, and "wealth-manage". A core loop that punishes the thing the game is
  about is broken.
- **Ruling:** wealth pulls in the good *and* the bad, in the same currency.
  Richer colonies draw better settlers, more traders and rarer opportunities,
  alongside bigger raids. The raid budget also weighs *defensive strength*
  (armed colonists, walls, turrets) against *exposed wealth*. A rich, fortified
  colony reads as a hard target; a rich, naked one reads as prey. Building
  defences is how you "spend" your visibility safely. The scorer is a stat
  pipeline, so plugins can add to either side.

---

## 2. The arc: eras instead of scenarios

There are no scenarios, so the shape of a run has to come from inside the game.

| Era      | Reached by                          | What the world sends                         |
|----------|-------------------------------------|----------------------------------------------|
| Castaway | start: 1 pawn, nothing              | weather, hunger, predators                   |
| Camp     | shelter + a bed + a fire            | wanderers, small animal threats, lone raiders |
| Hamlet   | 3+ settlers, wealth threshold       | raiding bands, traders, sieges of hunger     |
| Village  | 8+ settlers, wealth threshold       | organised raids, caravans, diplomacy         |
| Town     | 15+ settlers, wealth threshold      | wars, rival towns, legendary events          |

- Eras are **data**: `[[era]]` defs with conditions. Plugins can add, remove or
  re-threshold eras and listen for `era_reached`.
- Eras only go forward. Losing wealth doesn't demote you, but the storyteller
  still reacts to current wealth *inside* an era.
- There is no win screen. The run ends when the last colonist dies, and it
  finishes with a chronicle of what happened.

### Tension: endless sandbox vs. a goal

- **For a goal:** runs without a goal fizzle out, and a goal gives decisions weight.
- **Against:** a fixed goal is a scenario in disguise, and you said no scenarios.
- **Ruling:** eras give the run direction without a finish line. A plugin can
  add a win condition (build a ship, found a kingdom) as its own event chain.

---

## 3. The founder

You start as *the warrior*: a strong fighter with nothing on them.

- **For making the founder special (founder death = game over):** strong
  identity and real stakes.
- **Against:** one bad wolf fight on day two ends a 30-hour run. That's
  frustrating, not dramatic.
- **Ruling:** the founder has a **Founder** trait (a combat bonus, and settlers
  are more willing to join while they live). Their death is a major colony
  event (a mood blow once mood exists, recorded in the chronicle) but it isn't
  game over as long as anyone else is alive. Early on, before anyone joins, the
  founder *is* the colony, so the stakes are highest exactly when the game is
  simplest.

---

## 4. Why shelter matters

"Build shelter" is meaningless unless the world hurts you outdoors.

- **Ruling:** the core game has **Exposure**. Nights and bad weather drain
  `warmth` for pawns under open sky. An enclosed room (walls and doors, cut off
  from the map edge) protects them. Sleeping in a bed in an enclosed room
  restores rest fastest. This single mechanic gives the first hour its goal:
  *get four walls up before the second night.*
- Temperature, roofs and seasons can deepen this later as a plugin. The core
  only needs "enclosed or not".

---

## 4b. Fights end in retreat, not death spirals

The first balance pass (`examples/balance.rs`: a bot plays the opening on
many seeds) found that **9 of 20 colonies were wiped out by day 5** and 16
of 20 lost someone. Food was never the problem: nobody went below 23%. Threats
were. Two causes:

- **Every fight was to the death.** Raiders are the same human as colonists,
  so each fight was a coin flip with a corpse at the end.
- **Retreat only worked for one side.** Once raiders could flee, a duel
  harness (`examples/duel.rs`) showed 43 colonist deaths against 1 raider
  death in 100 even 1v1 fights. A wounded raider walks off the map for good;
  a wounded colonist stays nearby and gets picked as a target again.

- **Options considered:** a longer grace period (only delays the wipe);
  weaker raiders (hides the problem and makes combat meaningless); a downed
  state with rescue (right eventually, but it's the Defense milestone).
- **Ruling:** creatures retreat, and retreat is symmetric.
  - `retreat_below` on a creature def (humans 0.3, wolves 0.25): below that
    fraction of max hp, colonists run and heal, while hostiles and hunting
    predators leave the map.
  - **Mercy rule:** nobody picks a wounded or retreating creature as a *new*
    target, and nobody chases one down. Anyone adjacent can still land a blow,
    so being surrounded stays deadly.
  - Colonists defend each other within 20 cells, so they don't get picked off
    one at a time.
  - Raids end: the storyteller gives raiders `rim.leave_after` 0.6 days.
  - Raid size is 80 threat points per raider (was 55), about one colonist's
    worth, so a lone warrior faces one raider and raids grow with wealth.
- **Result**, 40 seeds to day 5: 1 colony lost, 5 runs with a death, and
  **28 near-misses** (someone below 35% hp). Close calls are the norm and
  deaths the exception, which is what we want. Even fights now kill about as
  many raiders as colonists.
- **Unchanged:** need rates (food is never critical, so hunger pressure waits
  for seasons and the Shelter milestone) and the 2-day grace period (first
  threat lands around day 3, earliest day 2.05).
- **Revisit** when downed/rescue arrives in Defense: retreat should become the
  fallback, and being downed the usual outcome of losing a fight.

Re-run `cargo run --release -p rim_sim --example balance -- --seeds 40` after
any change to combat, creatures or incidents.

---

## 5. What's in `core` and what isn't

`core` is the smallest complete game. Everything else is a plugin, including
things we build ourselves.

| In `core`                                     | Out (plugins, first-party or community) |
|-----------------------------------------------|-----------------------------------------|
| Terrain, plants, rocks, map generation        | Seasons, temperature simulation         |
| Needs: food, rest, warmth                     | Mood, mental breaks, relationships      |
| Harvest, mine, build, haul (via delivery)     | Crafting benches, bills, research       |
| Melee combat, health, death                   | Ranged weapons, armour, medicine        |
| Wild animals, predators, hunting              | Taming, farming animals                 |
| Storyteller, wealth, eras                     | Trade, factions, diplomacy              |
| Incidents: raid, wanderer, herd, predators    | Everything else                         |

- **Tension:** mood is RimWorld's soul, so leaving it out makes the core feel thin.
- **Ruling:** ship mood as the **first first-party plugin** (`rim.mood`). If
  mood can't be built as a plugin, the API is wrong, and that's the test we
  want to run early.

---

## 6. Architecture: engine verbs, plugin nouns

The engine knows *mechanisms*. It never knows *content*.

```
rim_sim (engine)                        mods/core (content)
────────────────                        ───────────────────
harvestable, buildable, edible, bed  ←  tree_oak, wall_wood, berries, bed_wood
need w/ decay + satisfier kind       ←  food, rest
creature w/ stats                    ←  human, deer, wolf
incident registry, scheduler, events ←  storyteller.luau, raid.luau, ...
stat pipeline (wealth, threat)       ←  modifiers from every mod
```

### Three plugin tiers

1. **Data (TOML defs + patches).** Most mods live here. Patches are declarative
   (`target = "thing/wood"`, `set = {...}`), and the loader **detects
   conflicts** (two mods setting the same field) and reports them rather than
   letting the last one win silently.
2. **Script (Luau).** Sandboxed, typed, fast. Behaviour lives here: incidents,
   the storyteller, custom events. Scripts get a deterministic RNG and no I/O.
   Every hook call is timed **per mod**, and the timings are visible in-game (F3).
3. **Native (WASM, later).** For heavy systems such as new pathing or fluid
   simulation. Same API surface, compiled.

### Tension: typed extension points vs. "patch anything"

- **For patch-anything (Harmony-style):** unlimited power. Modders never wait on
  you to add a hook.
- **Against:** it's the main cause of mod conflicts, it destroys performance on
  hot paths, and every engine update breaks mods.
- **Ruling:** named extension points plus a stat pipeline plus events. When
  modders need a hook that doesn't exist, that's a feature request against the
  engine, and it's cheap to add because the engine is small. The API is semver'd
  (`api = "0.1"` in `mod.toml`).

### Load order

The loader discovers `mods/*/mod.toml`, topologically sorts on
`depends`/`load_after`, and breaks ties by id so the order is deterministic.
Then it merges defs, applies patches and runs scripts in that order.

---

## 7. Determinism is non-negotiable

- All player input becomes a `Command` that is applied at a tick boundary.
- Integer or fixed-point simulation math, a seeded RNG owned by the world, and
  ordered iteration.
- Rendering interpolates. It never feeds back into the simulation.

This buys us replays, reproducible bug reports ("seed + mod list + command
log"), desync-checkable **co-op lockstep multiplayer** later, and a headless
simulation for tests and CI.

---

## 8. Performance budget

Target: **250×250 map, 30 colonists, 200 total pawns, 6× speed, 60 fps** on a
mid-range laptop. That means ≤ 2 ms per sim tick at 6× (≈ 360 ticks/sec).

- Each system declares a tick interval (every tick, every N ticks, or
  event-only), and work is **staggered** by entity id.
- Pawns think only when idle, on a staggered cadence. Jobs run as small state
  machines.
- **Reachability regions** (flood-fill ids, rebuilt only when passability
  changes) reject impossible targets in O(1) before A* runs.
- A* uses generation-stamped scratch buffers with no allocation per search.
  Hierarchical pathing and flow fields for raids come later, behind the same
  interface.
- Wealth and other aggregates are cached and recomputed on an interval.
- Rendering culls to the viewport. Chunked meshes come later.
- Per-system and per-mod profiler overlay from day one.

---

## 9. Milestones

1. **Castaway** (vertical slice): map gen, one warrior, harvest, mine, build
   walls/doors/beds, food and rest needs, animals, melee, the Luau storyteller
   with raids and wanderers, wealth, F3 profiler. Two mods loaded: `core` and
   an example plugin that adds content, patches core and scripts an incident.
2. **Shelter:** warmth, enclosed rooms, day/night visuals, eras.
3. **Save/load:** string-keyed component serialisation; unknown mod data is
   preserved.
4. **`rim.mood`:** the first first-party plugin, and the test of the API.
5. **Stockpiles and hauling, work priorities, skills.**
6. **WASM tier, mod browser, co-op lockstep.**
