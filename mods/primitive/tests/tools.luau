-- The stone age's climb, played: stones and flint, a crafting spot, a
-- hammerstone, a hand axe, and only then a felled oak.

-- The oak nearest a cell.
local function nearest(w, thing: string, x: number, y: number)
	local best, bd = nil, math.huge
	for _, t in w:things(thing) do
		local d = math.max(math.abs(t.x - x), math.abs(t.y - y))
		if d < bd then
			best, bd = t, d
		end
	end
	return best
end

-- A colony with a crafting spot beside it, and stones and flint to hand.
local function camp(t)
	local w = t.world({ seed = 7 })
	w:step()
	local x, y = w:colony_center()
	local sx, sy = w:open_cell(x + 2, y)
	w:build("crafting:spot", sx, sy)
	w:spawn_item("primitive:stones", x, y, 2)
	w:spawn_item("primitive:flint", x, y, 2)
	w:run_hours(2)
	local spot = w:things("crafting:spot")[1]
	t.expect(spot and not spot.blueprint).to_be(true)
	return w, spot, x, y
end

local function add(w, spot, recipe: string)
	w:send("crafting:add_bill", { site = spot.id, recipe = recipe })
	w:step()
end

test("an oak waits for an axe", function(t)
	local w, _, x, y = camp(t)
	local oak = nearest(w, "core:tree_oak", x, y)
	w:designate("core:chop", oak.x, oak.y, oak.x, oak.y)
	w:run_hours(8)
	t.expect(nearest(w, "core:tree_oak", x, y).id).to_be(oak.id)
end)

test("bare hands to a felled tree", function(t)
	local w, spot, x, y = camp(t)
	add(w, spot, "primitive:hammerstone")
	add(w, spot, "primitive:hand_axe")
	w:run_hours(10)
	local axes = w:things("primitive:hand_axe")
	t.expect(#w:things("primitive:hammerstone")).to_be(1)
	t.expect(#axes).to_be(1)
	t.expect(axes[1].made_of).to_be("primitive:flint")

	local oak = nearest(w, "core:tree_oak", x, y)
	local wood = #w:things("core:wood")
	w:designate("core:chop", oak.x, oak.y, oak.x, oak.y)
	w:run_hours(10)
	t.expect(nearest(w, "core:tree_oak", x, y).id).never_to_be(oak.id)
	t.expect(#w:things("core:wood")).to_be_at_least(wood + 1)
end)
