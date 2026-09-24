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
