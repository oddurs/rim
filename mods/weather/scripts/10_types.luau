-- The weather types. Weights favour what the season brings, and each type
-- leans toward a natural successor: cloud before rain, fog after it.
--
-- Channel units: cloud and fog in %, precipitation in mm/h, wind in m/s,
-- temperature as an offset in °C on top of the seasonal curve.

local weather = require("./weather")

-- The first day is clear: the first night is the shelter test core is tuned
-- around (DESIGN.md §4a), and weather starts after it.
local function by_season(t)
	return function(ctx)
		return t[ctx.season] or t.default or 1
	end
end

local function not_first_day(weight)
	return function(ctx)
		if ctx.first_day then
			return 0
		end
		return if type(weight) == "function" then weight(ctx) else weight
	end
end

-- Multiply a weight by how well this type follows the previous one.
local function after(weight, follows)
	return function(ctx)
		local w = if type(weight) == "function" then weight(ctx) else weight
		return w * (follows[ctx.previous] or 1)
	end
end

weather.register({
	id = "clear",
	label = "clear",
	hours = { 8, 26 },
	blend = 2,
	weight = after(by_season({ spring = 3, summer = 4.5, autumn = 2.5, winter = 2.5 }), { clear = 0.8, fog = 1.4 }),
	set = { cloud = { 0, 20 }, precipitation = 0, wind = { 0.5, 5 }, fog = 0, temperature = { 0, 1.5 } },
})

weather.register({
	id = "cloudy",
	label = "overcast",
	hours = { 4, 16 },
	blend = 2,
	weight = not_first_day(after(by_season({ spring = 2.5, summer = 2, autumn = 3, winter = 3 }), { cloudy = 0.6 })),
	set = { cloud = { 65, 95 }, precipitation = 0, wind = { 2, 7 }, fog = 0, temperature = { -1, 0 } },
})

weather.register({
	id = "rain",
	label = "rain",
	cold_label = "snow",
	hours = { 3, 12 },
	blend = 1.5,
	weight = after(function(ctx)
		if ctx.first_day then
			return 0 -- the first night is about shelter, not a soaking
		end
		return by_season({ spring = 2.2, summer = 1.2, autumn = 2.6, winter = 2 })(ctx)
	end, { cloudy = 1.8, clear = 0.5, rain = 0.5 }),
	set = { cloud = { 85, 100 }, precipitation = { 0.8, 4 }, wind = { 3, 9 }, fog = 0, temperature = { -3.5, -1.5 } },
	message = function(label)
		return if label == "snow" then "It's snowing." else nil
	end,
})

weather.register({
	id = "storm",
	label = "storm",
	cold_label = "blizzard",
	hours = { 2, 7 },
	blend = 1,
	weight = after(function(ctx)
		if ctx.first_day then
			return 0
		end
		return by_season({ spring = 0.25, summer = 0.45, autumn = 0.4, winter = 0.3 })(ctx)
	end, { rain = 1.8, cloudy = 1.2, clear = 0.3, storm = 0.1 }),
	set = { cloud = 100, precipitation = { 5, 9 }, wind = { 12, 20 }, fog = 0, temperature = { -6, -3 } },
	message = function(label)
		return if label == "blizzard" then "A blizzard sweeps in. Get everyone inside." else "A storm breaks. Get everyone inside."
	end,
	message_kind = "threat",
})

weather.register({
	id = "fog",
	label = "fog",
	hours = { 2, 6 },
	blend = 1.5,
	weight = not_first_day(after(by_season({ spring = 0.6, summer = 0.25, autumn = 1.1, winter = 0.7 }), { rain = 1.8, fog = 0.1 })),
	set = { cloud = { 40, 70 }, precipitation = 0, wind = { 0, 1 }, fog = { 60, 90 }, temperature = -0.5 },
})

-- Say so when the season turns.
local SEASON_TEXT = {
	spring = "Spring. The thaw sets in.",
	summer = "Summer. The growing days are long.",
	autumn = "Autumn. Gather what you can before the cold.",
	winter = "Winter has come. Keep a fire lit.",
}

rim.on("season_changed", function(e)
	local text = SEASON_TEXT[e.season] or `It's {e.season}.`
	rim.message(text, if e.season == "winter" then "threat" else "info")
end)
