-- The map's orders, as a context menu: a held right-click on the map (or a
-- plain one where nothing safe can be done) opens every order the selected
-- colonists could be given there. The sim decides what's on offer
-- (`view.orders`, from `order::options`); a pick is sent by its key, so
-- the order given is the one the row named, damaging or not.
--
-- The subject is a map spot: kind "tile", id "x,y" or "x,y,<creature>".

local menus = require("@core/ui/menus")
local t = ui.t

local function spot(id)
	local x, y, on = string.match(tostring(id), "^(%-?%d+),(%-?%d+),?(%d*)$")
	if not x then
		return nil
	end
	return { x = tonumber(x), y = tonumber(y), on = if on ~= "" then tonumber(on) else nil }
end

menus.kind("tile", {
	resolve = spot,
	caption = function(ctx)
		return ctx.orders and ctx.orders.caption or nil
	end,
	actor = function(ctx)
		local o = ctx.orders
		if not o or o.actors == 0 then
			return nil
		end
		return if o.actors == 1 then o.actor else string.format(t("core:orders.actors", "%d colonists"), o.actors)
	end,
})

menus.add("tile", {
	id = "core:orders",
	rows = function(ctx)
		local s = ctx.subject
		-- Once per opening: the caption and actor read the same answer.
		ctx.orders = view.orders(s.x, s.y, s.on)
		-- "Deconstruct wall" under the caption "Wall" is "Deconstruct": a row
		-- names the object only when the caption doesn't.
		local caption = string.lower(ctx.orders.caption or "")
		local function short(label: string): string
			local verb, rest = string.match(label, "^(%S+) (.+)$")
			if verb and caption ~= "" and string.lower(rest) == caption then
				return verb
			end
			return label
		end
		local rows = {}
		for _, o in ctx.orders.rows do
			table.insert(rows, {
				label = short(o.label),
				group = o.group,
				trailing = o.trailing,
				disabled = o.disabled,
				run = function()
					act.order(o.key, s.x, s.y, s.on)
				end,
			})
		end
		return rows
	end,
})
