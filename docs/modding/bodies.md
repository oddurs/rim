# Bodies

What a creature looks like from above is its body: a few parts, drawn in
the plan's ink like furniture (DESIGN.md §6h). A body is client data, in a
mod's `ui/bodies.toml`, so it is the player's side of the game (§10). A mod
that redraws a creature, or draws its own, never touches the simulation,
and saving the file redraws everyone on the next frame.

Core draws its human, deer, wolf and hare in
[`mods/core/ui/bodies.toml`](../../mods/core/ui/bodies.toml);
`wildlife_plus` draws its boar the same way.

## A body

```toml
[[body]]
id = "human"                 # core:human
creatures = ["human"]        # what it draws: core:human; another mod's needs its prefix
parts = [
    { id = "shadow", draw = "disc", x = 0.05, y = 0.07, r = 0.26, squash = 0.88, color = "#00000042", world = true },
    { id = "hand_l", draw = "disc", x = -0.235, r = 0.052, color = "@skin", line = "light", detail = true, move = "swing", phase = 0.5 },
    { id = "torso", draw = "box", w = 0.5, h = 0.27, round = 1.0, color = "@torso", line = "light", move = "sway" },
    { id = "head", draw = "disc", y = -0.03, r = 0.115, color = "@skin", line = "light", move = "lean" },
]
sockets = { hair = "head", worn = "torso", hold = [0.0, -0.22], shoulder = [0.16, 0.0], tool = "hand_l" }
layers = ["body", "outer"]      # garment layers, innermost first
```

A body is written **facing north**: forward is `-y`, and positions and
sizes are in cells from the creature's centre. The renderer turns the whole
figure to where the creature is going.

Parts draw in the order listed, so list them bottom up: shadow, feet, hands,
torso, head, hair.

## Parts

| field | | |
|---|---|---|
| `id` | required | names the part in errors, sockets and devtools; unique in its body |
| `draw` | `"disc"` (default) or `"box"` | |
| `r`, `squash` | disc | radius across, and fore and aft as a share of it (0.2 to 5) |
| `w`, `h`, `round` | box | width, depth, and corners rounded by a share of the shorter half (1 is a capsule) |
| `x`, `y` | 0, 0 | where it sits, facing north, within a cell of the centre |
| `color` | `"@torso"` | `"#rrggbb"`, `"#rrggbbaa"`, or a channel (below) |
| `line` | none | an ink outline at one of the plan's weights: `hair`, `light`, `medium`, `heavy` |
| `world` | false | offset in the world's frame, so it falls the same way whatever the creature faces: a shadow. Its shape still turns with the creature |
| `detail` | false | drawn only close up (below) |
| `move`, `phase` | none | what a gait moves it with, `stride`, `swing`, `sway` or `lean`, and where in the stride |

Sizes are above 0 and at most one cell. An unknown field, a size out of
range or a colour that doesn't read is an error naming the file, the body
and the part, and that body is left out. In a running game the last good
bodies keep drawing and the error shows on screen.

## Channels

A colour starting with `@` is filled in per creature:

| channel | today |
|---|---|
| `@skin` | the creature's `color` |
| `@torso` | its outermost garment's colour, or its skin while it wears nothing |
| `@feet` | its colour, darker |
| `@hair` | its colour, darker still |

## Close up and far out

How much of a figure is drawn depends on how many points a cell is on
screen:

| | points a cell | draws |
|---|---|---|
| dot | below 10 | one disc in the torso's colour |
| silhouette | 10 to 20 | the parts without `detail` |
| full | from 20 | every part |

Every pawn's figure is drawn in one call, whatever mods add, so a body with
more parts costs a little more to fill and nothing more to submit.

A body's `layers` list the garment layers apparel is worn on, innermost
first (core's human: `["body", "outer"]`). The outermost layer a creature
wears colours `@torso`; a body with no `layers` shows the last garment put
on.

## Sockets

Where held and worn things attach: a part's id (its centre) or a point
`[x, y]`. Core's human names `hold`, `shoulder`, `tool`, `worn` and `hair`,
which carried items, tools and clothing will use.

## Conflicts

Two bodies that list the same creature are a conflict, reported in the load
warnings (F3); the later mod in load order draws it. A creature no body
lists is drawn as a disc in its colour, with a warning naming it.
