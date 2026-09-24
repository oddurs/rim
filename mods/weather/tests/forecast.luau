-- The forecast is the future: it holds four spells, and forcing the
-- weather (as the devtools panel does) takes effect at the next tick.

local WEATHER = "@weather/scripts/weather"

test("the forecast holds the current weather and three more", function(t)
	local w = t.world({ seed = 2 })
	w:run_hours(1) -- the weather hook runs on its own schedule
	t.expect(#w:call(WEATHER, "forecast")).to_be(4)
end)

test("the first day is clear", function(t)
	local w = t.world({ seed = 2 })
	w:run_hours(6)
	t.expect(w:call(WEATHER, "current").id).to_be("clear")
end)

test("a forced storm starts at once", function(t)
	local w = t.world({ seed = 2 })
	w:step()
	w:send("weather:force", { id = "storm", hours = 3 })
	w:step()
	t.expect(w:call(WEATHER, "current").id).to_be("storm")
end)

test("a cold snap makes it colder than the same day without one", function(t)
	local function temperature(snap: boolean)
		local w = t.world({ seed = 2 })
		w:step()
		if snap then
			w:call("@weather/scripts/incidents", "fire", "cold_snap")
		end
		w:run_hours(6)
		return w:ambient("temperature")
	end
	t.expect(temperature(true)).to_be_at_most(temperature(false) - 3)
end)
