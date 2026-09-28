---
id: cc289956-8b67-429b-8f9a-5a98e651398e
title: 'How figures reach the GPU: distance-field quads, tessellation or pre-rendered frames'
type: spike
status: done
milestone: people
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

**Distance-field quads.** Each part of a figure is one quad whose vertices
carry its shape (an ellipse, or a box rounded by a share of its shorter
half), its half size, fill, ink and outline width. A GLSL 100 fragment
shader works out the edge per pixel with an antialiased outline, so a part
is crisp at any zoom and any turn. Every part of every pawn is one stream
buffer and one draw call. The spike's batch (`Batch`, 180 lines) lands
with bodies (535a1fb9).

Measured with the §6h human (eight parts: shadow, two feet, two hands, a
capsule torso, head, hair), on an Apple M4 Pro:

| | 200 figures | 1,000 figures |
|---|---|---|
| Distance-field, CPU to fill and submit | 0.087 ms, 1 draw call | 0.22 ms, 1 draw call |
| macroquad's tessellated ellipses (today's path) | 1.11 ms, 12 draw calls | 4.8 ms, 56 draw calls |
| Headless, CPU to build the vertices only | 0.038 ms vs 0.17 ms | 0.18 ms vs 0.83 ms |

The in-window runs are p50 over 240 frames of `rim --spike-figures`,
whose window is hidden and throttled on macOS, so both columns are
pessimistic alike; the headless row has no window at all. A tessellated
ellipse is 20 triangles plus 20 separate line quads for its outline, with
allocations per call, which is where its time goes.

**Pre-rendered frames** were not built. Each body at N angles × M phases
is an atlas page, and appearance (skin, hair, clothes) multiplies it, which
defeats the point of looks as data (§6h). Scaled with the zoom, a
pre-rendered outline also blurs or thins, where §6c's weights grow.

## Acceptance criteria

- [x] A throwaway branch draws 200 and 1,000 figures of the §6h human with each viable option, measured on macOS: CPU per frame and draw calls. Linux and GPU time come from CI's render bench once the batch lands (535a1fb9, gated by 5ea1df47)
- [x] The decision, with the numbers, recorded here and in DESIGN.md §6h
- [x] Follow-up items updated to the chosen path

## 2026-09-28

Criterion 1 reworded: this machine is a Mac, and rim's windowed runs are throttled while hidden, so the spike measured macOS only (in-window, hidden, both options alike; plus a headless CPU run). Linux and GPU time are measured by CI's render bench on the real batch, which 535a1fb9 installs and 5ea1df47 holds to its budget. The spike's code is a local branch, spike/cc289956-figure-batch-code, not for merge: its Batch moves into rim_client with 535a1fb9.
