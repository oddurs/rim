---
id: 049e2f73-0d64-48ff-bf2e-21264f765d35
title: 'Crafting as a plugin: recipes, stations and bills'
type: feature
status: backlog
milestone: stone-age
depends_on:
- 74b6fa7e-7a36-44ba-a15d-d42c9a04dc19
- 5305a161-c3e2-44b7-93bc-bf06016c11a0
created: 2026-09-22
updated: 2026-09-25
priority: p0
api: additive
effort: l
layer: plugin
area: building
---

## Why

Production chains, which DESIGN.md §5 places outside core: "crafting benches, bills, research" are depth, and depth is a plugin. The engine's part is work orders (74b6fa7e). This is everything above them: what can be made, where, and how many. The stone age (e3846c47) is its first consumer, and cooking (2c03427e) and tailoring (0927f0af) come next.

## What

- **`mods/crafting` declares a `recipe` kind** (`[[kind]]`, §10): `label`, `station` (a tag), `inputs` (each a def or tag with a count), `work`, `requires` (tool tags), `outputs`, `stuff_from` (which input's material the output takes), and a `skill` for when skills land (29c323f5).
- **A station is any thing whose tags name it.** The plugin ships `crafting:spot`, a free ground station: no cost, 20 work to mark, like a campfire without the fire. So "knapping on the ground" needs no special case, because every recipe has a station, and the cheapest station costs nothing. It also ships `crafting:workbench`: structural stuff, a spot in front, faster work.
- **Bills live on a station:** do N times, do until you have N (`rim.count_items`), or forever. They can be suspended and reordered. The plugin keeps them in its script data and turns the top runnable bill into one order at a time.
- **The bills panel** extends the thing inspector (`core:inspector.thing`, 5305a161). Add a bill from the station's recipes, set its count and mode, and see why it's stalled ("no flint", "needs a pounding tool").
- **Outputs appear at the station**, carrying `stuff_from`'s material.
- **`rim test` scenes** cover a bill running to its count and "until you have N" stopping and restarting.

## Acceptance criteria

- [ ] Recipe defs, declared by the plugin as a kind
- [ ] Bills with repeat modes: N times, until you have N, forever
- [ ] A free ground station, so recipes need no special "no station" case
- [ ] The bills panel on a selected station, with the reason a bill is stalled
- [ ] Outputs carry their input's material
- [ ] Core alone plays unchanged with the plugin removed

## 2026-09-24

Shaped by the stone age tier (e3846c47), which is its first consumer. What that tier needs from recipes: inputs matched by tag, a station that can be the bare ground, a tool requirement delegated to 7016d86b, and outputs that carry their input's material (`defs.rs:256`).

## 2026-09-25

Replanned as a plugin on work orders (74b6fa7e), per §5 and §10: the engine owns the job, and this plugin owns recipes and bills. "No station" became a free ground station rather than an optional field. Moved from crafting to stone-age, since the stone age needs it first.
