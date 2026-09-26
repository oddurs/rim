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
| `title` | The title screen, before a colony exists (see below) | |

Panels in a region stack by `order` and never overlap. `refresh` says how
often a mounted component is rebuilt when no input or game change forces
it: `"fast"` (the default, twenty times a second), `"slow"` (four times, for
a top bar or a clock) or `"frame"` (every frame, for a live readout or an
animation). Give a live readout a fixed `w` and its neighbours keep their
layout while it changes; only a change of size lays the panel out again. The world stays visible
behind translucent panels, and clicks on empty screen go to the world.

A side column's `start` stack gives way before its `end` stack: a panel
there that says `minh = 0` and holds a `scroll` or `list` shrinks to the
room left and scrolls, instead of running under the panels at the bottom.
Core's people column does this above the inspector.

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
| `kind` | `row`, `col`, `text`, `spacer`, `scroll`, `anchored`, `grid`, `list`, `image` |
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
| `handle` | Window chrome: `move`, `resize` or `close` (see Windows) |
| `src`, `tint` | Image: `src = "mod:name"` names a PNG under that mod's `ui/img/`; `tint = true` draws it in the text colour (see Images) |
| `value`, `placeholder`, `on_change`, `on_submit` | Text input (see Text input and sliders) |
| `on_drag` | Called with `(fx, fy)`, fractions across the node, while the pointer is held on it |
| `disabled` | Dims the node and everything inside it, and ignores clicks |

A typo in a property or token name is an error. It appears as a red box where
the component would be, naming your mod, and the rest of the UI keeps running.

## The kit

`require("@core/ui/kit")` gives you the components core is built from:
`panel`, `row`, `col`, `label`, `button`, `toggle`, `bar`, `tabs`, `list`,
`menu`, `toast`, `bubble`, `divider` and `heading`. They take their colours
and sizes only from theme tokens, so they follow whatever theme is loaded.

Press **F12** and then **Kit gallery** to see every component in every state.

`kit.table` shows records in columns over a virtual list: give it `rows`
(an array of records) and `columns` with a `key` or `value` each, and a
click on a header sorts by that column; give it `count` and `row(i)` to
build cells by hand. `kit.input` and `kit.slider` are the text and drag
controls; their state is the engine's (see Text input and sliders).

### The title screen

Before the player picks a colony there is no game, and the only layer built
is `title`. In a game it is never built. Core's title screen
(`mods/core/ui/title.luau`) is a component like any other, and a mod can
replace it. It reads `view.saves()`, the player's saves newest first:
`path`, `file`, `day`, `colonists` (founder first), `age` (seconds since it
was played) and `error`, which says why the save can't be read or why
loading it failed. `act.load(path)` plays one and `act.new_colony()` starts
a new one. The world views see an empty world here: no colonists, tick 0.

## Reading the game: `view`

Every function in `ui`, `act` and `view`, with its types, is in the
[UI API reference](api-ui.md). For editor completion see
[Editor setup](scripting.md#editor-setup).

| Call | Returns |
|---|---|
| `view.day()`, `view.clock()`, `view.tick()`, `view.hour()` | Game time |
| `view.paused()`, `view.speed()`, `view.wealth()` | Colony state |
| `view.colonists()` | Pawn tables: `id`, `name`, `label`, `health`, `job`, `drafted`, `needs`... |
| `view.people()` | Every colonist, lean: `id`, `name`, `job`, `health`, `drafted`, `idle`, `selected`; for a list of them |
| `view.pawn(id)`, `view.thing(id)`, `view.selected()` | One pawn; one thing (label, count, hp, material, what stops its work); the selection, a pawn or a thing |
| `view.count_pawns(faction)` | Living pawns of `"player"`, `"hostile"` or `"wild"` |
| `view.visible_pawns()` | Pawns on screen, for anchored labels |
| `view.messages(n, skip?)`, `view.message_count()` | The message log, newest first (`text`, `kind`, `age`, `day`); `skip` pages back through it |
| `view.events(since_tick)` | Recent joins, deaths and departures |
| `view.fields()`, `view.hover()`, `view.overlay()` | Field layers; what's under the cursor; the active overlay |
| `view.tools()`, `view.hint()` | Toolbar entries; what a right-click would do |
| `view.profile()`, `view.stats()`, `view.mods()`, `view.warnings()` | Profiler and load information |
| `view.date()` | `{ year, season, day, day_of_year, year_days }` |
| `view.ambient(field)`, `view.explain(field)` | A field's outdoor value; each term and push that makes it up |
| `view.data(key)` | Data a sim script stored with `rim.set_data`, by its full key: the weather plugin's forecast is `"weather:forecast"` |
| `view.ticks_per_day()` | For turning ticks into hours |
| `view.time()` | Wall-clock seconds, for animation |
| `view.saves()` | The player's saves, on the title screen |

## Doing things: `act`

`act.select(id)`, `act.focus(id)`, `act.tool(key)`, `act.speed(n)`,
`act.toggle_pause()`, `act.draft(id, on)`, `act.cycle_overlay()`,
`act.set_overlay(index)`, `act.toggle_profiler()`, `act.toggle_devtools()`,
`act.send(name, table)`, `act.advance(hours)`, and on the title screen
`act.load(path)` and `act.new_colony()`.

`act.send(name, table)` sends an event to your mod's own sim scripts
(`"my_mod:do_thing"`, heard with `rim.on` there). It travels as a player
command, so it replays and stays in lockstep; a mod can only send under its
own name. `act.advance(hours)` runs the game forward (devtools).

Actions are queued and applied by the client, exactly like a key press.
Toolbar keys are the tool and a def id, like `"designate:core:chop"` and `"build:core:wall"`.

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
readouts), `core:inspector.sections` (sections in the colonist
inspector) and `core:inspector.thing` (sections in the inspector of a
selected thing: a station's bills, a tool's wear).

### Small screens and UI scale

`view.compact()` is true when the screen is under 1440 logical pixels wide.
Logical pixels count the player's UI scale, so a big interface on a big
screen is compact too. `view.screen()` returns logical pixels. Core's
panels take narrower widths when compact: people in one line each, three
news lines, and no key line in the top bar. A mod's panel can do the same.

The player's UI scale goes on top of the display's, from 0.75 to 2:
Ctrl+= and Ctrl+- step it and Ctrl+0 resets it. It is saved as `ui_scale`
in the player's `settings.toml`, and `--ui-scale` on the command line
overrides it for a run. `view.ui_scale()` reads it and `act.ui_scale(s)`
sets it. Layout and hit testing both follow the scale.

### The inspector

The selected pawn or thing shows bottom left (`core:inspector`, in
[`inspector.luau`](../../mods/core/ui/inspector.luau)). Its tabs and the row
of actions under its title are two registries, and core's Overview, Skills
and Work tabs and its Draft (R) and Centre (C) actions go through them:

```lua
local inspector = require("@core/ui/inspector")
inspector.tab({
	id = "my_mod:mood",
	label = "Mood",
	applies = function(sel) return sel.kind == "pawn" and sel.player end,
	build = function(view, sel) return kit.label(mood_of(sel)) end,
})
inspector.action({
	id = "my_mod:rally",
	label = "Rally",
	key = "g",
	applies = function(sel) return sel.kind == "pawn" and sel.player end,
	run = function(sel) act.send("my_mod:rally", sel.id) end,
})
```

`sel` is the pawn (`view.pawn`) or thing (`view.thing`) table with `kind`
set. A label may be a function of it, an action's `active(sel)` lights its
button, and `order` places either (core's run 10, 20, 30; the default is
100). An action's key is a binding with the action's id: it runs on the
selection when the action applies, and the command palette lists it. Tabs
are `core:inspector.tabs.<id>`, shown when more than one applies; actions
are `core:inspector.action.<id>`.

Several colonists can be selected: shift-click adds or removes one (on the
map or in the people column), and a drag with the Select tool picks every
colonist in the box. `view.selection()` lists the ids and `view.selected()`
is the first; `act.select(id, true)` is a shift-click. The inspector then
sums the group up and shows the actions declared with `group = true` that
apply to every member. Such an action's `label`, `active` and `run` get the
whole list as a second argument, and `run` is called once per member, so
core's Draft drafts the lot unless every one already is. A right-click
orders every selected colonist it means something to.

### People

The colonists run down the left edge (`core:colonists`, in
[`people.luau`](../../mods/core/ui/people.luau)), one button each with the
id `core:colonists.<name>`. The column changes density with the colony: a
card each up to 8, a line each from 9, and from 20 lines grouped as
Drafted, Hurt, Idle and Working. It is a virtual `list`, so only the lines
in view are built. A mod adds a badge to every row through the module:

```lua
local people = require("@core/ui/people")
people.badge(function(p)
	return p.asleep and kit.label("z", { size = "small", color = "muted" })
end)
```

### Now: alerts and news

The right edge is what needs the player. `core:alerts` lists standing
problems, most severe first; `core:messages` has the newest news, with a
button (and N) for the whole log in the `core:news` window, a virtual list
however long the log grows.

An alert is a check registered with core's module. Core's own alerts (a
colonist badly hurt, a need run dry, idle hands, no stockpile) are
registered the same way, as are the weather mod's:

```lua
local alerts = require("@core/ui/alerts")
alerts.add({
	id = "my_mod:no_salt",
	severity = "warn", -- "bad", "warn" or "info"
	check = function(view)
		if not has_salt(view) then
			return "No salt: meat will spoil" -- or { text = ..., subject = pawn_id }
		end
	end,
})
```

A check returns nothing when all is well. Given a `subject`, the alert
selects and centres on it when clicked; `on_click` in the definition
overrides that. Checks run at most four times a second however often the
panel is rebuilt, so one may read the world freely, and a check that errors
shows as a bad alert naming it. Each alert's row has the id
`core:alerts.<id>`.

### Screens

A screen is a window declared with `sheet = true`. The engine places it in
the band between the docked columns (the node `rim:center`), as wide as its
`w` allows and as tall as the band, so it never covers the people, the
alerts or the bars. One sheet is open at a time: opening another closes it.
A sheet isn't dragged or resized; ordinary windows still are, and stay open
over a sheet.

Register a screen and it gets a key binding (so the command palette finds
it) and a button at the right of the dock, beside core's Work (P),
Stockpiles and News (N):

```lua
local screens = require("@core/ui/screens")
ui.window("my_mod:herds", { title = "Herds", w = 640, h = 600, sheet = true }, function(view)
	return build_herds(view)
end)
screens.add({ id = "my_mod:herds", label = "Herds", key = "h" })
```

Buttons are `core:screens.<id>`, sorted by `order` (core's are 10, 20
and 30; the default is 100) and then by when they were added.

### The dock

The bottom bar (`core:toolbar`, in [`toolbar.luau`](../../mods/core/ui/toolbar.luau))
files every tool by category: Orders (Q), Build (B) and Zones (Z). The client
gives each row of `view.tools()` a `category` and a `group`; a buildable's
group is its `build.menu`. A mod's new designation, building or menu shows up
in the right palette with no UI code, and the dock stays one row wide however
many mods are loaded.

Only the open palette is built. A palette with more than one group shows its
groups as tabs (`core:dock.groups.<group>`); tool buttons keep the ids
`core:toolbar.<key>`, in the row `core:toolbar.buttons`. For mods, the bar
has `core:dock.right`, and each palette has `core:dock.palette.<category>`
after its tools, built only while that palette is open. Core's stockpile
list is a button there:

```lua
ui.extend("core:dock.palette.zones", function(view)
	return kit.button({ id = "core:zones.open", label = "All stockpiles…", on_click = function()
		ui.toggle("core:zones")
	end })
end)
```

Escape is the binding `core:escape`: it drops the tool and closes its
palette, else closes a palette opened by hand, else clears the selection.

If two mods replace or remove the same id, that's reported as a conflict
naming both, and load order decides which wins. Operating on an id nobody
defines is reported as a warning. Both show in the F3 profiler.

## Windows

A window is a panel the engine moves, resizes, stacks and remembers. Declare
one with its default size and the component shown inside; open it from any
handler:

```lua
local kit = require("@core/ui/kit")

ui.window("my_mod:settings", { title = "Settings", w = 420, h = 320, resizable = true }, function(view)
	return ui.col({ gap = "s", ui.text({ "Settings live here" }) })
end)

ui.define("my_mod:settings_button", function(view)
	return kit.button({ label = "Settings", on_click = function()
		ui.toggle("my_mod:settings")
	end })
end)
ui.mount("top", "my_mod:settings_button", { order = 50 })
```

`ui.open`, `ui.close` and `ui.toggle` take the window's id; `ui.is_open` reads
it. `open = true` in the options shows the window the first time it is seen.
A mod never moves a window: the player does, and the engine keeps where each
one is, per player and per machine, beside the client's settings (never in a
save game). The record is keyed by the window's id, so it survives a restart
and a mod update that changes the default size. Two mods declaring one id is
a conflict, reported like a replaced component.

Core draws the chrome (`kit.window`): a title bar that drags, a close button,
the body in a scroll area, and a resize grip when `resizable`. A theme mod
can register its own with `ui.window_chrome`; nodes marked `handle = "move"`,
`"resize"` or `"close"` are what the engine routes.
## Images

PNGs under a mod's `ui/img/` are images named `mod:stem`. An image node
draws one from the atlas the text already uses, at its own size unless `w`
and `h` say otherwise:

```lua
local kit = require("@core/ui/kit")

ui.define("my_mod:badge", function(view)
	return ui.row({ gap = "s", align = "center",
		kit.icon("core:check", "m"),
		kit.icon("core:check", "l", { color = "good" }),
		kit.image("core:check"),
	})
end)
ui.mount("top", "my_mod:badge", { order = 60 })
```

`tint = true` (what `kit.icon` sets) draws the image as a mask in the text
colour, so a white-on-transparent icon follows the theme. Without it the
picture keeps its own colours. Ship `stem@2x.png` beside `stem.png` for
dense displays: the engine picks it from 1.5x up. No SVG, no nine-slice, no
animation. A `src` nobody ships is a named warning and a red placeholder
where the image would be, so the rest of the panel still works.

## Text input and sliders

The tree is rebuilt twenty times a second, so a caret cannot live in a
script. An `input` node's buffer, caret and selection are the engine's, keyed
by the node's `id`: what the player typed is there after every rebuild, after
a hot reload, and until the mod reads it back.

```lua
local kit = require("@core/ui/kit")

ui.define("my_mod:rename", function(view)
	return ui.row({ gap = "s", align = "center",
		kit.input({ id = "my_mod:name", placeholder = "New name", on_submit = function(text)
			ui.set_state("my_mod:name", text)
		end }),
		kit.slider({ id = "my_mod:volume", value = ui.state("my_mod:volume", 0.5), on_change = function(v)
			ui.set_state("my_mod:volume", v)
		end }),
	})
end)
ui.mount("top", "my_mod:rename", { order = 70 })
```

Click an input to focus it; typing, Backspace, Delete, the arrows, Home and
End edit; Shift with an arrow selects; Enter calls `on_submit(text)`; Escape
gives the keyboard back to the game. `on_change(text)` runs after every
edit, and `on_key("up" | "down")` hears the keys the buffer has no use for,
so a list under a query can move its selection. While an input has focus
the game sees no keys at all. `value` is what the box holds until the player
types; `ui.set_input(id, text)` replaces it from a script.

A slider is any node with `on_drag`: while the pointer is held on it, the
handler gets `(fx, fy)`, the pointer's position across the node as fractions.
`kit.slider` draws a bar and reports `fx`.

## Keys and the command palette

An action a mod adds should be reachable without the mod drawing a button.
`ui.bind` gives it a name, a default key and a label; it then fires from the
key (when no text input is typing) and appears in the command palette
(Ctrl+K), which lists every binding and runs the one the player picks:

```lua
ui.bind("my_mod:muster", { key = "m", label = "Muster everyone" }, function()
	act.send("my_mod:muster")
end)
```

Keys are named plainly: letters and digits as themselves, `space`, `enter`,
`escape`, `tab`, `f1`..`f12`, the arrows, `home`, `end`, and punctuation as
typed; modifiers go in front as `ctrl+`, `alt+` and `shift+` (`cmd` counts as
`ctrl`). Two mods binding one id or one key is reported like a replaced
component, and the later mod wins. The player's own keys live in
`keybinds.toml` beside the UI layout, per player and per machine:

```toml
[keys]
"core:pause" = "p"
"my_mod:muster" = "ctrl+m"
```

A binding with no `key` is in the palette alone, until the player gives it
one in `keybinds.toml`: core's render scale commands are.

Core's own keys (pause, speed, overlay, draft, devtools) are bindings too,
so they can be rebound the same way. `ui.run(id)` runs a binding from a
script, and `ui.focus(id)` hands a text input the keyboard.

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
	return ui.col(out)
end)
ui.mount("anchored", "my_mod:tags")
```

A positive `offset` places the node below the anchor and a negative one
above it. When labels collide, the higher `priority` keeps its place and
the others step away. If there's no room at all, the lowest-priority label
is dropped: labels never draw on top of each other.

An anchored label follows its pawn every frame, however the camera moves,
without your component rebuilding: the engine moves what it drew.

### Speech

Pawns talk in speech bubbles. A line comes from one of three places:

- **A need running low.** Give a need def a `say` table and a pawn who
  can talk says one of its lines as the need drops below `below`, once per
  crossing:

  <!-- not a sample -->
  ```toml
  say = { below = 0.2, lines = ["I'm starving.", "So hungry…"], ticks = 600 }
  ```

- **A sim script**, on any event: `rim.say(pawn_id, text, ticks?, priority?)`.
  Core's raids have their leader call out as they arrive.
- **The UI itself**: core greets a colonist who joins.

`view.speech()` lists the lines up now (`id`, `text`, `age` from 0 to 1,
`priority`). Core's `core:labels` shows each pawn's most important line,
and at most six bubbles at a time, most important first. Speech is only
presentation: the sim never reads it back, it isn't saved, and a line
is cut at 120 characters.

## Themes

Every colour and size comes from a token. Override any of them in your mod's
`ui/theme.toml`:

```toml
[color]
accent = "#ff9f43"

[space]
m = 10

[font]
family = "My Serif"   # empty uses the system UI font
```

Sections are `space`, `text`, `weight`, `shape`, `color` and `font`. Core
names Inter, which it ships. A mod can ship fonts too: TrueType, OpenType
or collections under its `ui/fonts/`, loaded before any theme is read, so
its theme (or another mod's) can name the family. A family that isn't
there falls back to the system UI font, with a warning. Sizes are
logical pixels: the engine multiplies them by the display's DPI and the
player's UI scale (`--ui-scale`). If two mods override the same token, it's
reported as a conflict and load order decides.

## The API version

The UI surface is versioned apart from the sim API: `types/ui.d.luau` and
`docs/modding/api-ui.md` are generated from the engine's registrations and
checked against them in CI, so neither can drift. A mod says which surface
its UI scripts were written against with `ui_api = "0.6"` in `mod.toml`;
before 1.0 every minor is breaking, and `rim check` refuses a mod targeting
a version the engine does not provide. It also refuses one naming a `ui.`,
`act.` or `view.` member that does not exist, by file and line.

## Devtools and hot reload

`cargo test -p rim_ui --test shots -- --ignored` writes PNGs of the HUD, the
palette, devtools, the gallery and the profiler to `target/ui-shots/`, drawn
by a small software rasteriser from the engine's draw list. It is the way to
look at a panel from a test, without a window.

- **F12** opens devtools. Point at anything to see its id, which mod made it,
  and its layout box. **Outlines** draws every layout box, the tree lists the
  whole UI with owners, and **Kit gallery** shows every component.
- **F3** shows how long each mod's UI code takes, alongside the simulation.
- **Hot reload:** save a UI script or theme while the game runs and it
  reloads within a second, without touching the simulation. If the new
  version doesn't load, the error appears on screen and the last good
  version keeps running.
