---
id: e856a44d-5cf7-4810-9658-b4d11be726c7
title: The mine order as a cut line
type: feature
status: backlog
milestone: rock-face
depends_on:
- 8fea2eef-1909-4e97-8960-4888305d0003
- bb769d00-9370-42e9-90a1-00adb1add366
- cd59b515-b98c-4f24-92f8-ec55161d22aa
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: s
layer: client
area: ui
---

## Problem

The mine designation is `#b0a89e`, grey on grey rock. Every marked cell looks the same whether a miner can reach it, it waits behind the face, or nobody holds the tool. The worked cell's label ("Mine · 60% · 1 s") sits on the map. DESIGN.md §6g, "The work".

## Proposal

- `designation.mine.color = "#e9d44a"` (survey yellow). A designation field `mark = "cut"` draws the marked region's extent: a dashed line along its outer edges on a dark keyline, over a 14% wash. Below 10 points a cell, the dashes become a 28% wash.
- A mark at each marked cell's designation corner: a solid dot when a miner can reach it (it has a standable 4-neighbour), a hollow dot when it waits behind the face, and an amber triangle when no colonist holds the tool its harvest `requires`. The hover card says which tool ("needs pounding: a stone maul").
- The preview crosses out cells that can't be mined (bedrock), with the reason.
- The worksite's progress becomes a hairline bar under the cell; the text label moves to the hover card.
- Chalkline (DESIGN.md §6f) owns the designation corner and the keyline (d83192ed), the preview answer from the sim (cd59b515) and the designate highlight the preview ring uses (bb769d00). This item draws mining's marks with them.

## Acceptance criteria

- [ ] A marked region draws one dashed outline, not one per cell (autotest shot)
- [ ] Workable, queued and tool-gated cells draw a solid dot, a hollow dot and a triangle (autotest checks by pixel at the corner)
- [ ] Removing the only maul turns the marked granite cells' dots into triangles on the next frame
- [ ] Dragging over bedrock marks nothing and shows a cross (autotest)
