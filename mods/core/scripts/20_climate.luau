-- Core climate: a day/night temperature curve and daylight.
--
-- A seasons or weather plugin can take over by calling rim.set_ambient
-- itself (load after core so it has the last word).
--
-- Only plain arithmetic here: math.sin/cos come from each platform's maths
-- library and can differ in the last bit, which would break lockstep.

local BASE = 10 -- mean outdoor temperature, °C
local SWING = 9 -- warmest at 15:00, coldest at 03:00
local WARMEST = 15

local function smoothstep(x: number): number
	return x * x * (3 - 2 * x)
end

-- 1 at the warmest hour, -1 at the coldest, smooth in between.
local function daily(hour: number): number
	local d = math.abs(hour - WARMEST)
	if d > 12 then
		d = 24 - d
	end
	return 1 - 2 * smoothstep(d / 12)
end

-- Full daylight 08:00-18:00, dark 21:00-05:00, ramps between.
local function daylight(hour: number): number
	if hour >= 8 and hour <= 18 then
		return 100
	elseif hour >= 5 and hour < 8 then
		return 100 * smoothstep((hour - 5) / 3)
	elseif hour > 18 and hour <= 21 then
		return 100 * smoothstep((21 - hour) / 3)
	end
	return 0
end

local function update()
	local h = rim.hour()
	rim.set_ambient("temperature", BASE + SWING * daily(h))
	rim.set_ambient("light", daylight(h))
end

rim.every(100, update)
