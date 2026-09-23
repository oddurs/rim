-- World-anchored labels: names, the sleep marker and speech bubbles.
--
-- The engine places anchored nodes at their pawns and nudges overlapping
-- ones apart by priority, so names never pile on top of each other.

local kit = require("@core/ui/kit")

local BUBBLE_TICKS = 900 -- how long a "hello" stays up, in game ticks

local COLOR = { player = "player", hostile = "hostile", wild = "text" }

ui.define("core:labels", function(view)
	local out = {}
	local recent = {}
	local since = math.max(0, view.tick() - BUBBLE_TICKS)
	for _, ev in view.events(since) do
		if ev.kind == "joined" then
			recent[ev.id] = ev.name
		end
	end
	for _, p in view.visible_pawns() do
		-- Colonists are always named; others only when you point at them.
		local named = p.faction == "player" or p.selected or p.hovered
		if named then
			local priority = if p.selected then 3 elseif p.faction == "player" then 2 elseif p.faction == "hostile" then 1 else 0
			table.insert(out, {
				kind = "anchored",
				entity = p.id,
				priority = priority,
				offset = p.radius + 4,
				kit.label(p.name, { size = "small", color = COLOR[p.faction] or "text", weight = if p.selected then "strong" else nil }),
			})
		end
		if p.asleep then
			table.insert(out, { kind = "anchored", entity = p.id, priority = -1, offset = -(p.radius + 2), kit.label("z", { weight = "strong" }) })
		end
		if recent[p.id] then
			table.insert(out, {
				kind = "anchored",
				entity = p.id,
				priority = 4,
				offset = -(p.radius + 4),
				kit.bubble({ text = "Hello! I'm " .. recent[p.id] .. "." }),
			})
		end
	end
	return kit.col({}, out)
end)
ui.mount("anchored", "core:labels")
