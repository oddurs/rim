-- The first days: the founder lives, and the same seed is the same game.

test("the founder survives the first two days", function(t)
	local w = t.world({ seed = 1 })
	w:run_days(2)
	t.expect(#w:colonists()).to_be_at_least(1)
end)

test("the same seed and commands give the same world", function(t)
	local function play()
		local w = t.world({ seed = 11 })
		local x, y = w:colony_center()
		w:designate("chop", x - 10, y - 10, x + 10, y + 10)
		w:run_hours(12)
		return w:hash()
	end
	t.expect(play()).to_be(play())
end)
