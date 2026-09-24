-- Wildlife+'s scripted incident, through core's storyteller.

test("a stampede brings one boar per 60 points", function(t)
	local w = t.world({ seed = 5 })
	t.expect(w:call("@core/scripts/storyteller", "fire", "boar_stampede", { points = 300 })).to_be(true)
	t.expect(w:count_pawns("hostile")).to_be(5)
end)

test("a small stampede is still two boars", function(t)
	local w = t.world({ seed = 5 })
	w:call("@core/scripts/storyteller", "fire", "boar_stampede", { points = 10 })
	t.expect(w:count_pawns("hostile")).to_be(2)
end)
