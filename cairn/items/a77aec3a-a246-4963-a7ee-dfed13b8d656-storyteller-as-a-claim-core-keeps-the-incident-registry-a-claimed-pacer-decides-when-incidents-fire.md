---
id: a77aec3a-a246-4963-a7ee-dfed13b8d656
title: 'Storyteller as a claim: core keeps the incident registry, a claimed pacer decides when incidents fire'
type: feature
status: backlog
milestone: story
depends_on:
- 96db8898-cf55-4517-acd7-260c05fe9d08
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: additive
effort: m
layer: core
area: storyteller
pillar:
- plugin-first
- wealth-gravity
---

## Why

"No difficulty slider" is core's opinion, not an engine rule (DESIGN.md §6,
"Tension: which opinions does the engine enforce?"). But today no mod can
replace core's storyteller.

`mods/core/scripts/storyteller.luau` holds two things:

- the incident registry every mod registers into (`register_incident`,
  `fire`, `ids`);
- the pacing: a `rim.every(500)` roll with grace days, a mean time between
  incidents, a pull from wealth, and threat weighting.

The `rim` table is read-only and a module's exports are frozen. So another
mod can add incidents but can't change when they fire. Two mods that each
ran their own `rim.every` loop would both fire, doubling the pace, which is
the case §4g rules out.

## What

Split the storyteller in two, and make the second half a claim (the claims
item, 96db8898).

**The registry stays core's vocabulary.**

- `register_incident`, `fire` and `ids` keep their signatures. Every mod
  that calls `require("@core/scripts/storyteller")` keeps working unchanged:
  core's own incidents, weather and wildlife_plus.
- The registry owns eligibility (`min_day`, `can_fire`), running an
  incident, and the memory of the last incident and last threat in
  `core:storyteller` script data.
- Offers from story threads (e641fbdb) go into the registry too.

**The pacer decides when an incident fires, and which.** A new export:

```lua
storyteller.register_pacer({
	id = "gravity",
	-- Called every CHECK_TICKS with the colony's context and the eligible
	-- incidents at their base weights. Returns the one to run, or nil.
	tick = function(ctx, pool) ... end,
})
```

The grace days, mean time between incidents, pull from wealth and threat
weighting move out of the registry into core's `gravity` pacer, unchanged.
The default game plays exactly as it does today.

**Which pacer runs is data.** Core declares a `storyteller` def kind
(`[[kind]]` already exists) with `name`, `description` and `pacer`, and
ships:

```toml
[[storyteller]]
id = "gravity"
name = "Gravity"
description = "Wealth is gravity: the richer you are, the more comes."
pacer = "core:gravity"
claims = ["storyteller"]
```

- A replacement mod ships its own `[[storyteller]]` claiming `storyteller`,
  and registers its pacer.
- The claims mechanism reports the clash and the player picks. With no
  pick, a new game refuses to start.
- The registry calls the pacer named by the def that won.
- A def that names a pacer nobody registered is a `rim check` error.

**Constraints**

- A pacer draws from its own mod's RNG stream (DESIGN.md §7b), so switching
  storytellers doesn't reshuffle other mods' dice.
- The premise schema keeps no difficulty field (DESIGN.md §4g).
- The engine gains nothing named storyteller or difficulty.
- The pick is fixed for the run. Changing the storyteller mid-run is out of
  scope.

## Acceptance criteria

- [ ] Core's storyteller is a registry plus the `gravity` pacer; core's storyteller tests and the determinism test pass unchanged
- [ ] The split keeps the order of random draws: a 30-day run's incident log (ids and ticks) on three seeds is identical before and after, and the comparison is noted on this item
- [ ] A test builds a temporary mod with a second `[[storyteller]]` claiming `storyteller` and a pacer that never fires: picked, 20 days see no incident; with `gravity` picked, the run matches core's
- [ ] With both enabled and no pick, a new game refuses and names both storytellers (test)
- [ ] A `[[storyteller]]` whose pacer isn't registered is a `rim check` error naming the file and id (test)
- [ ] The `register_incident` calls in core, weather and wildlife_plus are unchanged
- [ ] `register_pacer` and the `storyteller` kind are in the API declaration, docs/modding/scripting.md and the generated types
