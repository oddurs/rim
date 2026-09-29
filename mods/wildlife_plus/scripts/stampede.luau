-- Tier 2 — script. Adds an incident through core's storyteller module.

local storyteller = require("@core/scripts/storyteller")

-- A boar's melee on colony_strength's scale (wildlife.toml: 8 damage every
-- 80 ticks, per second). A boar never retreats and never leaves, so the
-- herd is sized by the colony's strength as well as its points: at most
-- HERD_SHARE of what the colony can hit back with. Points alone once sent
-- seven against nine colonists, and none of them lived (6ca1ea2e).
local BOAR_STRENGTH = 8 * 60 / 80
local HERD_SHARE = 0.5

storyteller.register_incident({
	id = "boar_stampede",
	kind = "threat",
	min_day = 3,
	weight = 0.4,
	execute = function(ctx)
		local x, y = rim.edge_cell()
		if not x then
			return false
		end
		local bought = ctx.points / 60
		local faced = ctx.strength * HERD_SHARE / BOAR_STRENGTH
		local n = math.clamp(math.floor(math.min(bought, faced)), 2, 8)
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
