-- Nudges: a word at the stone age's first steps, once each. What has been
-- said lives in script data, so a loaded game doesn't say it again.

local KEY = "said"

local function once(key: string, text: string)
	local said = rim.get_data(KEY) or {}
	if said[key] then
		return
	end
	said[key] = true
	rim.set_data(KEY, said)
	rim.message(text, "good")
end

-- Flint is gathered, not made: look for it lying about.
rim.every(250, function()
	local said = rim.get_data(KEY)
	if not (said and said.flint) and rim.count_items({ thing = "primitive:flint" }) > 0 then
		once("flint", "Flint. Knap it at a crafting spot, with a hammerstone, into a flake or a hand axe.")
	end
end)

-- Tools are made: the crafting plugin says so as each comes off a spot.
-- (A made tool can be in a colonist's hand before any poll would see it.)
local FIRSTS = {
	["primitive:flake"] = { "edge", "A first edge. It cuts a branch into a haft or a digging stick." },
	["primitive:hand_axe"] = { "edge", "A first edge. A hand axe fells an oak, and cuts a branch into a haft." },
	["primitive:hafted_axe"] = { "hafted", "A hafted tool: work goes faster with a handle behind it." },
	["primitive:maul"] = { "hafted", "A hafted tool: work goes faster with a handle behind it." },
}

rim.on("crafting:made", function(e)
	local first = FIRSTS[e.thing]
	if first then
		once(first[1], first[2])
	end
end)
