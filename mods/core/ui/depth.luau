-- The depth ruler (DESIGN.md §6d): every level reached, who is on it and
-- what is wrong there, with the one on screen marked. [ and ] step down and
-- up through the reached levels; a click goes straight to one.
--
-- Hidden while the surface is the only level reached: one row says nothing.

local kit = require("@core/ui/kit")
local alerts = require("@core/ui/alerts")
local row, label = kit.row, kit.label
local t = ui.t

local WIDTH = 132

-- Reached levels, the highest first.
local function reached(view)
	local out = {}
	for _, l in view.levels() do
		if l.reached then
			table.insert(out, l)
		end
	end
	return out
end

local function name(z: number): string
	if z == 0 then
		return t("core:depth.surface", "Surface")
	end
	return if z < 0 then `−{-z}` else `+{z}`
end

-- The next reached level from the one on screen, `by` levels down (-1) or
-- up (1); where there is none, it stays.
local function step(by: number)
	local here = view.level()
	local best = nil
	for _, l in reached(view) do
		local d = (l.z - here) * by
		if d > 0 and (best == nil or d < (best - here) * by) then
			best = l.z
		end
	end
	if best then
		act.level(best)
	end
end

ui.bind("core:depth.down", { key = "[", label = t("core:keys.level_down", "Down a level") }, function()
	step(-1)
end)
ui.bind("core:depth.up", { key = "]", label = t("core:keys.level_up", "Up a level") }, function()
	step(1)
end)

ui.define("core:depth", function(view)
	local levels = reached(view)
	local here = view.level()
	if #levels < 2 and here == 0 then
		return nil
	end
	-- Alerts about something on a level count there.
	local warned: { [number]: number } = {}
	for _, a in alerts.active(view) do
		local z = a.subject and view.level_of(a.subject)
		if z then
			warned[z] = (warned[z] or 0) + 1
		end
	end
	local rows = {}
	for _, l in levels do
		local on = l.z == here
		table.insert(
			rows,
			row({
				id = "core:depth.level." .. l.z,
				gap = "item",
				align = "center",
				padx = "xs",
				radius = "radius",
				bg = if on then "accent_soft" else nil,
				hover = if on then nil else { bg = "surface_hover" },
				on_click = function()
					act.level(l.z)
				end,
				tooltip = string.format(
					t("core:depth.tip", "%s: %d colonists, %d others"),
					name(l.z),
					l.colonists,
					l.others
				),
			}, {
				label(name(l.z), { color = if on then nil else "muted", weight = if on then "strong" else nil, grow = 1 }),
				if warned[l.z]
					then label(tostring(warned[l.z]), { id = "core:depth.alerts." .. l.z, size = "small", color = "threat" })
					else nil,
				if l.colonists > 0
					then label(tostring(l.colonists), { id = "core:depth.count." .. l.z, size = "small", color = "faint" })
					else nil,
			})
		)
	end
	return kit.panel({ w = WIDTH, gap = "xxs" }, {
		kit.section(t("core:depth.title", "Levels"), label("[ ]", { size = "small", color = "faint" })),
		kit.col({ gap = "hair" }, rows),
	})
end)
ui.mount("right", "core:depth", { align = "start", order = -5 })
