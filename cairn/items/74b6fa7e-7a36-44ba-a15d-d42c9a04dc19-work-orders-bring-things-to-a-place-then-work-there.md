---
id: 74b6fa7e-7a36-44ba-a15d-d42c9a04dc19
title: 'Work orders: bring things to a place, then work there'
type: feature
status: backlog
milestone: stone-age
depends_on:
- 7016d86b-72d5-4d1f-a5ee-3f14e376fe7f
created: 2026-09-25
updated: 2026-09-25
priority: p0
api: additive
effort: l
layer: engine
area: sim
---

## Why

Crafting, cooking, research and construction are all the same job: bring these things to that place, then work there for a while. The engine already does it once, for blueprints: `Blueprint { cost, delivered, work, work_left }`, `Job::Deliver` and `Job::Construct` (`world.rs:201`, `ai.rs:713`, `ai.rs:780`). But it's welded to building.

DESIGN.md §10 says recipes are not an engine concept, and §5 puts benches and bills in plugins. So the engine should own the job and nothing else: an order that names its inputs, its place and its work, and tells whoever posted it when it's done. Blueprints become one kind of order. The crafting plugin (049e2f73) is another, and cooking (2c03427e) a third. That's the rule of three (§6).

## What

- **An `Order` component on a site entity.** It holds `needs: [(def or tag, count)]`, what has been delivered (with each input's `MadeOf`), `work`, `work_left`, `requires` (tool tags, 7016d86b), an optional spot (`SpotDef`, `defs.rs:125`), and the owning mod.
- **Inputs by tag as well as by def:** "2 × anything tagged `knappable`". Otherwise every mod that adds a material would have to patch every recipe.
- **Blueprints become orders.** `Deliver` and `Construct` generalise to any order, with no change in behaviour for building (the construction tests pass untouched).
- **Completion.** A blueprint becomes its building, as today. A scripted order raises `order_done { id, site, inputs, stuff }` to its mod, which makes the outputs. The inputs are consumed, and `stuff` is the material of the first input with a stuff category, so quality flows from material.
- **Luau surface:** `rim.post_order{ site, needs, work, requires, spot } -> id`, `rim.cancel_order(id)` (refunding what was delivered), `rim.orders(site)`, `rim.count_items(def_or_tag)`, and `rim.spawn_item(thing, x, y, count, stuff?)`. They are budgeted like any script call.
- Until work pools exist (0e73145a), orders join `find_work`'s nearest-first contest. When pools land they post there, under the work type the order names (03ad3e3a).
- **Saves:** an `engine:order` section. Blueprints move into it. Format bump, migrated on load.

## Budget

No per-tick cost for an idle order. The input search reuses `nearest_item`. A scripted order costs one script call when it completes.

## Acceptance criteria

- [ ] Blueprints run on orders; every construction and deconstruction test passes unchanged
- [ ] A script posts an order with tag inputs and a tool requirement, a pawn fulfils it, and `order_done` names the inputs and their stuff
- [ ] Cancelling an order refunds what was delivered
- [ ] `rim.count_items` counts by def and by tag
- [ ] `rim.spawn_item` takes a stuff, and the item keeps it
- [ ] An order survives a save and load, half delivered
- [ ] Determinism test passes
