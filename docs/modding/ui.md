# Modding the interface

Everything you see in rim's interface is a mod. The top bar, the toolbar, the
colonist inspector, the message feed, the names above pawns and the devtools
are all Luau components in [`mods/core/ui/`](../../mods/core/ui/). Your mod
uses the same system, and can change any of those parts.

The design and its reasoning are in [DESIGN.md §11](../../DESIGN.md).

## Where UI code lives

Put UI scripts in your mod's `ui/` folder:

```
mods/my_mod/
  mod.toml
  ui/
    theme.toml        (optional: override theme tokens)
    my_panel.luau     (any number of scripts)
```

UI scripts run in their own sandboxed Luau VM on the player's machine. They
can **read** the game through `view` and **ask for things** through `act`, but
they can never change the simulation directly. That's why a UI mod can't
break a save or desync a co-op game, and why each player can run different
UI mods.

## A first component

A component is a function from `view` to a tree of nodes. Register it with
`ui.define`, then put it on screen with `ui.mount`:

```lua
local kit = require("@core/ui/kit")

ui.define("my_mod:clock", function(view)
	return kit.panel({ pad = "s" }, {
		kit.label("Day " .. view.day() .. " · " .. view.clock(), { weight = "strong" }),
	})
end)

ui.mount("right", "my_mod:clock", { align = "start" })
```

- Ids are namespaced: `your_mod:name`. Other mods use the id to find your part.
- Return `nil` to draw nothing (core's inspector does this when nobody is
  selected).
- Components rebuild on any input or change in client state, and otherwise
  20 times a second. Hover and press colours apply at draw time, so they're
  always instant.

To use the kit from your own mod, list `core` in `depends` in your
`mod.toml`. `require` only reaches mods you depend on.

### Where things go: regions and layers

`ui.mount(layer, id, opts)` puts a component on screen:

| Layer | What it's for | Options |
|---|---|---|
| `top`, `bottom` | Full-width bars at the screen edges | `order` |
| `left`, `right` | Columns at the sides; `align = "start"` stacks from the top, `"end"` from the bottom | `order`, `align` |
| `anchored` | Labels attached to pawns or cells (see below) | |
| `cursor` | A small label that follows the mouse | |
| `windows` | Panels in the middle of the screen | |
| `modal` | One centred panel above everything | |

Panels in a region stack by `order` and never overlap. The world stays visible
behind translucent panels, and clicks on empty screen go to the world.

## Nodes

A node is a plain table. `ui.row`, `ui.col` and `ui.text` just set its `kind`:

```lua
ui.define("my_mod:greeting", function(view)
	return ui.row({ gap = "s", pad = "m", bg = "surface", align = "center",
		ui.text({ "Hello", weight = "strong" }),
		{ kind = "spacer" },
		ui.text({ "right-aligned", color = "muted" }),
	})
end)
ui.mount("windows", "my_mod:greeting")
```

| Property | Meaning |
|---|---|
| `kind` | `row`, `col`, `text`, `spacer`, `scroll`, `anchored` |
| `id` | Namespaced id, so other mods can find this node |
| `gap`, `pad`, `padx`, `pady` | Spacing: a `space` token (`"s"`, `"m"`) or a number |
| `w`, `h` | Size: a number, `"fill"`, or a percentage like `"50%"` |
| `minw`, `maxw`, `minh`, `maxh` | Size limits |
| `grow` | Share of leftover space (spacers grow by default) |
| `align`, `justify` | `start`, `center`, `end`, `stretch`, `between` |
| `bg`, `border`, `color` | A `color` token (`"surface"`, `"accent"`) or `"#rrggbb[aa]"` |
| `radius` | A `shape` token or a number (defaults to `radius` on anything with a background) |
| `size`, `weight`, `wrap` | Text: `size` token (`small`, `body`, `heading`, `title`), `weight` (`regular`, `strong`), wrap to width |
| `hover`, `press`, `focus` | Colour changes for each state: `{ bg = "surface_hover" }` |
| `on_click`, `on_right_click` | Functions called when clicked |
| `tooltip` | Text shown after a short hover |
| `disabled` | Dims the node and everything inside it, and ignores clicks |

A typo in a property or token name is an error. It appears as a red box where
the component would be, naming your mod, and the rest of the UI keeps running.

## The kit

`require("@core/ui/kit")` gives you the components core is built from:
`panel`, `row`, `col`, `label`, `button`, `toggle`, `bar`, `tabs`, `list`,
`menu`, `toast`, `bubble`, `divider` and `heading`. They take their colours
and sizes only from theme tokens, so they follow whatever theme is loaded.

Press **F12** and then **Kit gallery** to see every component in every state.

## Reading the game: `view`

| Call | Returns |
|---|---|
| `view.day()`, `view.clock()`, `view.tick()`, `view.hour()` | Game time |
| `view.paused()`, `view.speed()`, `view.wealth()` | Colony state |
| `view.colonists()` | Pawn tables: `id`, `name`, `label`, `health`, `job`, `drafted`, `needs`... |
| `view.pawn(id)`, `view.selected()` | One pawn; the selected pawn's id |
| `view.count_pawns(faction)` | Living pawns of `"player"`, `"hostile"` or `"wild"` |
| `view.visible_pawns()` | Pawns on screen, for anchored labels |
| `view.messages(n)`, `view.events(since_tick)` | The message feed; recent joins, deaths and departures |
| `view.fields()`, `view.hover()`, `view.overlay()` | Field layers; what's under the cursor; the active overlay |
| `view.tools()`, `view.hint()` | Toolbar entries; what a right-click would do |
| `view.profile()`, `view.stats()`, `view.mods()`, `view.warnings()` | Profiler and load information |
| `view.date()` | `{ year, season, day, day_of_year, year_days }` |
| `view.ambient(field)`, `view.explain(field)` | A field's outdoor value; each term and push that makes it up |
| `view.data(key)` | Data a sim script stored with `rim.set_data` (the weather plugin's forecast is `"weather:forecast"`) |
| `view.ticks_per_day()` | For turning ticks into hours |
| `view.time()` | Wall-clock seconds, for animation |

## Doing things: `act`

`act.select(id)`, `act.focus(id)`, `act.tool(key)`, `act.speed(n)`,
`act.toggle_pause()`, `act.draft(id, on)`, `act.cycle_overlay()`,
`act.set_overlay(index)`, `act.toggle_profiler()`, `act.toggle_devtools()`.

Actions are queued and applied by the client, exactly like a key press.
Toolbar keys look like `"designate:chop"` and `"build:wall"`.

For small bits of UI state that should survive rebuilds, such as the selected
tab, use `ui.state(key, default)` and `ui.set_state(key, value)`.

## Changing other mods' UI

Any node with an id can be changed by any mod. Operations apply in load
order.

```lua
local kit = require("@core/ui/kit")

-- Add to a container (core leaves extension points for this).
ui.extend("core:topbar.right", function(view)
	return kit.label("wildlife " .. view.count_pawns("wild"), { id = "wildlife_plus:counter", size = "small", color = "good" })
end)

-- Take a part over completely.
ui.replace("core:hover", function(view)
	local h = view.hover()
	return h and kit.panel({}, { kit.label(h.terrain) })
end)

-- Decorate a part: receive its tree, return a new one around it.
ui.wrap("core:clock", function(inner, view)
	return ui.row({ gap = "s", inner, ui.text({ "☀" }) })
end)

-- Hide a part.
ui.remove("core:messages")
```

The first example is the shipped [`wildlife_plus`](../../mods/wildlife_plus/ui/wildlife.luau)
plugin. Core's extension points include `core:topbar.right` (top-bar
readouts) and `core:inspector.sections` (sections in the colonist
inspector).

If two mods replace or remove the same id, that's reported as a conflict
naming both, and load order decides which wins. Operating on an id nobody
defines is reported as a warning. Both show in the F3 profiler.

## Anchored labels

Return `anchored` nodes from a component mounted on the `anchored` layer to
attach UI to a pawn (`entity = id`) or a cell (`cell = { x, y }`):

```lua
local kit = require("@core/ui/kit")

ui.define("my_mod:tags", function(view)
	local out = {}
	for _, p in view.visible_pawns() do
		table.insert(out, { kind = "anchored", entity = p.id, priority = 2, offset = p.radius + 4,
			kit.label(p.name, { size = "small" }) })
	end
	return ui.col({}, out)
end)
ui.mount("anchored", "my_mod:tags")
```

A positive `offset` places the node below the anchor and a negative one
above it. When labels collide, the higher `priority` keeps its place and
the others step away. If there's no room at all, the lowest-priority label
is dropped: labels never draw on top of each other.

## Themes

Every colour and size comes from a token. Override any of them in your mod's
`ui/theme.toml`:

```toml
[color]
accent = "#ff9f43"

[space]
m = 10

[font]
family = "Inter"    # empty uses the system UI font
```

Sections are `space`, `text`, `weight`, `shape`, `color` and `font`. Sizes are
logical pixels: the engine multiplies them by the display's DPI and the
player's UI scale (`--ui-scale`). If two mods override the same token, it's
reported as a conflict and load order decides.

## Devtools and hot reload

- **F12** opens devtools. Point at anything to see its id, which mod made it,
  and its layout box. **Outlines** draws every layout box, the tree lists the
  whole UI with owners, and **Kit gallery** shows every component.
- **F3** shows how long each mod's UI code takes, alongside the simulation.
- **Hot reload:** save a UI script or theme while the game runs and it
  reloads within a second, without touching the simulation. If the new
  version doesn't load, the error appears on screen and the last good
  version keeps running.
