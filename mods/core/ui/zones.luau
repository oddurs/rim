-- Stockpiles: each zone and the items it takes. Paint zones with the
-- stockpile tool; here a toggle per item lets a zone take it or not.
local kit = require("@core/ui/kit")
local menus = require("@core/ui/menus")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

-- ─── A stockpile's context menu ────────────────────────────────────────────
-- A stockpile in any list (this sheet, the Zones tray) is the subject
-- "zone".
menus.kind("zone", {
	resolve = function(id)
		for _, z in view.zones() do
			if z.id == id then
				return z
			end
		end
		return nil
	end,
	caption = function(ctx)
		return ctx.subject.name
	end,
	actor = function(ctx)
		return string.format(t("core:zones.cells", "%d cells"), ctx.subject.cells)
	end,
})
menus.add("zone", {
	id = "core:zone.edit",
	label = t("core:zone.edit", "What it takes…"),
	group = "manage",
	primary = function()
		return true
	end,
	run = function()
		ui.open("core:zones")
	end,
})
local function allow_all(on: boolean)
	return function(ctx)
		for _, it in view.items() do
			act.zone_allow(ctx.id, it.id, on)
		end
	end
end
menus.add("zone", { id = "core:zone.allow_all", label = t("core:zone.allow_all", "Take everything"), group = "manage", run = allow_all(true) })
menus.add("zone", { id = "core:zone.forbid_all", label = t("core:zone.forbid_all", "Take nothing"), group = "manage", run = allow_all(false) })
-- A stack only moves to a store at a higher level (DESIGN.md §4f).
local function step_level(by: number)
	return function(ctx)
		local top = #view.store_levels() - 1
		act.store_level(ctx.id, math.clamp(ctx.subject.level + by, 0, top))
	end
end
menus.add("zone", { id = "core:zone.raise", label = t("core:zone.raise", "Raise priority"), group = "manage", run = step_level(1) })
menus.add("zone", { id = "core:zone.lower", label = t("core:zone.lower", "Lower priority"), group = "manage", run = step_level(-1) })

local PER_ROW = 4

ui.window("core:zones", { title = t("core:zones.title", "Stockpiles"), w = 460, h = 320, sheet = true }, function(view)
	local zones = view.zones()
	if #zones == 0 then
		return col({ pad = "m" }, {
			label(t("core:zones.none", "No stockpiles yet. Paint one with the stockpile tool."), { color = "muted", wrap = true }),
		})
	end
	local items = view.items()
	local top = #view.store_levels() - 1
	local parts = {}
	for _, z in zones do
		table.insert(
			parts,
			row({ id = "core:zones.zone." .. z.id, menu = { kind = "zone", id = z.id }, gap = "xs" }, {
				kit.heading(z.name, string.format(t("core:zones.cells", "%d cells"), z.cells)),
				kit.button({
					id = "core:zones.lower." .. z.id,
					label = "−",
					tooltip = t("core:zone.lower", "Lower priority"),
					on_click = function()
						act.store_level(z.id, math.max(z.level - 1, 0))
					end,
				}),
				label(z.level_label, { id = "core:zones.level." .. z.id, minw = 72 }),
				kit.button({
					id = "core:zones.raise." .. z.id,
					label = "+",
					tooltip = t("core:zone.raise", "Raise priority"),
					on_click = function()
						act.store_level(z.id, math.min(z.level + 1, top))
					end,
				}),
			})
		)
		local toggles = {}
		for _, it in items do
			local on = z.allows[it.id] == true
			table.insert(
				toggles,
				kit.toggle({
					id = "core:zones." .. z.id .. "." .. it.id,
					label = it.label,
					on = on,
					on_click = function()
						act.zone_allow(z.id, it.id, not on)
					end,
				})
			)
		end
		-- Rows of a few, since a row doesn't wrap.
		for i = 1, #toggles, PER_ROW do
			table.insert(parts, row({ gap = "xs" }, { table.unpack(toggles, i, math.min(i + PER_ROW - 1, #toggles)) }))
		end
	end
	return col({ gap = "s", pad = "m" }, parts)
end)

-- A screen with no key of its own: Z opens the dock's Zones palette, and
-- the list is also a button at its end.
require("@core/ui/screens").add({ id = "core:zones", label = t("core:keys.zones", "Stockpiles"), order = 20 })
ui.extend("core:dock.palette.zones", function(view)
	return kit.button({
		id = "core:zones.open",
		label = t("core:zones.list", "All stockpiles…"),
		active = ui.is_open("core:zones"),
		on_click = function()
			ui.toggle("core:zones")
		end,
	})
end)
