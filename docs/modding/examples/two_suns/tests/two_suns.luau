-- The sky mod beside the weather plugin: two suns, a moon with phases,
-- and a storm still dims them. Tick 0 is 06:00 on day 0.

local MODS = { "core", "weather", "two_suns" }

local function world(t, hours: number)
	local w = t.world({ seed = 2, mods = MODS })
	w:run_hours(hours)
	return w
end

test("core's sun is gone: at 08:00 only the red sun is up", function(t)
	local w = world(t, 2)
	t.expect(w:ambient("daylight")).to_be_at_most(45)
	t.expect(w:ambient("daylight")).to_be_at_least(35)
end)

test("at noon the two suns add up, and no higher than daylight goes", function(t)
	local noon = world(t, 6):ambient("daylight")
	t.expect(noon).to_be_at_least(99)
	t.expect(noon).to_be_at_most(100)
end)

test("the green moon waxes: full at midnight three nights on", function(t)
	-- Midnight on the first night is an eighth of the way round; on the
	-- fourth, halfway: full.
	local first = world(t, 18):ambient("daylight")
	local full = world(t, 3 * 24 + 18):ambient("daylight")
	t.expect(first).to_be_at_most(3.5)
	t.expect(full).to_be_at_least(11.5)
end)

test("a storm dims the suns it has never heard of", function(t)
	local function light(storm: boolean)
		local w = t.world({ seed = 2, mods = MODS })
		w:step()
		w:send("weather:force", { id = if storm then "storm" else "clear", hours = 8 })
		w:run_hours(6)
		return w:ambient("light"), w:ambient("daylight")
	end
	local clear, clear_sky = light(false)
	local storm, storm_sky = light(true)
	t.expect(storm_sky).to_be(clear_sky)
	t.expect(storm).to_be_at_most(clear * 0.8)
end)
