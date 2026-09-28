-- Counts the hours, in saved data, and lays a pebble down each one: a
-- script, a def of its own and the map, with no core anywhere.
local hours = 0
rim.every(rim.ticks_per_day // 24, function()
	hours = (rim.get_data("hours") or 0) + 1
	rim.set_data("hours", hours)
	local w, h = rim.map_size()
	rim.spawn_item("pebble", w // 2, h // 2, 1)
end)
