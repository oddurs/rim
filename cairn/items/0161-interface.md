---
id: 7b325c7f-8a5c-4343-b898-1185c578bca3
key: interface
title: Interface
type: milestone
status: done
depends_on:
- 61abad41-ecc6-4597-9ad4-d0193639e828
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p2
api: none
due: 2026-10-09
---

A two-week sprint. The whole HUD moves out of Rust into `mods/core/ui/`, written in Luau on a small UI engine, so any mod can extend, replace, wrap or remove any part of the interface. Design: DESIGN.md §11.

## Sprint goal

Play the game exactly as today, except every panel, bar, label and toast is drawn by core's UI mod on the new engine, in the system font, with names that no longer overlap, and `wildlife_plus` proves a mod can change the HUD.

## Plan

- **Days 1–2, foundations:** spike, system font and text, theme tokens.
- **Days 3–5, engine:** node tree, layout and drawing; layers and input routing; the client-only UI VM.
- **Days 6–8, parts:** component kit, anchored layer, docking shell.
- **Days 9–11, the port:** move the HUD into `mods/core/ui/`, delete the Rust HUD, autotest by node id.
- **Days 12–14, hackability:** mod operations with conflict reports, hot reload, devtools, UI modding guide.

## Definition of done

- The Rust HUD code is deleted; the client draws only the world and the UI tree.
- The client autotest passes against node ids, and screenshots are reviewed.
- The whole UI costs under 1 ms per frame with 30 colonists, measured in the profiler.
- `wildlife_plus` changes the HUD through `ui.extend`, and a deliberate conflict is reported.
- Every item below is closed or explicitly moved out, with a note saying why.

## Out of scope (follow-ups filed)

Floating windows and a saved layout, sliders/text input/tables, a keybinds file and command palette, and an icon set. Each is filed in the milestone that first needs it.

## Risks

- **Text rendering** on macroquad is the unknown; the spike decides before anything depends on it.
- **Two Luau VMs** double the sandboxing surface. The UI VM uses the same rules as 0140, and the port must not slip in any mutation path.
- **Autotest churn:** tests move from screen coordinates to node ids in the port, not after.

## 2026-09-23

Sprint closed 2026-09-23, ahead of its 2026-10-09 date. Definition of done: the Rust HUD is deleted (draw.rs draws the world and a draw list); the autotest passes against node ids on macOS and Linux CI; the whole UI costs 0.12 ms median / 0.66 ms rebuild frames with 30 colonists; wildlife_plus changes the HUD through ui.extend and conflicts are reported (tested). Every item is closed; 0163's Windows check moved to 0156. PR #5.
