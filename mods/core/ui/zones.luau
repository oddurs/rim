-- Stockpiles: each zone and the items it takes. Paint zones with the
-- stockpile tool; here a toggle per item lets a zone take it or not.
local kit = require("@core/ui/kit")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

local PER_ROW = 4

ui.window("core:zones", { title = t("core:zones.title", "Stockpiles"), w = 460, h = 320, resizable = true }, function(view)
	local zones = view.zones()
	if #zones == 0 then
		return col({ pad = "m" }, {
			label(t("core:zones.none", "No stockpiles yet. Paint one with the stockpile tool."), { color = "muted", wrap = true }),
		})
	end
	local items = view.items()
	local parts = {}
	for _, z in zones do
		table.insert(parts, kit.heading(z.name, string.format(t("core:zones.cells", "%d cells"), z.cells)))
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

-- Z opens the dock's Zones palette; the list is a button at its end.
ui.bind("core:zones", { label = t("core:keys.zones", "Stockpiles") }, function()
	ui.toggle("core:zones")
end)
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
