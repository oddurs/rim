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

### Tension: roofs, or is enclosure enough?

- **For explicit roofs:** a roof is the obvious thing shelter means, and it
  allows open-sided sheds, overhangs and caves.
- **Against:** it's a second thing to build, a second overlay to read, and a
  new player doesn't know about it on night one. The first-hour goal is "four
  walls before dark", and walls should be enough.
- **Ruling:** a **room** is an area bounded by walls, doors, rock or water,
  cut off from the map edge, and at most `MAX_ROOM_CELLS` (400, i.e. 20×20)
  in size. Enclosed rooms count as indoors. The size cap is what an automatic
  roof would do: a valley ringed by mountains is not a house.
  - Doors are a def flag (`door = true`): passable for pathing, but they bound
    rooms like a wall does.
  - Rooms rebuild only when a wall, door or terrain changes, not every tick.
  - Scripts ask with `rim.indoors(x, y)` and `rim.room_at(x, y)`.
  - Explicit roofs, if ever wanted, are a plugin that marks cells roofed and
    hooks the same question.

---

## 4a. Field layers

Temperature, light, beauty, noise, danger and fertility are all the same
thing: a scalar value over the grid that the world produces and pawns and
systems read. So there is one engine mechanism, declared in data, instead of
a special case for each.

- **Ruling:** a `[[field]]` def declares a layer. Its value at a cell is a
  *base* plus *stamped* emitter contributions.
  - **Base:** the outdoor `ambient` under open sky, which scripts set (core's
    `20_climate.luau` drives day and night). Inside an enclosed room it
    depends on `indoor`: the room's own value (`room`, for temperature), zero
    (`none`, for light: indoors is dark unless something lights it), or the
    same as outdoors (`outdoor`).
  - **Emitters:** any thing def can `emit = [{ field, amount, radius, cap }]`.
    The contribution fades linearly with *walking* distance, so walls and
    doors block it. Each emitter remembers exactly which cells it touched, so
    adding or removing one costs only its own footprint, and a wall change
    re-stamps only the emitters within reach of it.
  - **Room state:** a `room` field holds one value per enclosed room. It leaks
    toward outdoors at `leak_per_hour` (insulation) and is pushed by emitters
    inside at `room_gain` (heating power), up to each emitter's `cap`. These
    are separate on purpose: how well a hut holds warmth and how fast a fire
    heats it are different questions. When walls change and rooms rebuild,
    each new room inherits the cell-weighted average of what its cells held,
    so a wall elsewhere changes nothing.
  - Inside an enclosed room a `room` field is the room's value only; the
    emitter's local stamp applies outdoors. (Adding both counted the same
    fire twice and made huts too hot to be comfortable.)
- **Cost:** O(1) to read a cell; O(footprint) per emitter change; O(rooms)
  per room update (every 60 ticks), not O(cells). Values are integers in
  hundredths, for determinism.
- **Needs** can be driven by a field: `satisfier = "field"` with a `comfort`
  range. The need drains in proportion to how far outside comfort the pawn
  stands and refills inside it. Warmth is the first; a mood plugin could
  drive comfort from beauty the same way.
- **AI** looks for the nearest comfortable cell by walking. Failing that it
  takes the least-uncomfortable cell in reach if it's clearly better than
  where the pawn stands (an unheated hut beats the night outside). Idle
  colonists wait somewhere comfortable instead of wandering in the cold.
- **Scripts:** `rim.field(id, x, y)`, `rim.ambient(id)`, `rim.set_ambient(id, v)`.
  **Client:** `O` cycles an overlay through every field that exists, so a
  mod's new layer gets a map view for free.
- **Scripts must not use `math.sin`/`math.cos`** for simulation values. They
  come from each platform's maths library and can differ in the last bit,
  which would break lockstep. The climate curve uses a smoothstep polynomial.

### Tuning warmth (balance harness, 40 seeds, 5 days)

Core climate: mean 10°C, swinging 9° either way (about 1°C at 03:00).
Warmth: comfort 10–32°C, drains fully in 0.2 days at 10° outside comfort,
refills in 0.1 days, hypothermia 40 hp/day at zero. Insulation 6%/hour,
campfire 12° with radius 5, capped at 24° in a room.

| Founder's hours at zero warmth after night one | Runs | Hours per run |
|---|---|---|
| No shelter | 40/40 | 14.5 |
| Hut with a bed | 17/40 | 2.6 |
| Hut with a bed and a campfire | 12/40 | 2.8 |

The remaining hut cases are runs where the bot's hut wasn't finished by
day 1. Exposure hurts and shelter fixes it, but a cold night isn't a death
sentence: deaths stay at the pre-warmth baseline. Harsher winters belong to a
seasons plugin, which only has to call `rim.set_ambient`.

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

---

## 10. Modding as a platform

§6 says how plugins plug in. This section covers who writes them and how the
work gets to players. The first modders will be developers who already live on
GitHub, and the platform should feel like publishing a small open-source
library: `rim new`, write typed code, `rim test` in CI, tag a release, open a
PR to the index. If that loop is good, content follows.

### Tension: where do mods live?

- **For Steam Workshop:** it's where players already look, and installing takes
  one click.
- **Against:** it's closed and tied to one store. You can't fork a mod, send it
  a pull request, review a diff or run CI on it. A mod that is abandoned there
  stays abandoned. It also doesn't exist for players who got the game any
  other way.
- **Ruling:** **a mod is a git repo**, and a release is a tag. Discovery goes
  through a **mod index**: a public git repo with one small TOML file per mod
  (id to repo URL). Adding a mod is a pull request, and the index's CI runs
  `rim check` and `rim test` on it. Players never need git: the game and the
  `rim` CLI download release archives over HTTPS. A Workshop mirror can come
  later as a second front door onto the same index.
- Every install is recorded in a **modlist lockfile** (exact versions plus
  content hashes). The same file pins replays, bug reports and co-op sessions.
  A modpack is just a lockfile someone shared.
- The index requires an SPDX `license` in `mod.toml`, so modpacks and forks
  know what they're allowed to do.

### Tension: global ids or namespaced ids?

- **For global ids (`wolf`):** they're short, and mods today can refer to core
  content without ceremony.
- **Against:** two mods that both add `iron` will collide, and with a hundred
  mods they will. Save files key on def ids, so renaming later breaks saves.
- **Ruling:** every def id is **namespaced by its mod**: `core:wolf`,
  `wildlife_plus:boar`. Inside a mod, a bare id means that mod's own def; any
  other mod's def needs the prefix. This is a breaking change, so it lands
  **before the save format** does, while breaking things is still cheap.

### Tension: how do mods talk to each other?

- **For the shared `rim` table (today):** it's simple. Core exposes
  `rim.register_incident` just by assigning it.
- **Against:** any mod can overwrite any function for everyone. That is
  patch-anything through the back door, which §6 ruled out. Names collide
  silently, and nothing records who depends on whom.
- **Ruling:** the `rim` engine table is **read-only**. Mods share code as
  **modules**: `require("@core/storyteller")` returns what that mod exports.
  A mod can only require mods listed in its `depends` or `optional`, so the
  dependency graph is real rather than hoped for. For loose coupling there are
  **namespaced events**: `rim.emit("wildlife_plus:stampede", data)` and
  `rim.on("wildlife_plus:stampede", fn)`. The rule of thumb: hard
  dependencies use `require`, soft ones use events.

### Tension: a fixed schema or one mods can extend?

- **For a fixed set of def kinds:** the engine validates everything and tools
  know the whole schema.
- **Against:** mood needs `thought` defs and crafting needs `recipe` defs.
  Neither is an engine concept, so the rule "if mood can't be a plugin, the
  API is wrong" fails immediately.
- **Ruling:** mods can **declare new def kinds** with a field schema. The
  loader validates, merges and patches them like built-in kinds and exposes
  them read-only to scripts. To attach data to *another* mod's def, a mod uses
  a table named after itself (`[creature.core:human.mood]` becomes
  `def.mood` in scripts). It can't collide with anyone else's data, and its
  owner is obvious.
- Content enums in the engine (`Faction`, `Satisfier`) become registries fed
  by defs. Draw primitives (`Shape`) and broad categories stay in code:
  those are mechanisms, not content.

### Tension: declarative patches or scripted defs?

- **For letting scripts generate and edit defs:** it's what power users want,
  e.g. twenty ore variants from one loop.
- **Against:** conflict detection, compatibility reports and index checks
  only work if changes are declarative.
- **Ruling:** stay declarative, and fill the actual gap, which is lists.
  Patches gain `append`, `remove` and match-by-key for arrays; today setting
  a list replaces all of it. Load-time def generation is **deferred** until a
  real mod needs it. If it comes, it must emit defs and patches through the
  same tracked pipeline.

### Tension: should players set the load order?

- **For manual order:** RimWorld players expect it, and it's an escape hatch
  when two mods fight.
- **Against:** it's hidden state. Two players with the same mods get different
  games, co-op desyncs, and bug reports can't be reproduced. Ordering is a
  job for the loader, not for players.
- **Ruling:** **order is derived from manifests only** (§6). When patches
  conflict, the mod manager shows the conflict and the player picks a winner
  per field. That choice is written into the modlist lockfile, so it is
  explicit, shareable and reproducible. Mod authors resolve conflicts
  properly with `load_after` or a compatibility patch.

### Tension: what can a mod from a random repo do?

- **For native code (DLLs, like RimWorld's C# assemblies):** unlimited power
  and speed.
- **Against:** "download code from a stranger's GitHub and run it" is only
  safe if the sandbox is real. One malicious mod would poison trust in the
  whole index.
- **Ruling:** **there is no native tier, ever.** Data and Luau mods can't do
  I/O. The Luau VM runs in sandbox mode, the engine tables are frozen, and
  each mod gets hard instruction and memory limits, so a runaway script is
  stopped and named instead of hanging the game. Installing a data or Luau
  mod needs no permission prompt. The WASM tier (§6) is the only one that
  declares capabilities.
- Scripts share the sim's determinism rules. Library math that can differ
  across platforms (`math.sin` and friends from the C library) is replaced
  with deterministic implementations, so co-op and replays hold across
  machines.

### Tension: a moving API or mods that rot?

- **For breaking freely before 1.0:** the API is young, and freezing it early
  locks in mistakes.
- **Against:** every break silently kills mods whose authors have moved on.
  Mod ecosystems die this way.
- **Ruling:** break freely, **but never silently**:
  - The API is declared once in Rust. The `.d.luau` type definitions and the
    reference docs are generated from that declaration, so they can't drift.
  - A deprecated call keeps working for one minor version, logs a warning
    with its replacement, and names the version it will be removed in.
  - **Mod crater:** engine CI runs every indexed mod's tests against the
    change. Breaks are found by us, before modders find them.

### Tension: how do modders know their mod works?

- **For "just play it":** it's how most games work.
- **Against:** the engine is deterministic and headless (§7). Not letting
  modders test with that wastes our biggest advantage.
- **Ruling:** **`rim test`** runs `tests/*.luau` against seeded headless
  worlds: set up a scene, advance ticks, assert. The GitHub Action that
  `rim new` generates runs it on every push, against the engine versions
  the mod supports.
- **Hot reload is deterministic replay.** When a file changes, reload the
  defs and scripts, rebuild the world from its seed and replay the command
  log to the current tick. You see what your change *would have done* in
  this exact game. Snapshots make it faster once saves exist.

### Tension: Luau, or a language more developers know?

- **For TypeScript or JavaScript:** far more developers know it.
- **Against:** a JavaScript engine is heavy, hard to sandbox deterministically
  and slow to embed. Luau is built for exactly this job: sandboxed, gradually
  typed, fast, and with a solid LSP (luau-lsp).
- **Ruling:** Luau for scripts. People who want other languages get them
  through the WASM tier, which compiles from Rust, AssemblyScript, Zig and
  others.

---

## 11. Interface

The UI follows the same split as the game: the engine provides a few UI
mechanisms, and **the interface you see is a mod** (`mods/core/ui/`). Any
panel, bar, menu or bubble can be extended, replaced, wrapped or removed by
another mod.

### Tension: immediate-mode widgets or a retained tree?

- **For immediate mode (egui, Dear ImGui, today's HUD):** quick to write, and
  there's no tree to keep in sync.
- **Against:** the only way to change it is to edit Rust. There's nothing to
  address, so mods can't reach in, restyle it or test it.
- **Ruling:** a **retained tree described in Luau**, built React-style: a
  component is a function from a read-only view of the game to a tree of
  plain nodes. The engine does layout (a flexbox subset via `taffy`),
  drawing, input and caching. Because a tree is data, mods can find a node by
  id and change it, tests can compare trees as text, and devtools can show
  which mod put what on screen.

### Tension: one Luau VM or two?

- **For sharing the sim's VM:** one runtime, and mods can call their own sim
  code directly.
- **Against:** the sim VM is deterministic and synchronised in co-op. UI code
  wants the wall clock, animation and per-player choices, and a UI bug must
  never be able to desync a game.
- **Ruling:** the UI runs in its **own client-only Luau VM**, sandboxed like
  the sim's (§10). It can only *read* the world through `view` and *act*
  through `act`, which queues the same Commands mouse and keyboard produce.
  Each co-op player can run different UI mods.

### Layout: a shell of regions and layers

- **Regions:** `top`, `bottom`, `left`, `right` dock panels at the screen
  edges; panels declare a region, an order and size limits, and regions
  stack them. The world stays visible under translucent panels.
- **Layers**, bottom to top: world, anchored (labels, bars, bubbles tied to
  entities or cells), docked, windows, menus and tooltips, modal, toasts.
  Input goes to the top layer first; whatever the UI doesn't handle falls
  through to the world.
- **Anchored UI avoids collisions:** labels near each other are nudged apart
  by priority (selected, colonists, hostiles, others), which fixes overlapping
  names once for everything.

### Looks: plain, minimal, tokens

- Every visual value is a **token** in `ui/theme.toml` (spacing on a 4 px
  grid, three text sizes, colours, radius, borders). Mods patch tokens like
  defs, with the same conflict reporting.
- Translucent dark surfaces, 1 px hairlines, one accent colour, no textures.
  Emphasis comes from weight and colour, not size.
- **The system UI font**, found at runtime and never shipped: San Francisco
  on macOS, Segoe UI on Windows, `sans-serif` from fontconfig on Linux, with
  fallbacks for other scripts. A theme can name another font.
- One **UI scale** multiplies every token and follows the display's DPI.

### Tension: stable ids or free-form trees?

- **For free-form:** less ceremony for small mods.
- **Against:** a mod can only change what it can name.
- **Ruling:** every component has a **namespaced id** (`core:clock`,
  `core:inspector.tabs`). Mods get four operations, applied in load order:
  `ui.extend` (add children), `ui.replace`, `ui.wrap` and `ui.remove`. Two
  mods replacing the same id is reported as a conflict, like def patches.

### Built for hacking

- **Hot reload:** saving a UI script or theme updates the running game
  without touching the simulation.
- **Devtools (F12):** hover any element to see its id, owning mod, layout box
  and tokens.
- **Per-mod UI time** shows in the profiler; a component that errors shows an
  error box in its place and the rest of the UI keeps running.
- Budget: **under 1 ms per frame** for the whole UI.
