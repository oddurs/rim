# Looks

A thing's `look` says how it is drawn: layers of a few primitives, painted
in order over its cell. The renderer knows the primitives and nothing about
what the thing is, so a loom, a fence or a hedge needs no engine change.
Core ships no art: out of the box the game is coloured rectangles and discs,
which is the fastest thing to draw and the easiest to read.

A thing with no look is its cell filled in its `color`. Nothing is ever
invisible.

## Layers

Positions and sizes are in cells, from the cell's top-left corner: `x = 0.5`
is the middle. Line widths are in screen points.

| `draw` | fields | draws |
|---|---|---|
| `fill` | `x`, `y`, `w`, `h` (0, 0, 1, 1), `min_px` | a rectangle |
| `outline` | `x`, `y`, `w`, `h`, `width` (1) | a rectangle's outline |
| `disc` | `x`, `y` (0.5, 0.5), `r` (0.4), `min_px`, `pulse` | a disc |
| `edges` | `width` (1.5) | the cell's border, left open toward joined neighbours |

Every layer also takes:

- `color`: `"#rrggbb"` or `"#rrggbbaa"`. Unset, the layer is the thing's
  colour, or its material's when it is built of one, so a marble wall
  arrives looking like marble.
- `shade`: multiplies the brightness (0.6 is a darker edge, 1.25 a
  highlight).
- `vary`: brightness varies per cell by up to this much, so a field of rock
  isn't one flat colour.

`min_px` keeps a small disc or a thin fill visible when zoomed out: a
radius, or a width and height, in points. `pulse` makes a disc's radius
flicker by up to that fraction.

A field that doesn't apply to the layer's `draw` is an error, and so is an
unknown one, so a typo can't pass for a default.

```toml
[[thing]]
id = "shrub"
label = "shrub"
color = "#4a6b35"
category = "plant"
natural = true
look.layers = [
    { draw = "disc", x = 0.55, y = 0.57, r = 0.36, color = "#00000040" },
    { draw = "disc", r = 0.34 },
    { draw = "disc", x = 0.42, y = 0.42, r = 0.14, shade = 1.3, min_px = 1.0 },
]
```

## Joining

Things with the same `look.join` label join up once they are built. An
`edges` layer leaves out the sides that face a joined neighbour, so a run of
wall reads as one wall with openings in it. Core's walls, windows and doors
all join as `"wall"`. A fence joins with fences:

```toml
[[thing]]
id = "fence"
label = "fence"
color = "#8a6a45"
category = "building"
blocks = true
look.join = "fence"
look.layers = [
    { draw = "fill", x = 0.4, y = 0.4, w = 0.2, h = 0.2 },
    { draw = "edges", width = 2.0, shade = 0.7 },
]
```

## States

`look.regrowing` is drawn instead of `layers` while a harvested plant grows
back (a bush picked clean). Blueprints, designations and stack counts are
drawn the same way for every thing, over its look.

## From API 0.3

`shape` is gone. A def that still has one is a load error that names it.
Core's old shapes are now looks in `mods/core/defs/`; copy the one you used.
