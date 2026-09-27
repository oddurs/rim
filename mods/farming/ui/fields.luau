-- A growing zone in the inspector, and in its menu: what it sows, and the
-- other crops it could. Picking one changes the field; what's already
-- growing there stays until it's harvested.

local kit = require("@core/ui/kit")
local inspector = require("@core/ui/inspector")
local menus = require("@core/ui/menus")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

local function field(sel): boolean
	return sel.kind == "zone" and sel.plant ~= nil
end

inspector.tab({
	id = "farming:field",
	-- A function, so the string is asked for only when a field is selected.
	label = function()
		return t("farming:field.tab", "Field")
	end,
	applies = field,
	build = function(view, sel)
		local crops = {}
		for _, c in view.crops() do
			table.insert(
				crops,
				kit.button({
					id = "farming:field.crop." .. c.id,
					label = c.label,
					active = c.id == sel.plant,
					on_click = function()
						act.zone_plant(sel.id, c.id)
					end,
				})
			)
		end
		return col({ gap = "s" }, {
			kit.heading(sel.name, string.format(t("farming:field.cells", "%d cells"), sel.cells)),
			label(
				string.format(t("farming:field.sows", "Sows %s while the season allows it to grow, and harvests what's grown."), sel.plant_label or ""),
				{ color = "muted", wrap = true }
			),
			row({ gap = "xs" }, crops),
		})
	end,
})

menus.add("zone", {
	id = "farming:zone.crop",
	group = "manage",
	applies = function(ctx)
		return ctx.subject.plant ~= nil
	end,
	rows = function(ctx)
		local rows = {}
		for _, c in view.crops() do
			if c.id ~= ctx.subject.plant then
				table.insert(rows, {
					label = string.format(t("farming:zone.grow", "Grow %s"), c.label),
					run = function()
						act.zone_plant(ctx.id, c.id)
					end,
				})
			end
		end
		return rows
	end,
})
