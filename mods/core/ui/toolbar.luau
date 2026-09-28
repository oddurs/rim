-- The command dock: the bottom edge holds verbs only (Select, Orders, Build,
-- Zones), in a bar of fixed height. Picking one floats a tray over the map,
-- above the dock, that reads in the order the player decides: Build's group
-- rail, then its things as tiles, then a card for the thing under the
-- pointer with its cost and materials. Picking a thing folds the tray to a
-- pill in the bar that says what the next click places; Escape unfolds it.
--
-- Nothing here changes the bar's height: the tray floats and the pill sits
-- in the bar, so the panels docked above the bar never move (DESIGN.md §11).
--
-- The client files every tool under a category and group (a buildable's
-- group is its `build.menu`), so a mod's designation, building or menu
-- lands in the right tray with no change here. Only the open tray is built.
--
-- Ids: the bar `core:dock`, each verb `core:dock.<category>`, the tray
-- `core:dock.tray`, its group rail `core:dock.groups.<group>`, its tiles
-- `core:toolbar.<key>` in `core:toolbar.buttons`, materials `core:stuff.<id>`
-- in `core:stuff`, the pill `core:dock.pill`. Extension points:
-- `core:dock.status` on the bar for what the player can act on now (core's
-- undo is there), `core:dock.right` on the bar, empty for mods, and
-- `core:dock.palette.<category>` at the foot of a tray's list (built only
-- while that tray is open).

local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

local dock = {}

-- Verbs in dock order, with their keys. One the client reports that is
-- not listed here still gets a verb, after these, without a key. Palette
-- ids are written out so `ui.extend`'s typo check can see them.
local CATEGORIES = {
	{ id = "orders", key = "q", label = t("core:dock.orders", "Orders"), palette = { id = "core:dock.palette.orders" } },
	{ id = "build", key = "b", label = t("core:dock.build", "Build"), palette = { id = "core:dock.palette.build" } },
	{ id = "zones", key = "z", label = t("core:dock.zones", "Zones"), palette = { id = "core:dock.palette.zones" } },
}

-- Core's build menus. A mod's own menu is shown by its name, capitalised.
local MENUS = {
	structure = t("core:dock.menu.structure", "Structure"),
	furniture = t("core:dock.menu.furniture", "Furniture"),
	production = t("core:dock.menu.production", "Production"),
	dig = t("core:dock.menu.dig", "Dig"),
}

-- Tiles per row in the Build tray, and how many take a number key.
local PER_ROW = 3
local NUMBERED = 9
-- The tray's height, fixed: what the pointer is over changes the card, and
-- a tray that grew with it would slide the tiles out from under the pointer.
-- A column with more than fits scrolls.
local TRAY_H = 272
-- The bar's height, fixed: the pill and the undo chip sit inside it.
local BAR_H = 40

local function cap(s: string): string
	return string.upper(string.sub(s, 1, 1)) .. string.sub(s, 2)
end

local function category_key(id: string): string?
	for _, c in CATEGORIES do
		if c.id == id then
			return c.key
		end
	end
	return nil
end

local function category_label(id: string): string
	for _, c in CATEGORIES do
		if c.id == id then
			return c.label
		end
	end
	return cap(id)
end

local caption = kit.caption

local function head(left, right)
	return row({ gap = "item", align = "center" }, { left, { kind = "spacer" }, right })
end

local function keycap(k: string)
	return label(k, { size = "small", color = "faint" })
end

-- ─── State ──────────────────────────────────────────────────────────────────

-- The tool in hand, if it belongs to a tray.
local function in_hand(tools)
	for _, tool in tools do
		if tool.active and tool.category ~= "" then
			return tool
		end
	end
	return nil
end

-- The open tray: the tool in hand's, else the one opened by hand.
local function open_category(tools): string?
	local hand = in_hand(tools)
	return if hand then hand.category else ui.state("core:dock.open")
end

-- Placing: a tool is in hand and its tray is folded to the pill.
local function placing(tools): boolean
	return in_hand(tools) ~= nil and not ui.state("core:dock.unfold")
end

local function reset_card()
	ui.set_state("core:dock.hover", nil)
	ui.set_state("core:dock.find", nil)
	act.preview(nil)
end

local function close()
	ui.set_state("core:dock.open", nil)
	ui.set_state("core:dock.unfold", nil)
	reset_card()
	act.tool("select")
end

-- A verb: open its tray; again, unfold it while placing, or close it.
local function toggle(id: string)
	local tools = view.tools()
	local hand = in_hand(tools)
	if hand and hand.category == id then
		if ui.state("core:dock.unfold") then
			close()
		else
			ui.set_state("core:dock.unfold", true)
		end
	elseif not hand and ui.state("core:dock.open") == id then
		close()
	else
		act.tool("select")
		reset_card()
		ui.set_state("core:dock.unfold", nil)
		ui.set_state("core:dock.open", id)
	end
end

local function pick(tool)
	act.tool(tool.key)
	ui.set_state("core:dock.open", tool.category)
	ui.set_state("core:dock.unfold", nil)
	reset_card()
end

-- The tray's groups (in the client's order) and the tools in each.
local function grouped(tools, category: string)
	local groups, by = {}, {}
	for _, tool in tools do
		if tool.category == category then
			if not by[tool.group] then
				by[tool.group] = {}
				table.insert(groups, tool.group)
			end
			table.insert(by[tool.group], tool)
		end
	end
	return groups, by
end

local function chosen_group(tools, category: string, groups, by)
	local hand = in_hand(tools)
	local g = if hand and hand.category == category then hand.group else ui.state("core:dock.group." .. category)
	if not g or not by[g] then
		g = groups[1]
	end
	return g
end

-- The tiles the open tray shows, in order: a found list, or a group.
local function shown(tools, category: string)
	local groups, by = grouped(tools, category)
	if #groups == 0 then
		return {}, groups, by, nil
	end
	local query = ui.state("core:dock.find")
	if query and query ~= "" then
		local q = string.lower(query)
		local found = {}
		for _, tool in tools do
			if tool.category == category and string.find(string.lower(tool.label), q, 1, true) then
				table.insert(found, tool)
			end
		end
		return found, groups, by, nil
	end
	local g = chosen_group(tools, category, groups, by)
	return by[g], groups, by, g
end

-- ─── Keys ───────────────────────────────────────────────────────────────────

for _, c in CATEGORIES do
	ui.bind("core:dock." .. c.id, { key = c.key, label = string.format(t("core:keys.dock", "Open %s"), c.label) }, function()
		toggle(c.id)
	end)
end

-- The nth thing in the open tray, if a tray is open and has one: the
-- number keys pick it. `keys.luau` asks this first for 1 to 3, which are
-- the speed keys otherwise.
-- ─── A tool's context menu ────────────────────────────────────────────────
-- A tile or row in any tray is the subject "tool".
local menus = require("@core/ui/menus")

local function tool_by_key(key)
	for _, tool in view.tools() do
		if tool.key == key then
			return tool
		end
	end
	return nil
end

menus.kind("tool", {
	resolve = tool_by_key,
	caption = function(ctx)
		return ctx.subject.label
	end,
	actor = function(ctx)
		return if ctx.subject.group ~= "" then (MENUS[ctx.subject.group] or cap(ctx.subject.group)) else category_label(ctx.subject.category)
	end,
})
menus.add("tool", {
	id = "core:tool.use",
	label = function(ctx)
		return if ctx.subject.category == "build" then t("core:tool.place", "Place") else t("core:tool.use", "Use")
	end,
	run = function(ctx)
		pick(ctx.subject)
	end,
})
menus.add("tool", {
	id = "core:tool.find",
	label = t("core:tool.find", "Find in tray"),
	group = "manage",
	key = "/",
	run = function(ctx)
		ui.set_state("core:dock.open", ctx.subject.category)
		ui.set_state("core:dock.find", "")
		ui.set_input("core:dock.find", "")
		ui.focus("core:dock.find")
	end,
})

function dock.number(n: number): boolean
	local tools = view.tools()
	local open = open_category(tools)
	if not open or placing(tools) then
		return false
	end
	local list = shown(tools, open)
	local tool = list[n]
	if not tool then
		return false
	end
	pick(tool)
	return true
end

for n = 4, NUMBERED do
	ui.bind("core:dock.pick" .. n, { key = tostring(n), label = string.format(t("core:keys.pick", "Pick thing %d in the open tray"), n) }, function()
		dock.number(n)
	end)
end

-- Shift+Tab and Tab: the previous or next group in the open tray. [ and ]
-- change level (depth.luau).
local function step_group(by_n: number)
	local tools = view.tools()
	local open = open_category(tools)
	if not open then
		return
	end
	local groups, by = grouped(tools, open)
	if #groups < 2 then
		return
	end
	local g = chosen_group(tools, open, groups, by)
	local i = table.find(groups, g) or 1
	local next_g = groups[((i - 1 + by_n) % #groups) + 1]
	ui.set_state("core:dock.group." .. open, next_g)
	ui.set_state("core:dock.find", nil)
	if in_hand(tools) then
		act.tool("select")
	end
end
-- Only while a tray is open: otherwise Tab is the next colonist.
local function tray_open(): boolean
	return open_category(view.tools()) ~= nil
end
ui.bind(
	"core:dock.prev_group",
	{ key = "shift+tab", label = t("core:keys.prev_group", "Previous group in the open tray"), when = tray_open },
	function()
		step_group(-1)
	end
)
ui.bind(
	"core:dock.next_group",
	{ key = "tab", label = t("core:keys.next_group", "Next group in the open tray"), when = tray_open },
	function()
		step_group(1)
	end
)

-- /: find a thing by name in the open tray.
ui.bind("core:dock.find", { key = "/", label = t("core:keys.find", "Find a thing in the open tray") }, function()
	local tools = view.tools()
	local open = open_category(tools)
	if not open then
		return
	end
	if in_hand(tools) then
		act.tool("select")
	end
	ui.set_state("core:dock.find", "")
	ui.set_input("core:dock.find", "")
	ui.focus("core:dock.find")
end)

-- Escape backs out one step at a time: placing to its tray, a tray to
-- closed, then an open sheet, then the selection. A mod that binds
-- `core:escape` itself replaces these steps whole, and the conflict is
-- reported.
ui.bind("core:escape", { key = "escape", label = t("core:keys.escape", "Back out: stop placing, close the tray, close the sheet, deselect") }, function()
	local tools = view.tools()
	if in_hand(tools) then
		act.tool("select")
		ui.set_state("core:dock.unfold", nil)
		reset_card()
		return
	end
	local sheet = ui.sheet()
	if ui.state("core:dock.open") then
		close()
	elseif sheet then
		ui.close(sheet)
	else
		act.select(nil)
	end
end)

-- ─── The parts ──────────────────────────────────────────────────────────────

local function swatch(color, size: number)
	return { kind = "row", w = size, h = size, bg = color, radius = 2 }
end

-- A tile: swatch, name over cost, and its number key.
local function tile(tool, n: number?)
	return row({
		id = "core:toolbar." .. tool.key,
		menu = { kind = "tool", id = tool.key },
		w = 150,
		padx = "m",
		pady = "s",
		gap = "m",
		align = "center",
		radius = "radius",
		bg = if tool.active then "accent_soft" else nil,
		border = if tool.active then "accent" else nil,
		hover = { bg = if tool.active then nil else "surface_hover" },
		press = { bg = "surface_press" },
		on_click = function()
			pick(tool)
		end,
		on_hover = function()
			ui.set_state("core:dock.hover", tool.key)
			act.preview(tool.key)
		end,
	}, {
		swatch(tool.color, 18),
		col({ gap = "xxs", grow = 1 }, {
			label(tool.label, { weight = if tool.active then "strong" else nil, color = if tool.locked ~= "" then "muted" else nil }),
			-- A locked buildable says why in place of its cost.
			if tool.locked ~= "" then label(tool.locked, { size = "small", color = "faint" }) else tool.cost ~= "" and label(tool.cost, { size = "small", color = "faint" }),
		}),
		n and n <= NUMBERED and keycap(tostring(n)),
	})
end

-- A list row, for trays without tiles: dot, name, a note, its number key.
local function list_row(tool, note: string?, n: number?)
	return row({
		id = "core:toolbar." .. tool.key,
		menu = { kind = "tool", id = tool.key },
		padx = "m",
		pady = "s",
		gap = "m",
		align = "center",
		radius = "radius",
		bg = if tool.active then "accent_soft" else nil,
		border = if tool.active then "accent" else nil,
		hover = { bg = if tool.active then nil else "surface_hover" },
		press = { bg = "surface_press" },
		on_click = function()
			pick(tool)
		end,
		on_hover = function()
			ui.set_state("core:dock.hover", tool.key)
		end,
	}, {
		swatch(tool.color, 10),
		label(tool.label),
		{ kind = "spacer" },
		note and label(note, { size = "small", color = "faint" }),
		n and n <= NUMBERED and keycap(tostring(n)),
	})
end

-- The materials a build takes: what you have is a count, what you don't
-- is shown, greyed, so a missing material is a fact rather than a gap.
local function materials(view, compact: boolean)
	local stuff = view.stuff()
	if #stuff == 0 then
		return nil
	end
	local rows = {}
	for _, m in stuff do
		local none = m.have == 0
		local have = if none then t("core:stuff.none_short", "none") else tostring(m.have)
		table.insert(
			rows,
			row({
				id = "core:stuff." .. m.id,
				padx = "m",
				pady = if compact then "xs" else "s",
				gap = "m",
				align = "center",
				radius = "radius",
				bg = if m.active then "accent_soft" else nil,
				border = if m.active then "accent" else nil,
				hover = { bg = if m.active then nil else "surface_hover" },
				tooltip = string.format(t("core:stuff.stats", "hp %d · %d work"), m.hp, m.work),
				on_click = function()
					act.stuff(m.id)
				end,
			}, {
				swatch(m.color, 10),
				label(m.label, { color = if none then "faint" else nil }),
				not compact and { kind = "spacer" },
				label(have, { size = "small", color = if none then "faint" else "muted" }),
			})
		)
	end
	if compact then
		return row({ id = "core:stuff", gap = "xs", align = "center" }, rows)
	end
	return col({ id = "core:stuff", gap = "xxs" }, rows)
end

-- The card: what the thing under the pointer (or in hand) is and costs.
local function blueprint(view, tool, group_label: string)
	if not tool then
		return nil
	end
	local stats = row({ gap = "xl" }, {
		kit.reading(t("core:dock.cost", "Cost"), if tool.cost ~= "" then tool.cost else "–"),
		kit.reading(t("core:dock.work", "Work"), tostring(tool.work)),
		kit.reading(t("core:dock.hp", "HP"), tostring(tool.hp)),
	})
	local mats = materials(view, false)
	return col({ id = "core:dock.card", w = 236, padx = "l", pady = "m", gap = "m", clip = true }, {
		head(caption(t("core:dock.blueprint", "Blueprint")), caption(group_label)),
		label(tool.label, { size = "heading", weight = "strong" }),
		tool.locked ~= "" and label(tool.locked, { color = "bad", wrap = true }),
		stats,
		mats and head(caption(t("core:dock.material", "Material")), nil),
		mats,
	})
end

-- The card for an order: how many it has marked, and how to mark more.
local function order_card(view, tool, marked)
	if not tool then
		return nil
	end
	local id = string.match(tool.key, "^designate:(.+)$")
	local n = id and marked[id] or 0
	return col({ id = "core:dock.card", w = 236, padx = "l", pady = "m", gap = "m" }, {
		head(caption(t("core:dock.order", "Order")), nil),
		label(tool.label, { size = "heading", weight = "strong" }),
		id and kit.reading(t("core:dock.marked_label", "Marked"), tostring(n)),
		label(t("core:dock.order_hint", "Drag across the map to mark; right-click to stop."), { size = "small", color = "muted", wrap = true }),
	})
end

local function tray_frame(view, tools, open: string)
	local list, groups, by, group = shown(tools, open)
	local query = ui.state("core:dock.find")
	local finding = query ~= nil
	local hover_key = ui.state("core:dock.hover")
	local described = nil
	for _, tool in list do
		if tool.key == hover_key then
			described = tool
		end
	end
	described = described or in_hand(tools) or list[1]

	-- The group rail, when the tray has more than one group.
	local rail = nil
	if #groups > 1 then
		local items = {}
		for _, g in groups do
			local on = not finding and g == group
			table.insert(
				items,
				row({
					id = "core:dock.groups." .. g,
					padx = "m",
					pady = "s",
					gap = "m",
					align = "center",
					radius = "radius",
					bg = if on then "surface_hover" else nil,
					hover = { bg = "surface_hover" },
					on_click = function()
						ui.set_state("core:dock.group." .. open, g)
						reset_card()
						if in_hand(tools) then
							act.tool("select")
						end
					end,
				}, {
					label(MENUS[g] or cap(g), { weight = if on then "strong" else nil, color = if on then nil else "muted" }),
					{ kind = "spacer" },
					label(tostring(#by[g]), { size = "small", color = "faint" }),
				})
			)
		end
		rail = col({ w = 160, padx = "l", pady = "m", gap = "xxs" }, {
			head(caption(category_label(open)), keycap("Tab")),
			{ kind = "scroll", gap = "xxs", grow = 1, minh = 0, table.unpack(items) },
		})
	end

	-- The things: tiles in rows for Build, a list for the rest.
	local body
	local marked = if open == "orders" then view.marked() else {}
	if open == "build" then
		local rows, current = {}, {}
		for i, tool in list do
			table.insert(current, tile(tool, i))
			if #current == PER_ROW then
				table.insert(rows, row({ gap = "xs" }, current))
				current = {}
			end
		end
		if #current > 0 then
			table.insert(rows, row({ gap = "xs" }, current))
		end
		body = col({ id = "core:toolbar.buttons", gap = "xs" }, rows)
	else
		local rows = {}
		for i, tool in list do
			local id = string.match(tool.key, "^designate:(.+)$")
			local note = if id and marked[id] then string.format(t("core:dock.marked", "%d marked"), marked[id]) else nil
			table.insert(rows, list_row(tool, note, i))
		end
		-- Zones: the colony's stockpiles under the tools; one opens the list.
		if open == "zones" then
			local zones = view.zones()
			if #zones > 0 then
				table.insert(rows, row({ padx = "m", pady = "s" }, { head(caption(t("core:dock.your_zones", "Your stockpiles")), caption(tostring(#zones))) }))
				for _, z in zones do
					table.insert(
						rows,
						row({
							id = "core:dock.zone." .. z.id,
							menu = { kind = "zone", id = z.id },
							padx = "m",
							pady = "s",
							gap = "m",
							align = "center",
							radius = "radius",
							hover = { bg = "surface_hover" },
							on_click = function()
								act.select_zone(z.id)
							end,
						}, {
							swatch("accent", 10),
							label(z.name),
							{ kind = "spacer" },
							label(string.format(t("core:dock.cells", "%d cells"), z.cells), { size = "small", color = "faint" }),
						})
					)
				end
			end
		end
		body = col({ id = "core:toolbar.buttons", gap = "xxs", w = 260 }, rows)
	end

	local finder = if finding
		then kit.input({
			id = "core:dock.find",
			placeholder = t("core:dock.find_hint", "Find by name"),
			w = "fill",
			on_change = function(text)
				ui.set_state("core:dock.find", text)
			end,
		})
		else row({ gap = "s", align = "center" }, {
			label(string.format(t("core:dock.count", "%d things"), #list), { size = "small", color = "faint" }),
			{ kind = "spacer" },
			label(t("core:dock.find_key", "/ find"), { size = "small", color = "faint" }),
		})
	local middle = col({ padx = "l", pady = "m", gap = "s" }, {
		if rail == nil then head(caption(category_label(open)), category_key(open) and keycap(string.upper(category_key(open) :: string))) else nil,
		finder,
		{
			kind = "scroll",
			id = "core:dock.list",
			grow = 1,
			minh = 0,
			gap = "s",
			body,
			row({ id = "core:dock.palette." .. open, gap = "s", align = "center" }), -- see CATEGORIES
		},
	})

	local card = nil
	if open == "build" then
		card = blueprint(view, described, if group then (MENUS[group] or cap(group)) else "")
	elseif open == "orders" then
		card = order_card(view, described, marked)
	end
	return kit.tray({ id = "core:dock.tray", h = TRAY_H }, { rail, middle, card })
end

-- The pill: what the next click on the map does, while placing.
local function pill(view, tool)
	local stuff = view.stuff()
	local chosen = nil
	for _, m in stuff do
		if m.active then
			chosen = m
		end
	end
	local what = if chosen then string.format(t("core:dock.placing_in", "%s in %s"), tool.label, chosen.label) else tool.label
	local sub = if tool.locked ~= "" then tool.locked
		elseif tool.category == "build"
		then (if tool.cost ~= "" then tool.cost .. " · " else "") .. string.format(t("core:dock.work_n", "%d work"), tool.work)
		else t("core:dock.drag", "drag on the map")
	-- One line, so it fits the bar without changing its height.
	return row({
		id = "core:dock.pill",
		padx = "m",
		gap = "m",
		align = "center",
	}, {
		swatch(tool.color, 12),
		label(what, { weight = "strong" }),
		label(sub, { size = "small", color = "faint" }),
		materials(view, true),
		label(t("core:dock.esc_back", "Esc back"), { size = "small", color = "faint" }),
	})
end

ui.define("core:toolbar", function(view)
	local tools = view.tools()
	local open = open_category(tools)
	local hand = in_hand(tools)

	-- Verbs in the order above, then any the client adds.
	local cats, seen = {}, {}
	for _, c in CATEGORIES do
		seen[c.id] = true
		table.insert(cats, c)
	end
	local select = nil
	local present = {}
	for _, tool in tools do
		present[tool.category] = true
		if tool.category == "" then
			select = select or tool
		elseif not seen[tool.category] then
			seen[tool.category] = true
			table.insert(cats, { id = tool.category })
		end
	end

	local bar = {}
	if select then
		table.insert(
			bar,
			button({
				id = "core:toolbar." .. select.key,
				label = select.label,
				active = select.active,
				on_click = function()
					close()
				end,
			})
		)
	end
	for _, c in cats do
		if present[c.id] then
			table.insert(
				bar,
				button({
					id = "core:dock." .. c.id,
					label = category_label(c.id),
					hint = c.key and string.upper(c.key),
					active = c.id == open,
					on_click = function()
						toggle(c.id)
					end,
				})
			)
		end
	end
	table.insert(bar, { kind = "spacer" })
	-- What the next click does, and what can be taken back, in the bar.
	local placing_now = hand and placing(tools)
	local state = if placing_now
		then pill(view, hand)
		elseif open then label(string.format(t("core:dock.state_choosing", "Choosing · %s"), category_label(open)), { size = "small", color = "faint" })
		else nil
	table.insert(bar, row({ id = "core:dock.status", gap = "l", align = "center" }, { state }))
	table.insert(bar, row({ id = "core:dock.right", gap = "s", align = "center" }))
	return row({ id = "core:dock", h = BAR_H, padx = "l", gap = "s", bg = "surface", radius = 0, align = "center" }, bar)
end)
ui.mount("bottom", "core:toolbar")

-- The open tray floats over the map, above the dock: it comes and goes
-- without moving anything docked.
ui.define("core:toolbar.tray", function(view)
	local tools = view.tools()
	local open = open_category(tools)
	local hand = in_hand(tools)
	if not open or (hand and placing(tools)) then
		return nil
	end
	return tray_frame(view, tools, open)
end)
ui.mount("float", "core:toolbar.tray", { align = "end" })

return dock
