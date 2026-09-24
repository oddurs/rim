-- The weather in the top bar, and a forecast panel behind it.
--
-- Reads only what the sim publishes: outdoor values (view.ambient), their
-- breakdown (view.explain) and the forecast queue the weather script keeps in
-- script data (view.data("weather:forecast")).

local kit = require("@core/ui/kit")
local row, col, label = kit.row, kit.col, kit.label

local ARROWS = { "→", "↘", "↓", "↙", "←", "↖", "↑", "↗" }

-- An arrow for where the wind blows (0° = east, 90° = south).
local function arrow(deg: number): string
	local i = math.floor(((deg % 360) + 22.5) / 45) % 8
	return ARROWS[i + 1]
end

local function cap(s: string): string
	return string.upper(string.sub(s, 1, 1)) .. string.sub(s, 2)
end

-- The label to show: rain below freezing is snow.
local function label_of(e, view): string
	local cold = e.cold_label and e.set and (e.set.precipitation or 0) > 0 and (view.ambient("temperature") or 0) <= 0
	return if cold then e.cold_label else (e.label or e.id)
end

-- "in 5 h", "tomorrow 14:00".
local function when(view, tick: number): string
	local tpd = view.ticks_per_day()
	local now = view.tick()
	local hours = (tick - now) / tpd * 24
	if hours < 1 then
		return "within the hour"
	elseif hours < 12 then
		return string.format("in %d h", math.floor(hours + 0.5))
	end
	-- Tick 0 is 06:00 on day 1.
	local abs_hours = tick / tpd * 24 + 6
	local day = math.floor(abs_hours / 24) + 1
	local today = math.floor((now / tpd * 24 + 6) / 24) + 1
	local clock = string.format("%02d:00", math.floor(abs_hours % 24))
	local d = if day == today then "today" elseif day == today + 1 then "tomorrow" else "day " .. day
	return d .. " " .. clock
end

ui.extend("core:topbar.right", function(view)
	local q = view.data("weather:forecast")
	if not q or not q[1] then
		return nil
	end
	local wind = view.ambient("wind") or 0
	local text = string.format(
		"%s · %.0f°C · %s %.0f m/s",
		cap(label_of(q[1], view)),
		view.ambient("temperature") or 0,
		arrow(view.ambient("wind_dir") or 0),
		wind
	)
	local open = ui.state("weather:open", false)
	return kit.button({
		id = "weather:readout",
		label = text,
		active = open,
		tooltip = "Forecast",
		on_click = function()
			ui.set_state("weather:open", not open)
		end,
	})
end)

ui.define("weather:forecast", function(view)
	if not ui.state("weather:open", false) then
		return nil
	end
	local q = view.data("weather:forecast")
	if not q or not q[1] then
		return nil
	end
	local d = view.date()
	local items = {
		kit.heading("Weather", cap(d.season) .. " " .. d.day),
		label(cap(label_of(q[1], view)) .. ", until " .. when(view, q[1].ends), { id = "weather:forecast.now", weight = "strong" }),
	}
	for i = 2, #q do
		local e = q[i]
		table.insert(
			items,
			row({ id = "weather:forecast." .. i, gap = "m" }, {
				label(when(view, e.start), { color = "muted", size = "small", minw = 110 }),
				label(cap(e.label or e.id), { size = "small" }),
			})
		)
	end
	table.insert(items, kit.divider())
	table.insert(items, label("Temperature", { size = "small", color = "muted" }))
	for _, part in view.explain("temperature") do
		table.insert(
			items,
			row({ gap = "m" }, {
				label(part.label, { size = "small", minw = 110 }),
				label(string.format("%+.1f°", part.value), { size = "small", color = if part.value < 0 then "weather_cold" else "weather_warm" }),
			})
		)
	end
	return kit.panel({ id = "weather:forecast.panel", gap = "xs", minw = 240 }, items)
end)
ui.mount("right", "weather:forecast", { align = "start", order = 5 })
