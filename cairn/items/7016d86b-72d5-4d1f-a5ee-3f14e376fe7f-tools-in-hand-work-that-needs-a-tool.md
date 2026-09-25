---
id: 7016d86b-72d5-4d1f-a5ee-3f14e376fe7f
title: 'Tools in hand: work that needs a tool'
type: feature
status: backlog
milestone: stone-age
created: 2026-09-24
updated: 2026-09-25
priority: p0
api: additive
effort: l
layer: engine
area: sim
---

## Why

"Hands first, then tools" is a gate, and the engine has no gate. Any pawn can mine granite bare-handed today (`mods/core/defs/nature.toml:39`), and nothing asks what a pawn is holding. Pawns have one carried stack (`Pawn.carry`, `world.rs:169`) and no equipment at all.

The vocabulary already exists: `ThingDef::tags` are free labels the engine matches as strings and never interprets (`defs.rs:131`). A tool gate is string matching over tags, so no content id reaches engine code (§6).

## What

- **A tool is an item with a `tool` block:** `tool = { tags = ["chopping", "cutting"], speed = 0.6, wear = 3 }`. Tags say what it does, `speed` scales work per tick, and `wear` is the hp a job costs it. Its material scales both: `MadeOf` stuff factors `tool_speed` and `hp` (`defs.rs:243`), so a flint axe and a bronze axe are one def.
- **One thing in hand.** A pawn holds at most one tool: the tool entity leaves the map's item layer and gets a `Held { by }` component. This is the slot equipment will reuse (d5d0ea1f): a spear is a tool that is also a weapon.
- **Gated work.** A harvest entry, or a work order (74b6fa7e), may say `requires = ["chopping"]`. A pawn takes the job only if its tool covers the tags, or if an unheld, unreserved tool that does is reachable. In that case the job gains a first stage: walk to the tool, drop what's in hand, and take it.
- **Wear.** A finished gated job costs the tool `wear` hp. At 0 it breaks: it's removed and the player gets a message ("Tarn's flint hand axe broke").
- **O(1) refusal.** Tool tags are interned to bits at load. The colony keeps a bitset of the tags its tools cover, held or loose, updated when a tool is made, breaks or is lost. Gated work the colony can't do is rejected without scanning pawns (work pools, 0e73145a).
- **It explains itself.** "Needs a chopping tool" is a refusal reason beside unreachable, reserved and no materials (§4d, f1924f03). A designated tree says so on hover, and a right-click order names it.
- **Saves:** an `engine:held` section, and the pawn's hand. Format bump.

## Budget

A gate is a bitset test. The fetch stage is one `nearest_item` over tools, only when the pawn's own tool doesn't cover the job. No per-tick cost beyond the job itself.

## Acceptance criteria

- [ ] `requires` on a harvest entry gates it; a pawn with no covering tool never takes it
- [ ] A pawn fetches a loose tool that covers the job, dropping what it held
- [ ] Making the tool unblocks the work without a reload
- [ ] Work speed follows `speed` × the material's `tool_speed`
- [ ] Wear breaks a tool at 0 hp, with a message
- [ ] Gated work the colony can't do is rejected in O(1)
- [ ] "Needs a chopping tool" is visible on the designated thing and in the why panel
- [ ] A held tool survives a save and load in its pawn's hand
- [ ] Determinism test passes
