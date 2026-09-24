-- Core's incidents, fired directly: `rim test mods/core`.

local STORYTELLER = "@core/scripts/storyteller"

test("every core incident is registered", function(t)
	local w = t.world({ seed = 7 })
	local ids = w:call(STORYTELLER, "ids")
	for _, id in { "raid", "predators", "wanderer", "herd", "windfall" } do
		t.expect(ids).to_contain(id)
	end
end)

test("a raid brings one raider per 80 points", function(t)
	local w = t.world({ seed = 7 })
	t.expect(w:count_pawns("hostile")).to_be(0)
	t.expect(w:call(STORYTELLER, "fire", "raid", { points = 240 })).to_be(true)
	t.expect(w:count_pawns("hostile")).to_be(3)
	local feed = w:messages()
	t.expect(feed[#feed].text).to_contain("Raid!")
end)

test("raiders give up and leave", function(t)
	local w = t.world({ seed = 7 })
	w:call(STORYTELLER, "fire", "raid", { points = 80 })
	w:run_days(1.5)
	t.expect(w:count_pawns("hostile")).to_be(0)
end)

test("a wanderer joins the colony", function(t)
	local w = t.world({ seed = 3 })
	local before = #w:colonists()
	t.expect(w:call(STORYTELLER, "fire", "wanderer")).to_be(true)
	t.expect(#w:colonists()).to_be(before + 1)
end)

test("a windfall drops supplies", function(t)
	local w = t.world({ seed = 3 })
	w:call(STORYTELLER, "fire", "windfall")
	local feed = w:messages()
	t.expect(feed[#feed].text).to_contain("Supplies have washed up")
end)

test("the storyteller remembers in script data, so saves keep it", function(t)
	local w = t.world({ seed = 7 })
	w:run_hours(2)
	w:call(STORYTELLER, "fire", "raid", { points = 80 })
	local memory = w:data("core:storyteller")
	t.expect(memory.last).to_be(w:tick())
	t.expect(memory.last_threat).to_be(w:tick())
end)
