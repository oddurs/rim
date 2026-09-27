-- Colony readings for core's standing orders (DESIGN.md §4d): the numbers
-- the reading rules in defs/colony.toml watch. Published hourly from the
-- stock ledger, so each costs a lookup per kind of item, never a scan.
--
--   core:food_days    days the colony's food lasts its colonists
--   core:loose_items  items lying outside a stockpile that takes them
--   core:wood         wood the colony has

local HOUR = rim.ticks_per_day // 24

-- What a unit of each food restores.
local foods: { { id: string, nutrition: number } } = {}
for _, d in rim.thing_defs do
	if d.nutrition then
		table.insert(foods, { id = d.id, nutrition = d.nutrition })
	end
end

-- The top of the item category tree: every item is under one of these, so
-- a loose count is a lookup per branch, not per kind of item.
local roots: { string } = {}
for _, c in rim.item_categories do
	if c.parent == nil then
		table.insert(roots, c.id)
	end
end

-- A full stomach a day per colonist, at the food need's rate.
local stomachs_per_day = 0.5
for _, n in rim.need_defs do
	if n.satisfier == "food" and n.days_to_empty > 0 then
		stomachs_per_day = 1 / n.days_to_empty
	end
end

local function publish()
	local fed = 0
	for _, f in foods do
		fed += rim.stock(f.id) * f.nutrition
	end
	local eaters = math.max(rim.colonists(), 1)
	rim.set_reading("food_days", fed / (eaters * stomachs_per_day))

	local loose = 0
	for _, id in roots do
		loose += rim.stock({ category = id }, "loose")
	end
	rim.set_reading("loose_items", loose)

	rim.set_reading("wood", rim.stock("core:wood"))
end

rim.every(HOUR, publish)

-- A standing order never acts silently: starting and stopping make the news,
-- whichever mod's order it is.
local function reading_text(v: number?): string
	if v == nil then
		return ""
	end
	return string.format(" (%s)", if v == math.floor(v) then tostring(v) else string.format("%.1f", v))
end

rim.on("rule_started", function(e)
	if e.reading then
		rim.message(`Standing order on: {e.label}{reading_text(e.value)}.`, "info")
	end
end)

rim.on("rule_stopped", function(e)
	if e.reading then
		rim.message(`Standing order off: {e.label}{reading_text(e.value)}.`, "info")
	end
end)
