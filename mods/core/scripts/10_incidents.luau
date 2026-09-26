-- Core incidents. Each is pulled in by the storyteller's gravity.

local storyteller = require("./storyteller")

-- How long a raid lasts before the attackers give up and leave.
local RAID_DAYS = 0.6
-- Threat points per attacker. A raider is worth about one colonist in a
-- fight, so a lone warrior faces one raider and raids grow from there.
local POINTS_PER_RAIDER = 80
local POINTS_PER_PREDATOR = 90

-- Spawn `n` creatures at one map edge. Returns how many appeared and a name.
-- Hostile groups give up after RAID_DAYS.
local function spawn_group(creature: string, faction: "player" | "hostile" | "wild", n: number)
	local x, y = rim.edge_cell()
	if not x then
		return 0, nil
	end
	local got, name, first = 0, nil, nil
	for _ = 1, n do
		local cx, cy = rim.near_cell(x, y, 3)
		if cx then
			local id, nm = rim.spawn_pawn(creature, faction, cx, cy)
			if id then
				if faction == "hostile" then
					rim.leave_after(id, math.floor(RAID_DAYS * rim.ticks_per_day))
				end
				got += 1
				name = nm
				first = first or id
			end
		end
	end
	return got, name, first
end

local function pick_creature(pred: (any) -> boolean)
	local pool = {}
	for _, c in rim.creature_defs do
		if pred(c) then
			table.insert(pool, c)
		end
	end
	if #pool == 0 then
		return nil
	end
	return pool[rim.random_int(1, #pool)]
end

local function count(n: number, one: string, many: string): string
	return if n == 1 then `1 {one}` else `{n} {many}`
end

storyteller.register_incident({
	id = "raid",
	kind = "threat",
	weight = function(ctx)
		return 1 + ctx.wealth / 3000
	end,
	execute = function(ctx)
		local got, _, leader = spawn_group("human", "hostile", math.max(1, math.floor(ctx.points / POINTS_PER_RAIDER)))
		if got == 0 then
			return false
		end
		-- Whoever leads them says why they came, as they come over the edge.
		rim.say(leader, if got == 1 then "Nobody needs to get hurt." else "Take everything!", 600, 3)
		rim.message(`Raid! {count(got, "raider", "raiders")} approaching.`, "threat")
		return true
	end,
})

storyteller.register_incident({
	id = "predators",
	kind = "threat",
	min_day = 1,
	weight = 0.6,
	execute = function(ctx)
		local c = pick_creature(function(c)
			return c.aggressive and not c.intelligent
		end)
		if not c then
			return false
		end
		-- Hunting predators come for the colony: they act as hostiles.
		local got = spawn_group(c.id, "hostile", math.max(1, math.floor(ctx.points / POINTS_PER_PREDATOR)))
		if got == 0 then
			return false
		end
		rim.message(`A pack of {count(got, c.label, c.plural)} is hunting you!`, "threat")
		return true
	end,
})

storyteller.register_incident({
	id = "wanderer",
	kind = "good",
	weight = function(ctx)
		-- Wealth draws people; a crowded colony draws fewer.
		return 1.5 * (1 + ctx.wealth / 1500) / (1 + (ctx.colonists - 1) * 0.5)
	end,
	execute = function(ctx)
		local cx, cy = rim.colony_center()
		if not cx then
			return false
		end
		local x, y = rim.near_cell(cx, cy, 12)
		if not x then
			return false
		end
		local id, name = rim.spawn_pawn("human", "player", x, y)
		if not id then
			return false
		end
		rim.message(`{name}, a wanderer, has heard of your colony and joined it.`, "good")
		return true
	end,
})

storyteller.register_incident({
	id = "herd",
	kind = "neutral",
	weight = 0.8,
	execute = function(ctx)
		local c = pick_creature(function(c)
			return c.wild and c.flees
		end)
		if not c then
			return false
		end
		local got = spawn_group(c.id, "wild", rim.random_int(3, 6))
		if got == 0 then
			return false
		end
		rim.message(`A herd of {c.label} has wandered into the area.`, "info")
		return true
	end,
})

storyteller.register_incident({
	id = "windfall",
	kind = "good",
	weight = 0.5,
	execute = function(ctx)
		local cx, cy = rim.colony_center()
		if not cx then
			return false
		end
		local x, y = rim.near_cell(cx, cy, 10)
		if not x then
			return false
		end
		local choices = { "wood", "berries", "stone" }
		local thing = choices[rim.random_int(1, #choices)]
		local count = math.floor(math.min(150, 25 + ctx.wealth * 0.02))
		rim.spawn_item(thing, x, y, count)
		rim.message(`Supplies have washed up nearby: {count} {thing}.`, "good")
		return true
	end,
})
