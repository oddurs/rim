-- People: the colonists, down the left edge. The column keeps to its edge
-- however large the colony gets, and its density follows the count: a card
-- each while there are few, a line each from 9, and lines grouped by what
-- needs attention from 20. Past the room it has, it scrolls.
--
-- Ids: the panel is `core:colonists` and each colonist `core:colonists.<name>`,
-- as they were in the top bar. A mod adds a badge to every row through the
-- module this file returns:
--
--     local people = require("@core/ui/people")
--     people.badge(function(p) return p.asleep and kit.label("z") end)

local kit = require("@core/ui/kit")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

local people = {}

-- From this many, one line each; from GROUPED, grouped as well.
local COMPACT = 9
local GROUPED = 20
local WIDTH = 210
local ROW_H = 24
local CARD_H = 44

local badges: { (any) -> any } = {}

-- Add a badge to every colonist's row: fn(p) returns a node or nil.
function people.badge(fn)
	table.insert(badges, fn)
end

local function pct(v: number): string
	return tostring(math.floor(v * 100 + 0.5)) .. "%"
end

-- What needs the player first: drafted, then hurt, then idle, then the rest.
local GROUPS = {
	{ id = "drafted", label = t("core:colonists.drafted", "Drafted") },
	{ id = "hurt", label = t("core:colonists.hurt", "Hurt") },
	{ id = "idle", label = t("core:colonists.idle", "Idle") },
	{ id = "working", label = t("core:colonists.working", "Working") },
}

local function group_of(p): string
	if p.drafted then
		return "drafted"
	elseif p.health < 0.5 then
		return "hurt"
	elseif p.idle then
		return "idle"
	end
	return "working"
end

local function person(p, compact: boolean)
	local name = label(p.name, { weight = if p.selected then "strong" else "regular", color = if p.drafted then "player" else nil })
	local extra = {}
	for _, b in badges do
		local n = b(p)
		if n then
			table.insert(extra, n)
		end
	end
	local body
	if compact then
		body = row({ gap = "s", align = "center", grow = 1 }, {
			name,
			{ kind = "spacer" },
			label(p.job, { size = "small", color = "muted" }),
			table.unpack(extra),
		})
	else
		body = col({ gap = "xxs", grow = 1 }, {
			row({ gap = "s", align = "center" }, { name, { kind = "spacer" }, table.unpack(extra) }),
			label(p.job, { size = "small", color = "muted" }),
		})
	end
	return row({
		id = "core:colonists." .. p.name,
		w = WIDTH,
		padx = "m",
		gap = "s",
		align = "center",
		bg = if p.selected then "accent_soft" else nil,
		border = if p.selected then "accent" else nil,
		hover = { bg = if p.selected then nil else "surface_hover" },
		press = { bg = "surface_press" },
		tooltip = string.format(t("core:colonists.tooltip", "%s · %s · health %s"), p.label, p.job, pct(p.health)),
		-- Shift adds to the selection; a plain click picks one and looks.
		on_click = function()
			if view.shift() then
				act.select(p.id, true)
			else
				act.select(p.id)
				act.focus(p.id)
			end
		end,
	}, {
		{ kind = "row", w = 3, h = if compact then 12 else 26, bg = if p.health > 0.5 then "good" else "bad", radius = 1 },
		body,
	})
end

ui.define("core:colonists", function(view)
	local all = view.people()
	local n = #all
	if n == 0 then
		return nil
	end
	local compact = n >= COMPACT
	-- One entry per line: a person, or a group's heading. The list builds
	-- only the lines in view, so the column costs the same at 200 as at 20.
	local entries = all
	if n >= GROUPED then
		local by = {}
		for _, p in all do
			local g = group_of(p)
			by[g] = by[g] or {}
			table.insert(by[g], p)
		end
		entries = {}
		for _, g in GROUPS do
			local members = by[g.id]
			if members then
				table.insert(entries, { heading = string.format(t("core:colonists.group", "%s · %d"), g.label, #members) })
				for _, p in members do
					table.insert(entries, p)
				end
			end
		end
	end
	-- minh = 0: the panel shrinks to the room the inspector leaves, and the
	-- list scrolls.
	return kit.panel({ w = WIDTH + 10, pad = "s", gap = "xs", minh = 0 }, {
		label(string.format(t("core:colonists.title", "Colonists · %d"), n), { size = "small", color = "muted" }),
		{
			kind = "list",
			id = "core:colonists.list",
			count = #entries,
			row_h = if compact then ROW_H else CARD_H,
			row = function(i)
				local e = entries[i]
				if e.heading then
					return row({ padx = "xs", align = "end" }, { label(e.heading, { size = "small", color = "faint" }) })
				end
				return person(e, compact)
			end,
		},
	})
end)
ui.mount("left", "core:colonists", { align = "start", order = -10, refresh = "slow" })

return people
