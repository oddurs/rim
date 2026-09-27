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
| `size`, `weight`, `wrap`, `tracking` | Text: `size` token (`caption`, `small`, `body`, `heading`, `title`), `weight` (`regular`, `strong`), wrap to width, letter spacing (a `tracking` token or em) |
| `hover`, `press`, `focus` | Colour changes for each state: `{ bg = "surface_hover" }` |
| `on_click`, `on_right_click` | Functions called when clicked |
| `on_hover` | Function called once when the pointer comes onto the node |
| `menu` | A context menu subject, `{ kind = ..., id = ... }`: a right-click here opens its menu |
| `at`, `on_key`, `on_outside` | Popup roots: where to open (`{ x, y }`), every named key while open, a press anywhere else |
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
`panel`, `row`, `col`, `label`, `caption`, `section`, `reading`, `button`,
`toggle`, `bar`, `tabs`, `list`, `menu`, `toast`, `bubble`, `divider` and
`heading`. They take their colours and sizes only from theme tokens, so they
follow whatever theme is loaded.

Spacing has named roles on the 4 px grid, so a gap says what it separates:
`hair` (2) between a label and its sub-line, `tight` (4) between rows in a
list, `item` (8) between separate things, `inset` (12) at a panel's sides,
`panel` (8) between panels in a region and `group` (16) between groups in a
sheet. `kit.panel` pads `item` top and bottom and `inset` at the sides unless
you give `pad`. A panel's header is `kit.section(label, right?)`: a caption,
and on the right whatever belongs to the whole section (a count, a link).
`kit.reading(name, value)` is a caption over a value, the way the tile
readout shows each field:

```lua
local kit = require("@core/ui/kit")
ui.define("my_mod:herd", function(view)
	return kit.panel({ gap = "item" }, {
		kit.section("Herd", kit.label("12", { size = "caption", color = "faint" })),
		ui.row({ gap = "inset", kit.reading("Grazing", "8"), kit.reading("Hungry", "4") }),
	})
end)
```

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
| `view.fields()`, `view.hover()`, `view.overlay()` | Field layers; what's under the cursor (with `values`, each field's `label` and `value`); the active overlay |
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

### Context menus

One menu serves everything the player can point at
([`menus.luau`](../../mods/core/ui/menus.luau), drawn with `kit.menu`). Give
a node a subject and a right-click on it (or anything inside it) opens the
menu for that subject:

```lua
ui.define("my_mod:crate", function(view)
	return ui.row({ menu = { kind = "my_mod:crate", id = 1 }, pad = "item", ui.text({ "A crate" }) })
end)
ui.mount("top", "my_mod:crate")
```

The rows come from providers, registered by kind, and a mod adds to a menu
without owning it:

```lua
local menus = require("@core/ui/menus")
menus.add("thing", {
	id = "my_mod:check_fuel",
	label = "Check fuel",
	group = "manage", -- do | work | manage | damaging
	applies = function(ctx) return ctx.subject.def == "my_mod:kiln" end,
	disabled = function(ctx) return #ctx.actors == 0 and "no one selected" end,
	run = function(ctx) ui.open("my_mod:kiln") end,
})
```

`ctx` holds the subject's `kind`, `id`, its table (`subject`, found by the
kind's `resolve`), and `actors`, the selected ids. `menus.kind(name, {
resolve, caption, actor })` describes a kind once: how to find its subject,
and what the menu's caption says. A provider can give several rows with
`rows = fn(ctx)` instead of `label` and `run`.

Rows sort into four groups, always in this order: **do** (act now),
**work** (mark for work), **manage** (look and set), **damaging** (take
away). A hairline separates the groups. One row is primary, marked with a
dot: the first flagged by its provider, else the first enabled row that
acts now, and never a damaging one. A disabled row stays, with its reason
("no axe") where a key would be.

The menu opens 4 px from the pointer and flips to stay on screen. While it
is open it takes the keys: the arrows move, Enter picks, 1–9 pick the nth
row that can run, a letter jumps, Escape closes. A click outside closes it,
and pressing on a subject, dragging onto a row and letting go picks that
row. Providers run once, when it opens; a closed menu builds nothing. Rows
are verbs first and three words at most, and name the object only when the
caption doesn't.

The map is a subject too, kind `tile` (id `"x,y"`, or `"x,y,<creature>"`),
opened by a held right-click or a plain one where nothing safe can be done
([`orders.luau`](../../mods/core/ui/orders.luau)). Its rows come from the
sim through `view.orders(x, y, on)`: every order the selected colonists
could be given there, damaging ones (deconstruct, felling an unmarked tree)
in the last group. `act.order(key, x, y, on)` gives a row by its key. A
plain right-click only ever gives a safe order.

Every order given leaves `view.last_order()` (`label`, `age` in seconds)
for core's toast (`core:undo`): for five seconds it says what the order
was, with Undo, and Cmd/Ctrl+Z (`act.undo()`) takes the newest back. Undo
is a command: colonists still on the ordered job stop it, and a mark the
order put on something (a deconstruct's) goes. What the order already did
stays done.

Core's kinds, each registered in the file that draws it, so a mod can add
rows to any of them:

| Kind | Where it opens | Core's rows |
|---|---|---|
| `colonist` | People column, inspector title | Select, centre on, draft or undraft, work priorities… |
| `tool` | A tile or row in any dock tray | Place (or use), find in tray |
| `zone` | Zones tray, the Stockpiles sheet | What it takes…, take everything, take nothing |
| `alert` | The alerts panel | Go to (when it's about someone), hide for today |
| `message` | A news line | All news…, hide here |
| `tile` | The map | Every order the selected colonists could be given there |

The engine side is general: any node's `menu` subject calls the handler
`ui.on_context` set (core's menus), and a `popup` layer places a root at
its `at`, taking keys through `on_key` and hearing a press elsewhere
through `on_outside`.

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

### Stores

A stockpile or a container, selected, shows two tabs (DESIGN.md §4f), in
[`storage.luau`](../../mods/core/ui/storage.luau): **Contents**, what it
holds as item tokens (a container's slots, empty ones dashed; a
stockpile's totals by thing and material), and **Accepts**, what it takes
(the category tree, materials and condition, with a line saying it all).
A stockpile is selected by clicking one of its empty cells or its row in
the Zones tray; `view.selected_zone()` says which, and the inspector's
`sel` is then `{ kind = "zone", id, name, cells, level }`. A container is a
thing whose `sel.store` is true.

`view.store(ref)` is everything the tabs paint in one read, and
`act.store_filter(ref, edit)` and `act.store_level(ref, level)` change it.
`ref` is a stockpile's id, `{ zone = id }` or `{ thing = id }`; `edit` is
`{ thing | category | material = id, on = bool }`, `{ min, max }` (percent
of condition) or `{ all = bool }`. Every change is a command, so it
replays. Copy and paste (Ctrl+C, Ctrl+V) carry a filter between stores;
comma and full stop lower and raise a store's level.

The Contents tab sorts by an order a mod can add to:

```lua
local storage = require("@core/ui/storage")
storage.sorter({
	id = "my_mod:weight",
	label = "Weight",
	order = 50,          -- core's run 10 to 40: Category, Count, Value, Name
	key = function(row, view) return -row.count end,
})
```

A key is a number or a string; rows sort by it, then by label. Orders run
in the UI's VM, so none can change the game.

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

A screen is a window declared with `sheet = true`. The engine floats it at
fixed proportions of the screen: centred, as big as its `w` and `h` ask up
to 60% of the width and the height between the bars, and set a little
above the middle. Nothing docked moves it, so a hover readout or an open palette
never nudges it, and at 1280 wide and up it leaves the side columns clear.
One sheet is open at a time: opening another closes it, and `ui.sheet()`
names the open one. A sheet isn't dragged or resized; ordinary windows
still are, and stay open over a sheet. Every window casts a soft shadow
and fades in as it opens, an offset of its draws that costs no layout.

Register a screen and it gets a key binding (so the command palette finds
it) and a tab at the right of the top bar (`core:topbar.views`), beside core's Work (P),
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

### Work lenses

The Work screen shows one lens at a time: core's Board and Person, and any a
mod adds (DESIGN.md §4d). A lens draws from the same `view.board()` data and
writes only through `act`, so it can't disagree with the board. A headcount
view or a labour list is a lens, not a replacement screen:

```lua
local work = require("@core/ui/work")
work.lens({ id = "my_mod:crews", label = "Crews", order = 40, draw = function(view, board)
	return build_crews(board)
end })
```

Tabs are `core:work.lens.<id>`, sorted by `order` (core's are 10 and 20;
the default is 100). Registering the same id again replaces the lens and
keeps its place.

### The dock

The bottom edge holds verbs only (`core:toolbar`, in
[`toolbar.luau`](../../mods/core/ui/toolbar.luau)): Select, then Orders (Q),
Build (B) and Zones (Z). The client gives each row of `view.tools()` a
`category` and a `group` (a buildable's group is its `build.menu`), and a
buildable its `cost` in the material it would use, `work` and `hp`. A mod's
new designation, building or menu shows up in the right tray with no UI
code.

A verb raises a tray (`core:dock.tray`, built with `kit.tray`) that reads
in the order the player decides. Build has a group rail
(`core:dock.groups.<group>`, stepped with `[` and `]`), its things as tiles
(`core:toolbar.<key>` in `core:toolbar.buttons`, each with a number key),
and a card for the thing under the pointer: its cost, work, hit points and
the materials it can be made of (`core:stuff.<id>`). Orders lists the orders
with how many things each has marked (`view.marked()`); Zones lists the zone
tools and the colony's stockpiles. `/` finds a thing by name across groups.
Picking a thing folds the tray to a pill (`core:dock.pill`) that says what
the next click places, in what, and at what cost. The number keys pick in
an open tray; 1 to 3 set the speed when none is open.

Only the open tray is built, at a fixed height so the card changing under
the pointer never moves the tiles. For mods, the bar has `core:dock.right`,
and each tray has `core:dock.palette.<category>` at the foot of its list,
built only while that tray is open. Core's stockpile list is a button there:

```lua
ui.extend("core:dock.palette.zones", function(view)
	return kit.button({ id = "core:zones.open", label = "All stockpiles…", on_click = function()
		ui.toggle("core:zones")
	end })
end)
```

Escape is the binding `core:escape`: it stops placing (back to the tray),
else closes the tray, else closes the open sheet, else clears the selection.

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

## Items

Every item on every screen is one component, the **item token**
(DESIGN.md §4f): the thing's own world look tinted by its material, the
count bottom right, a notch when the stack is full, and a condition bar
when it is worn. A mod's new item gets a token without an icon.

```lua
local kit = require("@core/ui/kit")
ui.define("my_mod:shelf", function(view)
    return kit.row({ gap = "xs" }, {
        kit.item({ thing = "core:wood", count = 140, limit = 75 }),   -- full notch
        kit.item({ thing = "core:stone", count = 12, hp = 0.6, size = "l" }),
        kit.item({ state = "empty" }),                                -- an empty slot
    })
end)
ui.mount("windows", "my_mod:shelf")
```

`size` is `"s"` (20, no count), `"m"` (32) or `"l"` (48). `state` is
`"incoming"` (a hauler has reserved it), `"leaving"` (its store no longer
takes it) or `"empty"`. Counts read exact to 999, then `1.0k` and `12k`;
a count of one is never drawn.

Underneath, `view.look(thing, made_of?)` returns an index into the looks
the engine has made, and a node `{ kind = "token", look, count, full, hp,
state }` paints it. A grid cell takes the same fields as `token = {...}`,
so a contents grid of two hundred items is still one node. A look's
sprite layers draw as a fill in their colour: the UI can't reach the
world's atlas.

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

Sections are `space`, `text`, `weight`, `shape`, `color`, `font`,
`leading` and `tracking`. `leading` is line height over text size: `line`
for a single line and `wrap` for text that wraps (core: 1.3 and 1.4).
`tracking` is letter spacing in em, by name (core's `caption` is 0.06). Core
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
