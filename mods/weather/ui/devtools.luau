-- Weather devtools (F12): force a weather type, skip ahead, and see what the
-- forecast will do to each channel over the next three days.
--
-- Forcing goes through act.send("weather:force", ...), a command the weather
-- plugin's sim script handles, so it's deterministic like any player input.
-- Skipping ahead (act.advance) runs the simulation forward on this machine:
-- a development tool, not something for a co-op game.

local kit = require("@core/ui/kit")
local row, col, label = kit.row, kit.col, kit.label

-- Channels to chart, with the range a full bar means.
local CHANNELS = {
	{ id = "cloud", label = "cloud", max = 100, color = "muted" },
	{ id = "precipitation", label = "rain", max = 10, color = "accent" },
	{ id = "wind", label = "wind", max = 25, color = "text" },
	{ id = "fog", label = "fog", max = 100, color = "muted" },
}

local STEP_HOURS = 3
local STEPS = 24 -- 72 hours

-- The forecast entry covering `tick`, if the queue reaches that far.
local function entry_at(q, tick: number)
	for _, e in q do
		if tick >= e.start and tick < e.ends then
			return e
		end
	end
	return nil
end

ui.define("weather:devtools", function(view)
	if not view.show_devtools() then
		return nil
	end
	local q = view.data("weather:forecast")
	local types = view.data("weather:types") or {}
	local rows = { kit.heading("Weather", "devtools") }

	local force = {}
	for _, id in types do
		table.insert(force, kit.button({
			id = "weather:devtools.force." .. id,
			label = id,
			active = q and q[1] and q[1].id == id,
			on_click = function()
				act.send("weather:force", { id = id, hours = 12 })
			end,
		}))
	end
	table.insert(rows, label("Force for 12 h", { size = "small", color = "muted" }))
	table.insert(rows, row({ gap = "xs" }, force))

	local skip = {}
	for _, s in { { "+1 h", 1 }, { "+6 h", 6 }, { "+1 day", 24 }, { "+1 season", 24 * 15 } } do
		table.insert(skip, kit.button({
			id = "weather:devtools.advance." .. s[2],
			label = s[1],
			on_click = function()
				act.advance(s[2])
			end,
		}))
	end
	table.insert(rows, label("Skip ahead", { size = "small", color = "muted" }))
	table.insert(rows, row({ gap = "xs" }, skip))

	-- The next 72 hours of each channel, from the forecast queue: one bar per
	-- three hours. The queue is the future, so this is what will happen.
	if q and q[1] then
		local tpd = view.ticks_per_day()
		local now = view.tick()
		table.insert(rows, label("Next 72 hours", { size = "small", color = "muted" }))
		for _, ch in CHANNELS do
			local bars = {}
			for i = 0, STEPS - 1 do
				local e = entry_at(q, now + i * STEP_HOURS * tpd / 24)
				local v = if e and e.set then (e.set[ch.id] or 0) else 0
				local h = math.clamp(v / ch.max, 0, 1) * 18
				table.insert(bars, col({ w = 5, h = 18, justify = "end" }, {
					{ kind = "row", w = 5, h = math.max(1, h), bg = if e then ch.color else "line" },
				}))
			end
			table.insert(rows, row({ id = "weather:devtools.chart." .. ch.id, gap = "xs", align = "end" }, {
				label(ch.label, { size = "small", minw = 44 }),
				row({ gap = 1, align = "end" }, bars),
			}))
		end
	end
	return kit.panel({ id = "weather:devtools.panel", w = 400, gap = "s" }, rows)
end)
ui.mount("right", "weather:devtools", { align = "start", order = -9 })
