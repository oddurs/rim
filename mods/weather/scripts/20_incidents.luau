-- Weather incidents, through core's storyteller. Each is a named push or a
-- forced weather, so it shows up by name in the temperature breakdown and
-- in the forecast, and stacks with whatever else is going on.

-- Is a named contribution to the temperature still in effect?
local function active(key: string): boolean
	for _, part in rim.explain("temperature") do
		if part.label == key then
			return true
		end
	end
	return false
end

local function calm(ctx): boolean
	return not active("cold_snap") and not active("heat_wave")
end

rim.register_incident({
	id = "cold_snap",
	kind = "neutral",
	min_day = 3,
	weight = function(ctx)
		local s = rim.season()
		return if s == "winter" then 1.2 elseif s == "autumn" or s == "spring" then 0.6 else 0
	end,
	can_fire = calm,
	execute = function(ctx)
		-- -12°C for two days, easing in over three hours so there's time to act.
		rim.push_ambient("temperature", "cold_snap", -12, 48, 3)
		rim.message("A cold snap is coming. Bring in wood and keep a fire lit.", "threat")
	end,
})

rim.register_incident({
	id = "heat_wave",
	kind = "neutral",
	min_day = 3,
	weight = function(ctx)
		return if rim.season() == "summer" then 0.8 else 0
	end,
	can_fire = calm,
	execute = function(ctx)
		rim.push_ambient("temperature", "heat_wave", 10, 48, 4)
		-- Clear, still skies while it lasts.
		rim.weather.force("clear", 30)
		rim.message("A heat wave settles in.", "bad")
	end,
})

rim.register_incident({
	id = "storm",
	kind = "neutral",
	min_day = 2,
	weight = function(ctx)
		return if rim.season() == "summer" or rim.season() == "autumn" then 0.25 else 0.15
	end,
	can_fire = function(ctx)
		local w = rim.weather.current()
		return w ~= nil and w.id ~= "storm"
	end,
	execute = function(ctx)
		-- The storm's own message says what's happening.
		rim.weather.force("storm", 6 + rim.random() * 6)
	end,
})
