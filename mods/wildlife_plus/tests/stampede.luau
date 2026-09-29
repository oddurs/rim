-- Wildlife+'s scripted incident, through core's storyteller.

test("a stampede brings one boar per 60 points", function(t)
	local w = t.world({ seed = 5 })
	local ctx = { points = 300, strength = 100 }
	t.expect(w:call("@core/scripts/storyteller", "fire", "boar_stampede", ctx)).to_be(true)
	t.expect(w:count_pawns("hostile")).to_be(5)
end)

test("a stampede is no bigger than half what the colony can hit back with", function(t)
	-- Seed 4's colony on day 18: 431 points bought seven boars against a
	-- strength of 31.7, and all nine colonists died (6ca1ea2e).
	local w = t.world({ seed = 5 })
	w:call("@core/scripts/storyteller", "fire", "boar_stampede", { points = 431, strength = 31.7 })
	t.expect(w:count_pawns("hostile")).to_be(2)
	-- Twice as strong, and the same points: five of the seven.
	local strong = t.world({ seed = 5 })
	strong:call("@core/scripts/storyteller", "fire", "boar_stampede", { points = 431, strength = 63.4 })
	t.expect(strong:count_pawns("hostile")).to_be(5)
end)

test("a small stampede is still two boars", function(t)
	local w = t.world({ seed = 5 })
	w:call("@core/scripts/storyteller", "fire", "boar_stampede", { points = 10 })
	t.expect(w:count_pawns("hostile")).to_be(2)
end)
