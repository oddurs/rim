-- The component kit: the parts every panel is built from.
--
--   local kit = require("@core/ui/kit")
--
-- Every colour and size here is a theme token, never a literal, so a theme
-- mod restyles every component at once. Components return plain node
-- tables; `ui.row`/`ui.col`/`ui.text` just tag a table with its kind.

local kit = {}

-- Build a node: `props` plus children, skipping nils and falses so an
-- optional child doesn't leave a hole (a hole would end the child list).
-- Fills in `props` itself rather than copying it: callers pass fresh tables.
local function node(kind: string, props: { [any]: any }, children: { any }?)
	props.kind = kind
	if children then
		local n = #props
		for _, c in children do
			if c ~= nil and c ~= false then
				n += 1
				props[n] = c
			end
		end
	end
	return props
end
kit.node = node

function kit.row(props, children)
	return node("row", props or {}, children)
end

function kit.col(props, children)
	return node("col", props or {}, children)
end

-- Text. `opts`: size ("caption"/"small"/"body"/"heading"/"title"), weight,
-- color, wrap, tracking (em, or a [tracking] token), id, minw, grow.
function kit.label(text: string, opts: { [string]: any }?)
	local o = opts or {}
	return {
		kind = "text",
		text,
		size = o.size,
		weight = o.weight,
		color = o.color,
		wrap = o.wrap,
		tracking = o.tracking,
		id = o.id,
		minw = o.minw,
		grow = o.grow,
	}
end

-- A caption: the small, tracked, uppercase label that names a section or
-- a reading. `opts` as kit.label's.
function kit.caption(text: string, opts: { [string]: any }?)
	local o = opts or {}
	return kit.label(string.upper(text), { size = "caption", tracking = "caption", color = o.color or "faint", id = o.id })
end

-- A surface: panels, cards, popovers. Inset `item` top and bottom and
-- `inset` at the sides unless `pad` (or padx / pady) says otherwise.
function kit.panel(props, children)
	local p = props or {}
	p.bg = p.bg or "surface"
	p.border = p.border or "line"
	if p.pad == nil then
		p.padx = p.padx or "inset"
		p.pady = p.pady or "item"
	end
	p.gap = p.gap or "s"
	return node(p.dir == "row" and "row" or "col", p, children)
end

-- A section's header: its caption, and on the right whatever belongs to
-- the whole section (a count, a link). Give the column it heads gap `item`.
function kit.section(label: string, right: any?, props: { [string]: any }?)
	local p = props or {}
	return node("row", { id = p.id, align = "center", gap = "item" }, {
		kit.caption(label),
		{ kind = "spacer" },
		right,
	})
end

-- A reading: a caption naming it over its value, as the tile readout lays
-- them out. `opts`: color (the value's), id.
function kit.reading(name: string, value: string, opts: { [string]: any }?)
	local o = opts or {}
	return node("col", { id = o.id, gap = "hair" }, {
		kit.caption(name),
		kit.label(value, { color = o.color }),
	})
end

function kit.divider()
	return { kind = "row", h = 1, w = "fill", bg = "line", radius = 0 }
end

-- What a button's role looks like: its fill and edge at rest, under the
-- pointer and pressed, and its text. Secondary is the default: most actions.
local ROLE = {
	primary = { bg = "accent", border = "accent", hover = "accent_hover", press = "accent_press", color = "on_accent" },
	secondary = { bg = "surface_raised", border = "line", hover = "surface_hover", edge = "line_strong", press = "surface_press" },
	quiet = { hover = "surface_hover", press = "surface_press" },
	danger = { bg = "surface_raised", border = "threat", hover = "threat_soft", press = "threat_press", color = "threat" },
}

-- A button's size: its padding and its text's size.
local SIZE = {
	s = { padx = "s", pady = "xs", size = "small" },
	m = { padx = "m", pady = "s" },
	l = { padx = "l", pady = "m", size = "heading" },
}

-- A button. props: label, on_click, active, disabled, tooltip, id, swatch
-- (a token or #colour shown as a small bar), strong, hint (a key, shown faint),
-- kind ("primary" for the one main action in a window, "secondary", the
-- default, "quiet" for one among many, "danger" for what can't be undone)
-- and size ("s", "m", the default, or "l").
function kit.button(props)
	local active = props.active
	local role = ROLE[props.kind or "secondary"]
	if not role then
		error(string.format("kit.button: unknown kind '%s' (primary, secondary, quiet or danger)", tostring(props.kind)), 2)
	end
	local size = SIZE[props.size or "m"]
	if not size then
		error(string.format("kit.button: unknown size '%s' (s, m or l)", tostring(props.size)), 2)
	end
	return node("row", {
		id = props.id,
		padx = props.padx or size.padx,
		pady = props.pady or size.pady,
		gap = "s",
		align = "center",
		bg = if active then "accent_soft" else role.bg,
		border = if active then "accent" else role.border,
		hover = { bg = if active then nil else role.hover, border = role.edge },
		press = { bg = role.press },
		on_click = props.on_click,
		disabled = props.disabled,
		tooltip = props.tooltip,
	}, {
		props.swatch and { kind = "row", w = 3, h = 12, bg = props.swatch, radius = 1 },
		kit.label(props.label, {
			size = size.size,
			weight = if active or props.strong or props.kind == "primary" then "strong" else "regular",
			color = props.color or role.color,
		}),
		props.hint and kit.label(props.hint, { size = "small", color = if role.color then role.color else "faint" }),
	})
end

-- An on/off switch with a label.
function kit.toggle(props)
	local on = props.on
	return kit.button({
		id = props.id,
		label = props.label,
		active = on,
		on_click = props.on_click,
		tooltip = props.tooltip,
		swatch = if on then "accent" else "faint",
	})
end

-- A labelled meter. props: label, value (0..1), color, width, label_width, tooltip, id.
function kit.bar(props)
	local v = math.clamp(props.value or 0, 0, 1)
	local pct = math.floor(v * 1000 + 0.5) / 10
	return node("row", { id = props.id, gap = "m", align = "center", w = props.width, tooltip = props.tooltip }, {
		props.label and kit.label(props.label, { size = "small", color = "muted" }) and {
			kind = "col",
			w = props.label_width or 52,
			kit.label(props.label, { size = "small", color = "muted" }),
		},
		{
			kind = "row",
			grow = 1,
			h = 6,
			bg = "track",
			radius = 3,
			pct > 0 and { kind = "row", w = pct .. "%", h = "fill", bg = props.color or "accent", radius = 3 },
		},
	})
end

-- Tabs. props: id, tabs = { {key, label} }, selected, on_select(key).
function kit.tabs(props)
	local items = {}
	for _, tab in props.tabs do
		table.insert(
			items,
			kit.button({
				id = props.id and (props.id .. "." .. tab.key),
				label = tab.label,
				active = tab.key == props.selected,
				on_click = function()
					props.on_select(tab.key)
				end,
			})
		)
	end
	return node("row", { id = props.id, gap = "xs" }, items)
end

-- A vertical list: render(item, index) for each item.
function kit.list(items, render, props)
	local rows = {}
	for i, item in items do
		table.insert(rows, render(item, i))
	end
	return node("col", props or { gap = "xs" }, rows)
end

-- A grid of cells the engine paints: one node however many cells. props:
-- rows, cols, cell(r, c) -> text | { text, bg, color } | nil, cell_w,
-- cell_h (tokens or px), gap, on_press(r, c) -> value, on_paint(r, c,
-- value), id, size, weight. Press picks a value; the drag paints it into
-- each cell it enters, once.
function kit.grid(props)
	props.kind = "grid"
	props.cell_w = props.cell_w or "l"
	props.cell_h = props.cell_h or "l"
	props.gap = props.gap or 1
	return props
end

-- A virtual table: a header row over a list that builds only the rows on
-- screen. props: id (required: its scroll position and sort are kept by
-- it), columns = { {label, w, key, value, sort} }, row_h, h (the visible
-- height), on_click(i, rec). Rows come one of two ways:
--   rows = { record, ... }   a cell is rec[col.key] or col.value(rec);
--                            a column with key or value sorts when its
--                            header is clicked (sort = false to refuse)
--   count, row(i) -> { cell, ... }   cells by hand, no sorting
-- A cell is a string, a number or a node.
function kit.table(props)
	local cols = props.columns
	local sort = props.rows and ui.state(props.id .. ".sort", nil)
	-- The sorted order of the records, when there is one to sort by.
	local order
	if props.rows then
		order = table.create(#props.rows)
		for i = 1, #props.rows do
			order[i] = i
		end
		local col = sort and cols[sort.col]
		if col then
			local value = col.value or function(rec)
				return rec[col.key]
			end
			local desc = sort.dir == "desc"
			table.sort(order, function(a, b)
				local va, vb = value(props.rows[a]), value(props.rows[b])
				if va == vb then
					return a < b
				end
				if type(va) ~= type(vb) then
					va, vb = tostring(va), tostring(vb)
				end
				if desc then
					return va > vb
				end
				return va < vb
			end)
		end
	end
	local header = {}
	for k, c in cols do
		local sortable = props.rows and c.sort ~= false and (c.key ~= nil or c.value ~= nil)
		local mark = sort and sort.col == k and (if sort.dir == "desc" then ui.t("core:table.desc", "▼") else ui.t("core:table.asc", "▲")) or nil
		table.insert(
			header,
			node("row", {
				id = props.id .. ".head." .. k,
				w = c.w,
				pad = "s",
				gap = "xs",
				align = "center",
				hover = sortable and { bg = "surface_hover" } or nil,
				on_click = sortable and function()
					local dir = if sort and sort.col == k and sort.dir == "asc" then "desc" else "asc"
					ui.set_state(props.id .. ".sort", { col = k, dir = dir })
				end or nil,
			}, {
				kit.label(c.label, { size = "small", color = "muted" }),
				mark and kit.label(mark, { size = "small", color = "accent" }),
			})
		)
	end
	local function row(i)
		local rec = order and props.rows[order[i]]
		local cells = props.row and props.row(i) or {}
		local out = { kind = "row", align = "center", hover = { bg = "surface_raised" } }
		if props.on_click then
			out.on_click = function()
				props.on_click(i, rec)
			end
		end
		for k, c in cols do
			local v = cells[k]
			if v == nil and rec then
				v = if c.value then c.value(rec) else rec[c.key]
			end
			if type(v) ~= "table" then
				v = kit.label(tostring(v or ""), { size = "small" })
			end
			table.insert(out, node("row", { w = c.w, pad = "s" }, { v }))
		end
		return out
	end
	local count = if props.rows then #props.rows else props.count
	return node("col", { id = props.id .. ".table", gap = 0 }, {
		node("row", { bg = "surface_raised" }, header),
		{ kind = "list", id = props.id, count = count, row = row, row_h = props.row_h or "l", h = props.h },
	})
end

-- The chrome around a window the engine manages: a title bar that drags,
-- a close button, the body in a scroll area, and a resize grip when the
-- window allows it. `win` is the engine's record: id, title, w, h,
-- resizable, comp (the component shown inside). The `handle` marks are
-- what the engine routes; the rest is ordinary kit.
function kit.window(win)
	return node("col", {
		id = win.id .. ".window",
		w = win.w,
		h = win.h,
		gap = 0,
		pad = 0,
		-- Windows sit over the world and other panels: more opaque than a
		-- docked bar, with a title bar a step lighter still.
		bg = "surface_raised",
		border = "line_strong",
		clip = true,
	}, {
		-- A sheet is placed by the engine: no move handle, and a title with
		-- more room, since it is a screen rather than a note.
		node("row", {
			id = win.id .. ".title",
			handle = if win.sheet then nil else "move",
			align = "center",
			gap = "s",
			pad = if win.sheet then "m" else "s",
			padx = "l",
			bg = "surface_hover",
			radius = 0,
		}, {
			kit.label(win.title, { weight = "strong", size = if win.sheet then "heading" else nil }),
			{ kind = "spacer" },
			node("row", {
				id = win.id .. ".close",
				handle = "close",
				pad = "xs",
				padx = "s",
				radius = "radius",
				hover = { bg = "surface_hover" },
				press = { bg = "surface_press" },
			}, { kit.label(ui.t("core:window.close", "×"), { color = "muted" }) }),
		}),
		{ kind = "scroll", id = win.id .. ".body", grow = 1, pad = "l", gap = "s", ui.slot(win.comp) },
		win.resizable and node("row", { justify = "end", pad = 0 }, {
			node("row", { id = win.id .. ".resize", handle = "resize", w = 14, h = 14, bg = "line", radius = 0 }),
		}),
	})
end

-- An icon: a mod's image (`mod:name`, from its ui/img/) drawn in the text
-- colour at `size` (a space token or px, square). opts: color, tooltip, id.
function kit.icon(name: string, size: any, opts: { [string]: any }?)
	local o = opts or {}
	return { kind = "image", src = name, w = size, h = size, tint = true, color = o.color, tooltip = o.tooltip, id = o.id }
end

-- ─── The item token (DESIGN.md §4f, Showing contents) ─────────────────────
-- One component draws every item on every screen, from the thing's own
-- world look tinted by its material, so a modded item needs no icon.

-- Sizes in logical px: s in lines, lists and tooltips (no count drawn), m in
-- grids, l as a selected stack's header.
local ITEM_SIZE = { s = 20, m = 32, l = 48 }

-- A count as a token shows it: exact to 999, then 1.0k and 12k. A count of
-- one is never drawn. Lists have room for the exact number.
function kit.item_count(n: number): string
	if n <= 1 then
		return ""
	elseif n < 1000 then
		return tostring(n)
	elseif n < 10000 then
		return string.format("%.1fk", math.floor(n / 100) / 10)
	end
	return tostring(math.floor(n / 1000)) .. "k"
end

-- The fields of a token, for `kit.item` and for a grid cell's `token`, so
-- a contents grid stays one node. p: thing, made_of (qualified ids), count,
-- limit (full at it) or full, hp (0 to 1), state ("incoming", "leaving",
-- "empty"), size.
function kit.item_token(p)
	local count = p.count or 1
	return {
		look = p.thing and view.look(p.thing, p.made_of) or nil,
		count = (p.size or "m") ~= "s" and kit.item_count(count) or "",
		full = p.full or (p.limit ~= nil and p.limit > 1 and count >= p.limit),
		hp = p.hp,
		state = p.state,
	}
end

-- An item token. p: as `kit.item_token`, plus id and tooltip.
function kit.item(p)
	local px = ITEM_SIZE[p.size or "m"] or ITEM_SIZE.m
	local t = kit.item_token(p)
	t.kind = "token"
	t.w = px
	t.h = px
	t.id = p.id
	t.tooltip = p.tooltip
	return t
end

-- A picture as shipped: its own colours and, unless w/h say otherwise,
-- its own size.
function kit.image(name: string, opts: { [string]: any }?)
	local o = opts or {}
	return { kind = "image", src = name, w = o.w, h = o.h, tooltip = o.tooltip, id = o.id }
end

-- A line of text the player edits. The engine keeps the buffer by `id`, so
-- it survives rebuilds and reloads. props: id (required), value (until the
-- player types), placeholder, on_change(text), on_submit(text), w, size.
function kit.input(props)
	return {
		kind = "input",
		id = props.id,
		value = props.value,
		placeholder = props.placeholder,
		on_change = props.on_change,
		on_submit = props.on_submit,
		on_key = props.on_key,
		w = props.w or 200,
		pad = "s",
		padx = "m",
		bg = "surface_raised",
		border = "line",
		focus = { border = "accent" },
		clip = true,
		size = props.size,
	}
end

-- A bar the pointer drags: on_change(v) with v in 0..1 while held. props:
-- value, on_change, width, color, id.
function kit.slider(props)
	local v = math.clamp(props.value or 0, 0, 1)
	local pct = math.floor(v * 1000 + 0.5) / 10
	return node("row", {
		id = props.id,
		w = props.width or 160,
		h = 16,
		align = "center",
		on_drag = function(fx)
			props.on_change(fx)
		end,
	}, {
		{
			kind = "row",
			grow = 1,
			h = 6,
			bg = "track",
			radius = 3,
			pct > 0 and { kind = "row", w = pct .. "%", h = "fill", bg = props.color or "accent", radius = 3 },
		},
	})
end

-- A context menu: a caption (what it's about, and who acts), then rows in
-- groups with a hairline between groups. props: id, subject, actor,
-- groups = { { row, ... }, ... } (empty groups leave no divider), hi (the
-- highlighted row's number, counting every row), and for a popup root:
-- at, on_key, on_outside. A row: label, tone ("primary" or "damaging"),
-- trailing (a key or a short note), disabled (the reason it can't run),
-- on_click, on_hover.
function kit.menu(props)
	local children = {}
	if props.subject or props.actor then
		table.insert(
			children,
			node("row", { padx = "item", pady = "tight", gap = "item", align = "center" }, {
				props.subject and kit.caption(props.subject),
				{ kind = "spacer" },
				props.actor and kit.label(props.actor, { size = "small", color = "faint" }),
			})
		)
	end
	local n = 0
	for _, group in props.groups do
		if #group > 0 then
			if n > 0 then
				table.insert(children, node("row", { h = 1, bg = "line" }))
			end
			for _, r in group do
				n += 1
				local primary, damaging = r.tone == "primary", r.tone == "damaging"
				local off = r.disabled ~= nil and r.disabled ~= false
				table.insert(
					children,
					node("row", {
						id = props.id and (props.id .. ".row." .. n),
						h = 28,
						padx = "item",
						gap = "item",
						align = "center",
						radius = "radius",
						bg = if n == props.hi then "surface_hover" else nil,
						hover = if off then nil else { bg = "surface_hover" },
						press = if off then nil else { bg = "surface_press" },
						on_click = if off then nil else r.on_click,
						on_hover = r.on_hover,
					}, {
						node("row", { w = 10, justify = "center", align = "center" }, {
							primary and node("row", { w = 5, h = 5, radius = 3, bg = "accent" }),
						}),
						kit.label(r.label, {
							weight = if primary then "strong" else nil,
							color = if off then "faint" elseif damaging then "threat" else nil,
						}),
						{ kind = "spacer" },
						kit.label(if off and type(r.disabled) == "string" then r.disabled else (r.trailing or ""), { size = "small", color = "faint" }),
					})
				)
			end
		end
	end
	return node("col", {
		id = props.id,
		at = props.at,
		on_key = props.on_key,
		on_outside = props.on_outside,
		minw = 184,
		maxw = 264,
		pad = "tight",
		gap = 0,
		bg = "surface_raised",
		border = "line_strong",
		radius = "radius",
	}, children)
end

local TOAST_COLOR = { info = "text", good = "good", threat = "threat", bad = "bad" }

-- A message toast. props: text, kind ("info"/"good"/"threat"/"bad"), age (0..1).
function kit.toast(props)
	local old = (props.age or 0) > 0.5
	return node("row", { padx = "l", pady = "s", bg = "surface", radius = "radius", maxw = 520 }, {
		kit.label(props.text, { color = if old then "muted" else TOAST_COLOR[props.kind] or "text", wrap = true }),
	})
end

-- A speech bubble for the anchored layer: text above a small tail, the
-- tail on the speaker. props: text, age (0..1 through its life; it fades
-- at the end). A long line wraps rather than spanning the screen.
function kit.bubble(props)
	local fading = (props.age or 0) > 0.8
	return node("col", { align = "center", gap = 0 }, {
		kit.panel({ pad = "s", padx = "m", bg = "surface_raised", border = "line_strong", maxw = 240 }, {
			kit.label(props.text, { size = "small", wrap = true, color = if fading then "muted" else nil }),
		}),
		{ kind = "row", w = 6, h = 4, bg = "line_strong", radius = 0 },
	})
end

-- A heading with an optional right-hand note.
function kit.heading(text: string, note: string?)
	return node("row", { gap = "m", align = "center" }, {
		kit.label(text, { weight = "strong" }),
		note and { kind = "spacer" },
		note and kit.label(note, { size = "small", color = "muted" }),
	})
end

-- A tray: columns side by side on one surface with a hairline between
-- them, rising from the dock. Nil columns are left out. props: id, w, and
-- anything a row takes.
function kit.tray(props, columns)
	local children = {}
	for _, c in columns do
		if c then
			if #children > 0 then
				table.insert(children, node("row", { w = 1, bg = "line" }))
			end
			table.insert(children, c)
		end
	end
	props.bg = props.bg or "surface"
	props.border = props.border or "line"
	props.radius = props.radius or "radius"
	return node("row", props, children)
end

-- Ask before something that can't be undone. props: title (the question),
-- body, verb (the button that does it), danger (the verb is red), cancel
-- (the safe button's label), on_confirm, on_cancel. One question at a time;
-- the safe answer has focus. See confirm.luau.
function kit.confirm(props)
	-- Required here, not at the top: confirm.luau builds with the kit.
	require("@core/ui/confirm").ask(props)
end

return kit
