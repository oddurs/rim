---
id: cc289956-8b67-429b-8f9a-5a98e651398e
title: 'How figures reach the GPU: distance-field quads, tessellation or pre-rendered frames'
type: spike
status: backlog
milestone: people
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
pillar:
- performance
effort: s
layer: client
area: render
---

## Question

How should every pawn be drawn in one call? Today `draw.rs` `pawns()`
issues immediate-mode discs and ring lines per pawn. The plan (DESIGN.md
§6h) needs up to 14 rotated, outlined primitives per pawn and 200 pawns in
0.4 ms of CPU, on the reference ultrabook's integrated GPU. On macOS's GL
each pass on the window's framebuffer is expensive (DESIGN.md §8), so the
answer has to stay inside the world target's single pass.

## Options

- **Distance-field quads.** One mesh of quads, one material; each quad
  carries its shape (disc, rounded box, segment, ring), size, turn, fill and
  ink, and the fragment shader draws the shape with antialiased edges. Crisp
  at every zoom and angle; sprites share the quad format through the world
  atlas.
- **Tessellated paths.** Build triangles for each ellipse and outline on the
  CPU into the existing batch. No new shader, more vertices and CPU.
- **Pre-rendered frames.** Render each body at N angles × M phases into an
  atlas and blit. Cheapest per frame; appearance multiplies the atlas, and
  outlines blur when scaled.

Leading option: distance-field quads, if macroquad's material path accepts
the per-instance data without a second pass.

## Decision

## Acceptance criteria

- [ ] A throwaway branch draws 200 and 1,000 figures of the §6h human with each viable option, measured with `rim --bench-render` on macOS and Linux: CPU per frame, draw calls, GPU time where available
- [ ] The decision, with the numbers, recorded here and in DESIGN.md §6h
- [ ] Follow-up items updated to the chosen path
