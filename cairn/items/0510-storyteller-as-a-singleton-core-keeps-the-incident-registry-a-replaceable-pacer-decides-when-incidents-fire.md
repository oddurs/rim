---
id: 510
uid: a77aec3a-a246-4963-a7ee-dfed13b8d656
title: 'Storyteller as a singleton: core keeps the incident registry, a replaceable pacer decides when incidents fire'
type: feature
status: backlog
milestone: plugin-api
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
"Tension: which opinions does the engine enforce?"). Today no mod can
replace core's storyteller.

`mods/core/scripts/storyteller.luau` holds two things:

- the incident registry every mod registers into (`register_incident`,
  `fire`, `ids`);
- the pacing: a `rim.every(500)` roll with grace days, a mean time between
  incidents, a pull from wealth, and threat weighting.

The `rim` table is read-only and module exports are frozen, so another mod
can add incidents but can't change when they fire. Two mods each running
their own loop would both fire, doubling the pace.

## What

Split the storyteller in two, and make the pacer's seat a **singleton**, as
the calendar and the sky already are.

**The registry stays core's vocabulary.**

- `register_incident`, `fire` and `ids` keep their signatures, so core's own
  incidents, weather and wildlife_plus work unchanged.
- The registry owns eligibility (`min_day`, `can_fire`), running an
  incident, and the memory of the last incident and last threat in
  `core:storyteller` script data.
- Offers from story threads (e641fbdb) go into the registry too.

**A pacer decides when an incident fires, and which.** A new export follows
the pattern `rim.planner` already uses (a def names a function a script
registered):

```lua
storyteller.register_pacer("gravity", function(ctx, pool)
	-- ctx: day, wealth, colonists, strength, points, since_threat
	-- pool: the eligible incidents, each with its base weight
	-- returns the incident to run now, or nil
end)
```

The grace days, mean time between incidents, pull from wealth and threat
weighting move from the registry into core's `gravity` pacer, unchanged, so
the default game plays exactly as before.

**One storyteller, patched to replace.**

- `[[kind]]` gains `single = true`. A single kind holds exactly one entry,
  and a second definition is a load error that says to patch instead, the
  rule built-in singletons already follow.
- Core declares the kind and its one entry:

```toml
[[kind]]
id = "storyteller"
single = true
[kind.fields]
pacer = "string"

[[storyteller]]
id = "storyteller"
pacer = "core:gravity"
```

- A replacement patches that field:

```toml
[[patch]]
target = "core:storyteller/core:storyteller"
set = { pacer = "gentle_start:gentle" }
```

- With one replacement installed, it wins without a question. Two
  replacements setting `pacer` are a contested slot, settled by the ladder
  (dbb92ebe: a compatibility mod, then the player's pick, then
  load order, labelled).
- A `pacer` naming a function nobody registered is a `rim check` error.

**Constraints**

- A pacer draws from its own mod's RNG stream (DESIGN.md §7b), so switching
  storytellers doesn't reshuffle other mods' dice.
- The premise schema keeps no difficulty field (DESIGN.md §4g), and the
  engine gains nothing named storyteller or difficulty.
- The storyteller is fixed for the run like any def; changing it is a change
  of mods, so an epoch boundary.

## Acceptance criteria

- [ ] `[[kind]] single = true` works for any mod kind: a second entry is a load error naming both files, and patches to the one entry apply (tests)
- [ ] Core's storyteller is a registry plus the `gravity` pacer; core's storyteller tests and the determinism test pass unchanged
- [ ] The split keeps the order of random draws: a 30-day run's incident log (ids and ticks) on three seeds is identical before and after, noted on this item
- [ ] A temporary test mod patching `pacer` to a pacer that never fires sees no incident in 20 days; without it, the run matches core's (test)
- [ ] Two mods patching `pacer` are reported as a contested slot naming both (test)
- [ ] A `pacer` with no registered function is a `rim check` error naming the file and id (test)
- [ ] The `register_incident` calls in core, weather and wildlife_plus are unchanged
- [ ] `register_pacer`, `single` and the `storyteller` kind are in the API declaration, docs/modding/scripting.md, patches.md and the generated types
