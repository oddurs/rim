-- Core incidents. Each is pulled in by the storyteller's gravity.

-- Spawn `n` creatures at one map edge. Returns how many appeared and a name.
local function spawn_group(creature: string, faction: string, n: number)
	local x, y = rim.edge_cell()
	if not x then
		return 0, nil
	end
	local got, name = 0, nil
	for _ = 1, n do
		local cx, cy = rim.near_cell(x, y, 3)
		if cx then
			local id, nm = rim.spawn_pawn(creature, faction, cx, cy)
			if id then
				got += 1
				name = nm
			end
		end
	end
	return got, name
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

local function plural(n: number, word: string): string
	return if n == 1 then `1 {word}` else `{n} {word}s`
end

rim.register_incident({
	id = "raid",
	kind = "threat",
	weight = function(ctx)
		return 1 + ctx.wealth / 3000
	end,
	execute = function(ctx)
		local got = spawn_group("human", "hostile", math.max(1, math.floor(ctx.points / 55)))
		if got == 0 then
			return false
		end
		rim.message(`Raid! {plural(got, "raider")} approaching.`, "threat")
	end,
})

rim.register_incident({
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
		local got = spawn_group(c.id, "hostile", math.max(1, math.floor(ctx.points / 70)))
		if got == 0 then
			return false
		end
		rim.message(`A pack of {plural(got, c.label)} is hunting you!`, "threat")
	end,
})

rim.register_incident({
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
	end,
})

rim.register_incident({
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
	end,
})

rim.register_incident({
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
	end,
})
