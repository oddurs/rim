-- World-anchored labels: names, the sleep marker and speech bubbles.
--
-- The engine places anchored nodes at their pawns and nudges overlapping
-- ones apart by priority, so names never pile on top of each other.

local kit = require("@core/ui/kit")
local t = ui.t

local BUBBLE_TICKS = 900 -- how long a "hello" stays up, in game ticks

local COLOR = { player = "player", hostile = "hostile", wild = "text" }

-- How many pawns get labels in one frame. A crowd of two hundred names is
-- unreadable, and building them all was the frame; the engine caps what
-- it places too, but not building the rest is where the time goes.
local LABEL_CAP = 48

ui.define("core:labels", function(view)
	local out = {}
	local recent = {}
	local since = math.max(0, view.tick() - BUBBLE_TICKS)
	for _, ev in view.events(since) do
		if ev.kind == "joined" then
			recent[ev.id] = ev.name
		end
	end
	-- Pick who gets a label before building any: the selected and hovered
	-- first, then colonists, then hostiles; a greeting always shows.
	local cands = {}
	for _, p in view.visible_pawns() do
		local named = p.faction == "player" or p.selected or p.hovered
		local greeted = recent[p.id] ~= nil
		if named or greeted or p.asleep then
			local priority = if greeted then 4 elseif p.selected or p.hovered then 3 elseif p.faction == "player" then 2 elseif p.faction == "hostile" then 1 else 0
			table.insert(cands, { p = p, named = named, priority = priority })
		end
	end
	table.sort(cands, function(a, b)
		return a.priority > b.priority
	end)
	for i, c in cands do
		if i > LABEL_CAP then
			break
		end
		local p = c.p
		if c.named then
			table.insert(out, {
				kind = "anchored",
				entity = p.id,
				priority = c.priority,
				offset = p.radius + 4,
				kit.label(p.name, { size = "small", color = COLOR[p.faction] or "text", weight = if p.selected then "strong" else nil }),
			})
		end
		if p.asleep then
			table.insert(out, { kind = "anchored", entity = p.id, priority = -1, offset = -(p.radius + 2), kit.label(t("core:labels.sleep", "z"), { weight = "strong" }) })
		end
		if recent[p.id] then
			table.insert(out, {
				kind = "anchored",
				entity = p.id,
				priority = 4,
				offset = -(p.radius + 4),
				kit.bubble({ text = string.format(t("core:labels.hello", "Hello! I'm %s."), recent[p.id]) }),
			})
		end
	end
	return kit.col({}, out)
end)
ui.mount("anchored", "core:labels")
