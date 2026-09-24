-- Tier 2 — script. Adds an incident through core's storyteller API.

rim.register_incident({
	id = "boar_stampede",
	kind = "threat",
	min_day = 3,
	weight = 0.4,
	execute = function(ctx)
		local x, y = rim.edge_cell()
		if not x then
			return false
		end
		local n = math.clamp(math.floor(ctx.points / 60), 2, 8)
		for _ = 1, n do
			local cx, cy = rim.near_cell(x, y, 3)
			if cx then
				rim.spawn_pawn("boar", "hostile", cx, cy)
			end
		end
		rim.message(`Enraged boars are stampeding toward the colony!`, "threat")
		return true
	end,
})
