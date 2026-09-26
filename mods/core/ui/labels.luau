-- World-anchored labels: names, the sleep marker and speech bubbles.
--
-- The engine places anchored nodes at their pawns and nudges overlapping
-- ones apart by priority, so names never pile on top of each other.
--
-- Speech comes from view.speech(): a need's line as it runs low, a
-- script's rim.say, and here, a greeting when someone joins. A pawn shows
-- one line at a time, its most important, and only a few bubbles show at
-- once: a crowd all talking is noise, and each is a laid-out panel.

local kit = require("@core/ui/kit")
local t = ui.t

local BUBBLE_TICKS = 900 -- how long a "hello" stays up, in game ticks

local COLOR = { player = "player", hostile = "hostile", wild = "text" }

-- How many pawns get labels in one frame. A crowd of two hundred names is
-- unreadable, and building them all was the frame; the engine caps what
-- it places too, but not building the rest is where the time goes.
local LABEL_CAP = 48

-- How many speech bubbles show at once, most important first.
local BUBBLE_CAP = 6

ui.define("core:labels", function(view)
	local out = {}
	-- What each pawn is saying: its most important line, the newest on a tie.
	local said = {}
	local function hear(id, line)
		local cur = said[id]
		if not cur or line.priority > cur.priority or (line.priority == cur.priority and line.age < cur.age) then
			said[id] = line
		end
	end
	for _, s in view.speech() do
		hear(s.id, s)
	end
	local now = view.tick()
	for _, ev in view.events(math.max(0, now - BUBBLE_TICKS)) do
		if ev.kind == "joined" then
			local text = string.format(t("core:labels.hello", "Hello! I'm %s."), ev.name)
			hear(ev.id, { text = text, age = (now - ev.tick) / BUBBLE_TICKS, priority = 3 })
		end
	end
	-- Pick who gets a label before building any: the selected and hovered
	-- first, then colonists, then hostiles; a greeting always shows.
	local cands = {}
	for _, p in view.visible_pawns() do
		local named = p.faction == "player" or p.selected or p.hovered
		local line = said[p.id]
		if named or line or p.asleep then
			local priority = if line then 4 elseif p.selected or p.hovered then 3 elseif p.faction == "player" then 2 elseif p.faction == "hostile" then 1 else 0
			table.insert(cands, { p = p, named = named, priority = priority, line = line })
		end
	end
	table.sort(cands, function(a, b)
		return a.priority > b.priority
	end)
	-- Which speakers get a bubble: the most important lines, then the newest.
	local speakers = {}
	for _, c in cands do
		if c.line then
			table.insert(speakers, c)
		end
	end
	table.sort(speakers, function(a, b)
		if a.line.priority ~= b.line.priority then
			return a.line.priority > b.line.priority
		end
		return a.line.age < b.line.age
	end)
	local bubble = {}
	for i = 1, math.min(BUBBLE_CAP, #speakers) do
		bubble[speakers[i].p.id] = speakers[i].line
	end
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
		local line = bubble[p.id]
		if line then
			table.insert(out, {
				kind = "anchored",
				entity = p.id,
				priority = 4 + line.priority,
				offset = -(p.radius + 4),
				kit.bubble({ text = line.text, age = line.age }),
			})
		end
	end
	return kit.col({}, out)
end)
ui.mount("anchored", "core:labels")
