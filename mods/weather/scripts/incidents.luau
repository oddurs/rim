-- Weather incidents, through core's storyteller. Each is a named push or a
-- forced weather, so it shows up by name in the temperature breakdown and
-- in the forecast, and stacks with whatever else is going on.

local storyteller = require("@core/scripts/storyteller")
local weather = require("./weather")

-- Is a named contribution to the temperature still in effect?
local function active(key: string): boolean
	for _, part in rim.explain("core:temperature") do
		if part.label == key then
			return true
		end
	end
	return false
end

local function calm(ctx): boolean
	return not active("cold_snap") and not active("heat_wave")
end

-- Each incident's effect, by id, so tools and tests can set one off:
-- require("@weather/scripts/incidents").fire("cold_snap").
local effects = {}
local incidents = {}

local function register(def)
	effects[def.id] = def.execute
	storyteller.register_incident(def)
end

function incidents.fire(id: string)
	local f = effects[id]
	assert(f, `unknown weather incident '{id}'`)
	f({})
end

register({
	id = "cold_snap",
	kind = "neutral",
	min_day = 5,
	weight = function(ctx)
		local s = rim.season()
		return if s == "winter" then 1.2 elseif s == "autumn" or s == "spring" then 0.6 else 0
	end,
	can_fire = calm,
	execute = function(ctx)
		-- Two days of cold, easing in over three hours so there's time to
		-- act: -12°C in winter, -8°C in spring and autumn.
		local drop = if rim.season() == "winter" then -12 else -8
		rim.push_ambient("core:temperature", "cold_snap", drop, 48, 3)
		rim.message("A cold snap is coming. Bring in wood and keep a fire lit.", "threat")
	end,
})

register({
	id = "heat_wave",
	kind = "neutral",
	min_day = 3,
	weight = function(ctx)
		return if rim.season() == "summer" then 0.8 else 0
	end,
	can_fire = calm,
	execute = function(ctx)
		rim.push_ambient("core:temperature", "heat_wave", 10, 48, 4)
		-- Clear, still skies while it lasts.
		weather.force("clear", 30)
		rim.message("A heat wave settles in.", "bad")
	end,
})

register({
	id = "storm",
	kind = "neutral",
	min_day = 3,
	weight = function(ctx)
		return if rim.season() == "summer" or rim.season() == "autumn" then 0.25 else 0.15
	end,
	can_fire = function(ctx)
		local w = weather.current()
		return w ~= nil and w.id ~= "storm"
	end,
	execute = function(ctx)
		-- The storm's own message says what's happening.
		weather.force("storm", 6 + rim.random() * 6)
	end,
})

return incidents
