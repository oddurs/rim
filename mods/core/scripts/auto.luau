-- Auto (DESIGN.md §4d): core's planner for the Auto work role. Once an
-- in-game hour the engine hands it the board and takes back levels for
-- Auto's members. It fills the gaps the rest of the colony leaves, then
-- sets everything else by skill:
--
--   1. Need: a work type's waiting jobs over its `auto.per_person`, rounded
--      up, at most the colony's size. Nothing waiting, nobody needed.
--   2. Gap: minus the colonists at First or Soon whom Auto doesn't move:
--      other roles' members, and pins.
--   3. Fill: every gap gets its first person before any gets a second;
--      larger `waiting × auto.weight` first; best skill wins, less a
--      penalty for each First already given; at most two First each,
--      then Soon.
--   4. Rest: Later if skilled or if the work needs no skill, Spare time if
--      unskilled.
--
-- It never plans never and never touches a pin; the engine refuses both
-- anyway. A mod replaces this by patching `work_role/core:auto` with its
-- own planner.

-- Firsts one colonist may be given.
local MAX_FIRST = 2
-- What each First already given costs, in tenths of a skill level: two
-- levels' worth, so a colonist with one First still wins a job they're
-- three levels better at.
local LOAD = 20
-- A skill at this level or above is worth doing when free.
local SKILLED = 5
-- What a work type with no skill counts as when choosing who fills it.
local NO_SKILL = 5

local function plan(board)
	local levels = board.levels
	local first, soon = 1, math.min(2, levels)
	local later, spare = math.min(3, levels), levels
	local members = {}
	for _, c in board.colonists do
		if c.member then
			table.insert(members, c)
		end
	end
	local out, firsts = {}, {}
	for _, c in members do
		out[c.id] = {}
		firsts[c.id] = 0
	end
	if #members == 0 then
		return out
	end

	-- Needs and gaps, as slots to fill: one per missing person.
	local slots = {}
	for i, w in board.work do
		local need = 0
		if w.waiting > 0 then
			need = math.min(#board.colonists, math.ceil(w.waiting / math.max(w.per_person, 1)))
		end
		local covered = 0
		for _, c in board.colonists do
			local fixed = if c.member then c.pins[w.id] else c.base[w.id]
			if fixed and fixed > 0 and fixed <= soon then
				covered += 1
			end
		end
		for k = 1, need - covered do
			table.insert(slots, { work = w, k = k, i = i, need = need, has = covered + k - 1, score = w.waiting * w.weight })
		end
	end
	table.sort(slots, function(a, b)
		if a.k ~= b.k then
			return a.k < b.k
		end
		if a.score ~= b.score then
			return a.score > b.score
		end
		return a.i < b.i
	end)

	-- Fill: the best fit among members free for this work, in board order
	-- on a tie, so the same board always gives the same plan.
	for _, s in slots do
		local w = s.work
		local best, best_fit = nil, -math.huge
		for _, c in members do
			if c.pins[w.id] == nil and out[c.id][w.id] == nil then
				local skill = if w.skill then c.skills[w.id] or 0 else NO_SKILL
				local fit = skill * 10 - firsts[c.id] * LOAD
				if fit > best_fit then
					best, best_fit = c, fit
				end
			end
		end
		if best then
			local level = soon
			if firsts[best.id] < MAX_FIRST then
				level = first
				firsts[best.id] += 1
			end
			out[best.id][w.id] = {
				level = level,
				reason = string.format("fills %s: %d waiting, wants %d, has %d", w.label, w.waiting, s.need, s.has),
			}
		end
	end

	-- The rest, by skill.
	for _, c in members do
		for _, w in board.work do
			if c.pins[w.id] == nil and out[c.id][w.id] == nil then
				local skill = c.skills[w.id]
				local cell
				if w.waiting == 0 then
					cell = { level = later, reason = "nothing waiting" }
				elseif not w.skill then
					cell = { level = later, reason = "covered; anyone can do it" }
				elseif (skill or 0) >= SKILLED then
					cell = { level = later, reason = string.format("covered; %s %d", w.skill, skill) }
				else
					cell = { level = spare, reason = string.format("covered; low %s (%d)", w.skill, skill or 0) }
				end
				out[c.id][w.id] = cell
			end
		end
	end
	return out
end

rim.planner("auto", plan)
