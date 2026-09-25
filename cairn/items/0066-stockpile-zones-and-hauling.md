---
id: ecd54de8-e9f2-4767-a746-d751f906fb40
title: Stockpile zones with item filters
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-25
closed_at: 2026-09-25
priority: p0
api: additive
effort: l
layer: engine
area: ai
pillar:
- growth
---

## Why

Items should end up somewhere useful, not where they fell. The player marks
where things go; hauling (split out) is the work that takes them there.

## What

- A zone is cells the player paints and a filter of which items it takes;
  it lives in the world, in the state hash and the save, and changes only by
  commands.
- Painting a rectangle that touches one zone extends it; otherwise it makes
  a new one. Clearing cells shrinks a zone, and a zone with none is gone.
- The filter is a set of item defs; a new zone takes every item.
- The client draws zones and has a stockpile tool and a clear tool; a panel
  lists zones and toggles what each takes.

## Acceptance criteria

- [x] Zones with item filters, painted and cleared by commands (test)
- [x] Zones survive a save and a load, including across a mod change
- [x] The client draws zones and edits filters
- [x] Determinism test passes with zones

## 2026-09-25

Split: hauling is 8d551753. Built as crates/rim_sim/src/zone.rs: Zones { list, next_id, cells } in World, in the state hash, the snapshot (engine:zones, filters remapped by id across a def change with a note for what's lost) and the text save. Commands Stockpile { a, b, zone: Option } (None makes a zone taking every item), ClearZone, ZoneAllow. A zone with no cells is gone; ids are never reused. The client decides extend-or-new with Zones::touched (the one zone a drag touches). Client: Stockpile and Clear zone tools, zones drawn as a wash with edges in world_ui, a Stockpiles panel on Z (view.zones, view.items, act.zone_allow; UI API 0.4). Autotest paints, extends and toggles, and screenshots it.

## 2026-09-25

Review: ZoneAllow took non-items (walls); now items only. A drag over two zones made a third that swallowed them, losing their filters; painting now never takes a cell from another zone (clear it first). A hand-edited save could leave duplicate ids, cells of missing zones or repeated filter entries; Zones::tidy runs on load. By design, a zone takes every item that exists when it is made; an item a mod adds later is the player's to allow. Noted, not fixed: view.zones counts each zone's cells by scanning the map, fine while the panel is small.
