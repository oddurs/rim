---
id: e3846c47-f425-46f5-8e96-fec056074052
title: 'mods/primitive: the stone age as a plugin'
type: content
status: backlog
milestone: stone-age
depends_on:
- 049e2f73-0d64-48ff-bf2e-21264f765d35
- 4675019b-c017-47e7-b148-b12e4be58bda
- 4e9d5a12-ad0c-4170-bc53-5d8c2a5328a2
- 61c93a4e-e2d6-4422-9c53-5a3412d0a6f2
- 7016d86b-72d5-4d1f-a5ee-3f14e376fe7f
- 89f6d138-26e0-4d93-ad32-39c320d6e76f
- ccd44902-24d5-4855-9def-406d1ed39f4a
created: 2026-09-24
updated: 2026-09-25
priority: p0
api: none
effort: l
layer: plugin
area: modding
---

## Why

The early game should start with your hands: branches off a tree, fibre from the grass, loose stone off the ground, a sharp edge knapped from flint, and only then an axe that makes felling worth it. It's a tech tree, however short, and its pacing comes from materials and tools rather than a research screen.

DESIGN.md §5 puts this outside core ("depth goes out ... crafting chains"), so it ships as `mods/primitive`, on by default. That's also the test: it's the first real consumer of several harvests (4e9d5a12), tools (7016d86b) and crafting (049e2f73). If a stone age can't be built out of those, the API is wrong (pillar 3681b319).

## What

**Materials**, all items:

| Item | From | Tags | Stuff |
|---|---|---|---|
| branches | gather an oak (regrows), gather deadfall | `branch`, `fuel` | structural: hp 0.4, work 0.5, insulation 0.6 |
| plant fibre | gather tall grass, reeds on marsh | `fibre` | none |
| stones | gather loose stones | `stone_raw`, and a tool: `pounding` 0.4 | none |
| flint | gather flint nodules (scarce: sand, rock edges) | `knappable` | lithic: tool_speed 1.0, hp 1.0 |
| bone | butchering, via a patch on creatures | `knappable` | lithic: tool_speed 0.7, hp 0.8 |
| clay | gather a clay bank, with a `digging` tool | `clay` | structural (cob): hp 0.8, work 1.2, insulation 1.6, flammability 0 |
| cordage | crafted from 3 fibre | `cordage` | none |
| clay pot | fired from 3 clay at a campfire | `vessel` | none |

**Wild things** are natural fixtures with `harvest = { designation = "gather" }` and mapgen `spawn`s:
- deadfall, on grass and dirt: 4 branches, destroyed;
- tall grass, on grass and marsh: 3 fibre, spreads;
- loose stones, on dirt, sand and rock floor: 3 stones;
- flint nodules: 2 flint, rare;
- clay banks, on marsh (mapgen places by terrain and density; a bank that hugs the water needs a mapgen stage, e63fd9c3): 4 clay, requires `digging`, not destroyed, regrow in 4 days, so a bank is a pit you keep coming back to.

`tree_oak` gains a second harvest by patch: gather gives 3 branches, regrows in 3 days, 60 work.

**Tools**, made at `crafting:spot`, taking their material from the knapped input:

| Tool | Recipe | Needs | Tool tags | Speed | Wear |
|---|---|---|---|---|---|
| hammerstone | the stones item itself | none | `pounding` | 0.4 | 0 |
| flint flake | 1 knappable | `pounding` | `cutting` | 0.6 | 2 |
| hand axe | 2 knappable | `pounding` | `chopping`, `cutting` | 0.6 | 3 |
| hafted axe | 2 knappable, 1 branch, 1 cordage | `cutting` | `chopping` | 1.0 | 2 |
| stone maul | 3 stones, 1 branch, 1 cordage | `cutting` | `pounding` | 1.0 | 2 |
| digging stick | 1 branch | `cutting` | `digging` | 1.0 | 1 |

`core:campfire` gains the station tag `crafting:fire` by patch, and the clay pot is fired there: 3 clay, 300 work. Pots are the vessel cooking (2c03427e) will ask for. Until then they are wealth.

**Gates**, all patches on core:
- `core:tree_oak` chop requires `chopping`;
- `core:granite` mine requires `pounding`.

Hunting stays barehanded (a spear waits for equipment, d5d0ea1f).

**Night one** still works:
- branches are structural stuff, so walls, a door and a bed of branches build straight away (weak, and draughty through `insulation`);
- a patch lets a campfire take 10 branches;
- the Camp era (shelter, bed, fire, §2) is reachable on day one.

**After night one**, clay is the upgrade. Cob (clay walls) is warmer than branches and doesn't burn: branches, then cob, then wood and stone once the axe and maul are in hand. Each step is a reason to go further from the fire.

**Nudges**, as messages from the plugin's scripts: the first flint found, the first tool made, and "no flint within reach" on day two if none has been gathered.

## Acceptance criteria

- [x] Branches, fibre and stones are gatherable by hand, and oak branches regrow
- [x] A flint flake and a hand axe are made at a free ground station
- [x] Chopping and mining are gated behind the tools, and say why when blocked
- [x] Tool quality comes from material, not duplicate defs
- [x] A branch shelter and campfire are buildable with no tools
- [x] Clay is dug with a digging stick, builds cob walls, and fires into a pot at a campfire
- [x] A `rim test` scene goes from bare hands to a felled tree
- [ ] Core alone still plays with the plugin removed (§5 CI smoke test)

## 2026-09-25

First tier landed as mods/primitive (gather by hand): branches, fibre, loose stones and flint as items; deadfall, tall grass, loose stones and flint nodules as wild things with core:gather harvests and spawns; oak branches that regrow; branches as structural stuff; a campfire of 10 branches. Nothing is gated yet, so core's bare-hand chop and mine still work until tools (7016d86b). With the plugin on, an unmarked oak's right-click gathers (the gentlest harvest), so tests of core's own rules now load core alone. Loose stones spawn on dirt and sand only: rock_floor is all granite.

## 2026-09-25

Split into three: stone tools and the gates (4675019b), clay (89f6d138) and bone (61c93a4e). Each is a PR of its own. This item stays as the umbrella: its criteria are ticked as the parts land, and 'core alone still plays' is checked once at the end.

## 2026-09-25

The materials table above lists the clay pot's stuff as none. As built (89f6d138), the pot is made of its clay: a clay pot, at clay's hp. The hammerstone is its own item, shaped from a stone (4675019b).
