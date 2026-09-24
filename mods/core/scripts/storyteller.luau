-- The core storyteller. Wealth is gravity: it pulls in good and bad alike.
--
-- Exports register_incident, so any plugin can add incidents:
--
--   local storyteller = require("@core/scripts/storyteller")
--   storyteller.register_incident {
--     id = "my_event",
--     kind = "threat" | "good" | "neutral",
--     min_day = 0,
--     weight = 1 or function(ctx) -> number,
--     can_fire = function(ctx) -> bool,        -- optional
--     execute = function(ctx) -> false?,       -- return false if it fizzled
--   }
--
-- ctx = { day, wealth, colonists, strength, points, since_threat }

local incidents = {}

local function register_incident(def)
	assert(type(def.id) == "string", "incident needs an id")
	assert(type(def.execute) == "function", `incident '{def.id}' needs execute(ctx)`)
	def.kind = def.kind or "neutral"
	def.min_day = def.min_day or 0
	table.insert(incidents, def)
end

local GRACE_DAYS = 2 -- no threats before this
local CHECK_TICKS = 500 -- how often we roll
local MTB_DAYS = 0.9 -- mean days between incidents for a poor colony

local state = { last = 0, last_threat = 0 }

-- Exposure is wealth relative to how hard a target the colony is.
-- A rich, well-defended colony draws the same raids as a poorer, weaker one.
local function threat_points(wealth: number, strength: number, day: number): number
	local exposure = math.clamp(wealth / (150 + strength * 60), 0.5, 2.0)
	return math.max(35, (40 + wealth * 0.03 + day * 6) * exposure)
end

-- What incidents are told about the colony.
local function context()
	local tick, tpd = rim.tick(), rim.ticks_per_day
	local ctx = {
		day = tick / tpd,
		wealth = rim.wealth(),
		colonists = rim.colonists(),
		strength = rim.colony_strength(),
		since_threat = (tick - state.last_threat) / tpd,
	}
	ctx.points = threat_points(ctx.wealth, ctx.strength, ctx.day)
	return ctx
end

-- Run an incident's effect now, as if the storyteller had picked it.
local function run(inc, ctx): boolean
	if inc.execute(ctx) == false then
		return false
	end
	state.last = rim.tick()
	if inc.kind == "threat" then
		state.last_threat = state.last
	end
	return true
end

rim.every(CHECK_TICKS, function()
	local colonists = rim.colonists()
	if colonists == 0 then
		return
	end
	local tick = rim.tick()
	local tpd = rim.ticks_per_day
	if tick - state.last < tpd * 0.35 then
		return
	end

	local wealth = rim.wealth()
	-- Gravity: richer colonies draw events more often.
	local pull = math.clamp(1 + wealth / 2500, 1, 3)
	if rim.random() >= CHECK_TICKS / (MTB_DAYS * tpd) * pull then
		return
	end

	local ctx = context()
	local day = ctx.day

	local total, pool = 0, {}
	for _, inc in incidents do
		local ok = day >= inc.min_day and (inc.kind ~= "threat" or day >= GRACE_DAYS)
		if ok and inc.can_fire then
			ok = inc.can_fire(ctx)
		end
		if ok then
			local w = if type(inc.weight) == "function" then inc.weight(ctx) else (inc.weight or 1)
			if inc.kind == "threat" then
				-- Threats become likelier the longer it has been quiet.
				w *= math.clamp(ctx.since_threat, 0.2, 2)
			end
			if w > 0 then
				total += w
				table.insert(pool, { inc = inc, w = w })
			end
		end
	end
	if total <= 0 then
		return
	end

	local r = rim.random() * total
	for _, e in pool do
		r -= e.w
		if r <= 0 then
			run(e.inc, ctx)
			return
		end
	end
end)

rim.on("colony_lost", function()
	rim.message(`The colony lasted {rim.day()} days.`, "bad")
end)

-- Fire an incident by id now, skipping the dice, for tests and tools:
-- fire("raid", { points = 300 }) overrides parts of its context. Returns
-- whether it happened (an incident can fizzle).
local function fire(id: string, overrides: { [string]: any }?): boolean
	for _, inc in incidents do
		if inc.id == id then
			local ctx = context()
			if overrides then
				for k, v in overrides do
					ctx[k] = v
				end
			end
			return run(inc, ctx)
		end
	end
	error(`no incident '{id}'`)
end

-- Registered incident ids, in order.
local function ids(): { string }
	local out = {}
	for _, inc in incidents do
		table.insert(out, inc.id)
	end
	return out
end

return {
	register_incident = register_incident,
	fire = fire,
	ids = ids,
}
