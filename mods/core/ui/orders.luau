-- The map's orders, as a context menu: a held right-click on the map (or a
-- plain one where nothing safe can be done) opens every order the selected
-- colonists could be given there. The sim decides what's on offer
-- (`view.orders`, from `order::options`); a pick is sent by its key, so
-- the order given is the one the row named, damaging or not.
--
-- The subject is a map spot: kind "tile", id "x,y" or "x,y,<creature>".

local kit = require("@core/ui/kit")
local menus = require("@core/ui/menus")
local t = ui.t

-- How long an order can be taken back from its toast, in seconds.
local UNDO_SECS = 5

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

-- ─── Urgent ────────────────────────────────────────────────────────────────
-- One job before the plan (DESIGN.md §4d): everyone takes it a level
-- sooner than its work type, and the mark goes when the job does.
local function toggle_urgent(x: number, y: number): boolean
	local m = view.markable(x, y)
	if not m then
		return false
	end
	act.mark_urgent(m.id, not m.urgent)
	return true
end

menus.add("tile", {
	id = "core:urgent",
	group = "work",
	key = "u",
	applies = function(ctx)
		return view.markable(ctx.subject.x, ctx.subject.y) ~= nil
	end,
	label = function(ctx)
		local m = view.markable(ctx.subject.x, ctx.subject.y)
		return if m and m.urgent then t("core:urgent.clear", "Clear urgent") else t("core:urgent.mark", "Mark urgent")
	end,
	run = function(ctx)
		toggle_urgent(ctx.subject.x, ctx.subject.y)
	end,
})

ui.bind("core:urgent", { key = "u", label = t("core:keys.urgent", "Mark the job under the cursor urgent") }, function()
	local h = view.hover()
	if h then
		toggle_urgent(h.x, h.y)
	end
end)

-- The HUD counts live marks, so a colony that marked everything sees it.
ui.extend("core:topbar.right", function(view)
	local n = view.urgent_count()
	if n == 0 then
		return nil
	end
	return kit.label(string.format(t("core:urgent.count", "%d urgent"), n), { id = "core:urgent.count", color = "bad" })
end)

-- ─── Taking an order back ──────────────────────────────────────────────────
-- For a few seconds after an order, a chip in the dock bar says what it was,
-- with Undo; Cmd/Ctrl+Z does the same for the newest order. It sits in the
-- bar rather than over it, so it never moves the panels docked above.
ui.bind("core:undo", { key = "ctrl+z", label = t("core:keys.undo", "Undo the last order") }, function()
	act.undo()
end)

ui.extend("core:dock.status", function(view)
	local o = view.last_order()
	if not o or o.age > UNDO_SECS then
		return nil
	end
	return kit.row({ id = "core:undo", gap = "m", align = "center", padx = "m", border = "line_strong", radius = "radius" }, {
		kit.label(o.label, { size = "small" }),
		kit.button({
			id = "core:undo.button",
			label = t("core:undo.label", "Undo"),
			hint = "⌘Z",
			pady = "xs",
			on_click = function()
				act.undo()
			end,
		}),
	})
end)
