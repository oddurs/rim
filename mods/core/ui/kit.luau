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
	return { kind = "text", text, size = o.size, weight = o.weight, color = o.color, wrap = o.wrap, id = o.id }
end

-- A surface: panels, cards, popovers.
function kit.panel(props, children)
	local p = props or {}
	p.bg = p.bg or "surface"
	p.border = p.border or "line"
	p.pad = p.pad or "m"
	p.gap = p.gap or "s"
	return node(p.dir == "row" and "row" or "col", p, children)
end

function kit.divider()
	return { kind = "row", h = 1, w = "fill", bg = "line", radius = 0 }
end

-- A button. props: label, on_click, active, disabled, tooltip, id, swatch
-- (a token or #colour shown as a small bar), strong.
function kit.button(props)
	local active = props.active
	return node("row", {
		id = props.id,
		padx = props.padx or "m",
		pady = props.pady or "xs",
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

-- A labelled meter. props: label, value (0..1), color, width, id.
function kit.bar(props)
	local v = math.clamp(props.value or 0, 0, 1)
	local pct = math.floor(v * 1000 + 0.5) / 10
	return node("row", { id = props.id, gap = "m", align = "center", w = props.width }, {
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
-- screen. props: id (required: its scroll position is kept by it), columns
-- = { {label, w} }, count, row(i) -> { cell, ... } (strings or nodes),
-- row_h, h (the visible height), on_click(i).
function kit.table(props)
	local cols = props.columns
	local header = {}
	for _, c in cols do
		table.insert(header, node("row", { w = c.w, pad = "xs" }, { kit.label(c.label, { size = "small", color = "muted" }) }))
	end
	local function row(i)
		local cells = props.row(i)
		local out = { kind = "row", align = "center", hover = { bg = "surface_raised" } }
		if props.on_click then
			out.on_click = function()
				props.on_click(i)
			end
		end
		for k, c in cols do
			local v = cells[k]
			if type(v) ~= "table" then
				v = kit.label(tostring(v or ""), { size = "small" })
			end
			table.insert(out, node("row", { w = c.w, pad = "xs" }, { v }))
		end
		return out
	end
	return node("col", { id = props.id .. ".table", gap = 0 }, {
		node("row", { bg = "surface_raised" }, header),
		{ kind = "list", id = props.id, count = props.count, row = row, row_h = props.row_h or "l", h = props.h },
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
				pady = "xs",
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
	return node("row", { padx = "m", pady = "xs", bg = "surface", radius = "radius", maxw = 520 }, {
		kit.label(props.text, { color = if old then "muted" else TOAST_COLOR[props.kind] or "text", wrap = true }),
	})
end

-- A speech bubble for the anchored layer: text above a small tail.
function kit.bubble(props)
	return node("col", { align = "center", gap = 0 }, {
		kit.panel({ pad = "s", padx = "m", bg = "surface_raised", border = "line_strong" }, {
			kit.label(props.text, { size = "small" }),
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
