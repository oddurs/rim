-- The dock: every tool the client offers, filed by category. The client
-- says which category and group each tool is in (a buildable's group is its
-- `build.menu`), so a mod's new designation, building or menu lands in the
-- right place without touching this file.
--
-- One palette is open at a time and only it is built: a hundred buildings
-- cost nothing until someone opens Build, and the dock stays one row wide
-- however many mods are loaded.
--
-- Ids: the bar is `core:dock`, each category's button `core:dock.<category>`,
-- the open palette's group tabs `core:dock.groups.<group>`. Tool buttons keep
-- `core:toolbar.<key>` in the row `core:toolbar.buttons`, and materials
-- `core:stuff.<id>` in `core:stuff`. Extension points, empty for mods:
-- `core:dock.right` on the bar, and `core:dock.palette.<category>` after a
-- palette's tools (built only while that palette is open).

local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

-- Categories in dock order, with their keys. One the client reports that is
-- not listed here still gets a button, after these, without a key. Palette
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
}

local function cap(s: string): string
	return string.upper(string.sub(s, 1, 1)) .. string.sub(s, 2)
end

local function category_label(id: string): string
	for _, c in CATEGORIES do
		if c.id == id then
			return c.label
		end
	end
	return cap(id)
end

-- The open category: the active tool's while one is in use, else the one
-- last opened by hand.
local function open_category(tools): string?
	for _, tool in tools do
		if tool.active and tool.category ~= "" then
			return tool.category
		end
	end
	return ui.state("core:dock.open")
end

local function close()
	ui.set_state("core:dock.open", nil)
	act.tool("select")
end

-- Open a category, or close it if it is already open. Opening another drops
-- the tool in hand: it belongs to the palette being left.
local function toggle(id: string)
	if open_category(view.tools()) == id then
		close()
	else
		ui.set_state("core:dock.open", id)
		act.tool("select")
	end
end

for _, c in CATEGORIES do
	ui.bind("core:dock." .. c.id, { key = c.key, label = string.format(t("core:keys.dock", "Open %s"), c.label) }, function()
		toggle(c.id)
	end)
end

-- Escape backs out one step at a time: the tool (and its palette), then a
-- palette opened by hand, then the selection. A mod that binds `core:escape`
-- itself replaces these steps whole, and the conflict is reported.
ui.bind("core:escape", { key = "escape", label = t("core:keys.escape", "Back out: drop the tool, close the palette, deselect") }, function()
	local tools = view.tools()
	for _, tool in tools do
		if tool.active and tool.category ~= "" then
			close()
			return
		end
	end
	if ui.state("core:dock.open") then
		ui.set_state("core:dock.open", nil)
	else
		act.select(nil)
	end
end)

local function tool_button(tool)
	return button({
		id = "core:toolbar." .. tool.key,
		label = tool.label,
		active = tool.active,
		swatch = tool.color,
		on_click = function()
			act.tool(tool.key)
		end,
	})
end

-- The material row: only when the active tool builds out of something.
-- What you have none of is shown greyed, not hidden, so the row keeps its
-- shape and a missing material is a fact rather than a mystery.
local function materials(view)
	local stuff = view.stuff()
	if #stuff == 0 then
		return nil
	end
	local picks = {}
	local chosen = nil
	for _, m in stuff do
		if m.active then
			chosen = m
		end
		local none = m.have == 0
		table.insert(
			picks,
			button({
				id = "core:stuff." .. m.id,
				label = if none then string.format(t("core:stuff.none", "%s · none"), m.label) else string.format(t("core:stuff.have", "%s ×%d"), m.label, m.have),
				active = m.active,
				swatch = m.color,
				color = if none then "faint" else nil,
				tooltip = string.format(t("core:stuff.stats", "hp %d · %d work"), m.hp, m.work),
				on_click = function()
					act.stuff(m.id)
				end,
			})
		)
	end
	if chosen then
		table.insert(picks, label(string.format(t("core:stuff.stats", "hp %d · %d work"), chosen.hp, chosen.work), { size = "small", color = "muted" }))
	end
	return row({ id = "core:stuff", padx = "l", pady = "s", gap = "s", bg = "surface", radius = 0, align = "center" }, picks)
end

-- The open category's palette: its groups as tabs when it has more than
-- one, and the chosen group's tools.
local function palette(tools, open: string)
	local groups, by_group = {}, {}
	local active_group = nil
	for _, tool in tools do
		if tool.category == open then
			if not by_group[tool.group] then
				by_group[tool.group] = {}
				table.insert(groups, tool.group)
			end
			table.insert(by_group[tool.group], tool)
			if tool.active then
				active_group = tool.group
			end
		end
	end
	if #groups == 0 then
		return nil
	end
	local chosen = active_group or ui.state("core:dock.group." .. open)
	if not chosen or not by_group[chosen] then
		chosen = groups[1]
	end
	local items = {}
	for _, tool in by_group[chosen] do
		table.insert(items, tool_button(tool))
	end
	local buttons = row({ gap = "s", align = "center" }, {
		row({ id = "core:toolbar.buttons", gap = "s", align = "center" }, items),
		row({ id = "core:dock.palette." .. open, gap = "s", align = "center" }), -- see CATEGORIES
	})
	if #groups == 1 then
		return row({ padx = "l", pady = "s", bg = "surface", radius = 0 }, { buttons })
	end
	local tabs = {}
	for _, g in groups do
		table.insert(tabs, { key = g, label = MENUS[g] or cap(g) })
	end
	return col({ padx = "l", pady = "s", gap = "s", bg = "surface", radius = 0 }, {
		kit.tabs({
			id = "core:dock.groups",
			tabs = tabs,
			selected = chosen,
			on_select = function(g)
				ui.set_state("core:dock.group." .. open, g)
				act.tool("select")
			end,
		}),
		buttons,
	})
end

ui.define("core:toolbar", function(view)
	local tools = view.tools()
	local open = open_category(tools)

	-- Categories in the order above, then any the client adds.
	local cats, seen = {}, {}
	for _, c in CATEGORIES do
		seen[c.id] = true
		table.insert(cats, c)
	end
	local select = nil
	for _, tool in tools do
		if tool.category == "" then
			select = select or tool
		elseif not seen[tool.category] then
			seen[tool.category] = true
			table.insert(cats, { id = tool.category })
		end
	end
	local present = {}
	for _, tool in tools do
		present[tool.category] = true
	end

	local bar = {}
	if select then
		table.insert(bar, tool_button(select))
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
	-- Screens (mods/core/ui/screens.luau) and mods' buttons, at the far end.
	table.insert(bar, { kind = "spacer" })
	table.insert(bar, row({ id = "core:dock.right", gap = "s", align = "center" }))

	return col({}, {
		materials(view),
		open and palette(tools, open),
		row({ id = "core:dock", padx = "l", pady = "s", gap = "s", bg = "surface", radius = 0, align = "center" }, bar),
	})
end)
ui.mount("bottom", "core:toolbar")
