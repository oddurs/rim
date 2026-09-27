-- What core's interface has already shown the player: the first day's Auto
-- hint, and which settlers were asked how to work. It lives in script data,
-- not the UI's own state, so a loaded game doesn't ask again. The UI writes
-- it through act.send and reads it as view.data("core:seen").

local KEY = "seen"

rim.on("core:seen", function(e)
	local seen = rim.get_data(KEY) or {}
	if e.hint then
		seen.hint = true
	end
	if e.settler then
		seen.settlers = seen.settlers or {}
		seen.settlers[e.settler] = true
	end
	rim.set_data(KEY, seen)
end)
