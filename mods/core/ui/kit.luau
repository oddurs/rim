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

-- Text. `opts`: size ("small"/"body"/"heading"/"title"), weight, color, wrap.
function kit.label(text: string, opts: { [string]: any }?)
	local o = opts or {}
	return { kind = "text", text, size = o.size, weight = o.weight, color = o.color, wrap = o.wrap, id = o.id, minw = o.minw, grow = o.grow }
end

-- A surface: panels, cards, popovers.
function kit.panel(props, children)
	local p = props or {}
	p.bg = p.bg or "surface"
	p.border = p.border or "line"
	p.pad = p.pad or "l"
	p.gap = p.gap or "s"
	return node(p.dir == "row" and "row" or "col", p, children)
end

function kit.divider()
	return { kind = "row", h = 1, w = "fill", bg = "line", radius = 0 }
end

-- A button. props: label, on_click, active, disabled, tooltip, id, swatch
-- (a token or #colour shown as a small bar), strong, hint (a key, shown faint).
function kit.button(props)
	local active = props.active
	return node("row", {
		id = props.id,
		padx = props.padx or "m",
		pady = props.pady or "s",
		gap = "s",
		align = "center",
		bg = if active then "accent_soft" else "surface_raised",
		border = if active then "accent" else "line",
		hover = { bg = if active then nil else "surface_hover", border = "line_strong" },
		press = { bg = "surface_press" },
		focus = { border = "accent" },
		on_click = props.on_click,
		disabled = props.disabled,
		tooltip = props.tooltip,
	}, {
		props.swatch and { kind = "row", w = 3, h = 12, bg = props.swatch, radius = 1 },
		kit.label(props.label, { weight = if active or props.strong then "strong" else "regular", color = props.color }),
		props.hint and kit.label(props.hint, { size = "small", color = "faint" }),
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
		node("row", { id = win.id .. ".title", handle = "move", align = "center", gap = "s", pad = "s", padx = "l", bg = "surface_hover", radius = 0 }, {
			kit.label(win.title, { weight = "strong" }),
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

-- A context menu. props: items = { {label, on_click, disabled} }, id.
function kit.menu(props)
	local rows = {}
	for _, item in props.items do
		table.insert(
			rows,
			node("row", {
				padx = "m",
				pady = "s",
				hover = { bg = "surface_hover" },
				press = { bg = "surface_press" },
				on_click = item.on_click,
				disabled = item.disabled,
			}, { kit.label(item.label) })
		)
	end
	return kit.panel({ id = props.id, pad = "xs", gap = "xxs", bg = "surface_raised" }, rows)
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

return kit
