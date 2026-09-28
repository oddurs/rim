-- Fire (DESIGN.md §4c): where it burns is this script's data, a list of
-- cells in the order they caught, so a step costs the fire and not the map.
-- Each step a burning cell loses heat (faster in rain and on wet ground,
-- slower while something stands in it to burn), chews at what stands there,
-- and may catch each neighbour: more likely downwind, on dry ground, where
-- there's more to burn. A burning cell carries flames on its floor layer,
-- which give off heat and light and which a colonist beats out near home.
-- Lightning in a storm strikes; in a dry one it catches.
--
-- Everything random is the world RNG, and the lists keep their order, so a
-- fire replays the same from the same seed.

local weather = require("@weather/scripts/weather")

local fire = {}

-- Tuning.
local STEP = 250 -- ticks between steps, about 18 game minutes
local BURN = 0.15 -- heat a bare cell loses a step: grass burns about two hours
local FUELLED = 0.3 -- that, times this, while something stands in it to burn
local RAIN = 0.2 -- more a step per mm/h of rain
local SOAK = 0.004 -- more a step per % of the ground's wetness
local SPREAD = 0.14 -- a step's chance to catch a neighbour, before the rest
local CATCHES = 0.3 -- heat below which a cell no longer spreads
local DRY_AT = 60 -- ground this wet (%) or wetter doesn't catch
local RAINS = 0.5 -- rain (mm/h) that stops a cell catching
local CHEW = 25 -- hit points a step takes off what stands in a burning cell
local HOME = 30 -- cells from the colony's middle within which it's fought
local SCORCH_DAYS = 2 -- burnt ground doesn't catch again for this long

local KEY = "fire:burning"
local SCORCHED = "fire:scorched"

-- Neighbours by octant: 0 east, then clockwise on screen (y grows south),
-- as core's wind_dir counts degrees. COS[k] is the cosine of k octants.
local DIRS = { { 1, 0 }, { 1, 1 }, { 0, 1 }, { -1, 1 }, { -1, 0 }, { -1, -1 }, { 0, -1 }, { 1, -1 } }
local COS = { 1, 0.7071, 0, -0.7071, -1, -0.7071, 0, 0.7071 }

local category: { [string]: string } = {}
for _, t in rim.thing_defs do
	category[t.id] = t.category
end

local function key(x: number, y: number): string
	return `{x},{y}`
end

local function scorched_at(scorched, x: number, y: number, now: number): boolean
	local t = scorched[key(x, y)]
	return t ~= nil and now - t < SCORCH_DAYS * rim.ticks_per_day
end

-- How readily what stands in a cell burns, 0 to 1: a plant wholly, a
-- building by its material (wood's factor is 3), anything else not.
local function fuel(id: number): number
	local t = rim.thing(id)
	if not t or t.blueprint then
		return 0
	end
	if category[t.thing] == "plant" then
		return 1
	end
	return math.min(1, (rim.stat(id, "flammability") or 0) / 3)
end

-- How readily a cell catches: what stands in it, else the ground.
local function flammability(x: number, y: number, scorched, now: number): number
	if scorched_at(scorched, x, y, now) then
		return 0
	end
	local f = rim.fixture_at(x, y)
	if f then
		return fuel(f)
	end
	return rim.terrain_prop(x, y, "flammability")
end

local function load()
	return rim.get_data(KEY) or {}
end

local function near_home(x: number, y: number): boolean
	local cx, cy = rim.colony_center()
	return cx ~= nil and math.max(math.abs(x - cx), math.abs(y - cy)) <= HOME
end

-- Set a cell alight, if it will burn: true if it caught.
local function light(burning, x: number, y: number): boolean
	for _, b in burning do
		if b.x == x and b.y == y then
			return false
		end
	end
	local scorched = rim.get_data(SCORCHED) or {}
	if flammability(x, y, scorched, rim.tick()) <= 0 then
		return false
	end
	local flames = rim.place("fire:flames", x, y)
	if flames and near_home(x, y) then
		rim.designate(flames, "beat_out")
	end
	table.insert(burning, { x = x, y = y, heat = 1, flames = flames })
	return true
end

-- Set a cell on fire from outside: a script, a test, a mod's own cause.
function fire.ignite(x: number, y: number): boolean
	local burning = load()
	local lit = light(burning, x, y)
	rim.set_data(KEY, burning)
	return lit
end

-- Lightning at a cell: it catches under the sky, on dry, burnable ground
-- out of the rain. True if it did.
function fire.strike(x: number, y: number): boolean
	if rim.indoors(x, y) then
		return false
	end
	local rain = rim.field("core:precipitation", x, y)
	local wet = rim.field("weather:wetness", x, y)
	if rain >= RAINS * 2 or wet >= DRY_AT then
		return false
	end
	if not fire.ignite(x, y) then
		return false
	end
	if near_home(x, y) then
		rim.message("Lightning started a fire near the colony.", "threat")
	else
		rim.message("Lightning started a fire.", "info")
	end
	return true
end

-- The cells burning now, in the order they caught.
function fire.burning()
	local out = {}
	for _, b in load() do
		table.insert(out, { x = b.x, y = b.y, heat = b.heat })
	end
	return out
end

-- One step of every fire.
function fire.step()
	local burning = load()
	if #burning == 0 then
		return
	end
	local now = rim.tick()
	local w, h = rim.map_size()
	local wind = rim.ambient("core:wind") or 0
	local toward = math.floor(((rim.ambient("core:wind_dir") or 0) + 22.5) / 45) % 8
	local scorched = rim.get_data(SCORCHED) or {}
	local lit: { [string]: boolean } = {}
	for _, b in burning do
		lit[key(b.x, b.y)] = true
	end
	local still = {}
	local caught = {}
	for _, b in burning do
		local rain = math.max(0, rim.field("core:precipitation", b.x, b.y))
		local wet = rim.field("weather:wetness", b.x, b.y)
		local f = rim.fixture_at(b.x, b.y)
		local fuelled = f ~= nil and fuel(f) > 0
		if fuelled then
			rim.damage(f :: number, CHEW)
		end
		b.heat -= (if fuelled then BURN * FUELLED else BURN) + RAIN * rain + SOAK * wet
		local beaten = b.flames ~= nil and rim.thing(b.flames) == nil
		if b.heat <= 0 or beaten then
			if b.flames then
				rim.remove(b.flames)
			end
			scorched[key(b.x, b.y)] = now
		else
			table.insert(still, b)
			if b.heat > CATCHES and rain < RAINS then
				for k, d in DIRS do
					local x, y = b.x + d[1], b.y + d[2]
					if x >= 0 and y >= 0 and x < w and y < h and not lit[key(x, y)] then
						-- The draw first, against the best this cell could do: most
						-- don't catch, and those cost no reads of the map.
						local downwind = COS[(k - 1 - toward) % 8 + 1]
						local blown = math.max(0.05, 1 + wind / 10 * 1.5 * downwind)
						local roll = rim.random() / (SPREAD * b.heat * blown)
						if roll < 1 and rim.field("core:precipitation", x, y) < RAINS then
							local fl = flammability(x, y, scorched, now)
							local dry = math.clamp(1 - rim.field("weather:wetness", x, y) / DRY_AT, 0, 1)
							if roll < fl * dry then
								lit[key(x, y)] = true
								table.insert(caught, { x, y })
							end
						end
					end
				end
			end
		end
	end
	for _, c in caught do
		light(still, c[1], c[2])
	end
	-- Burnt ground grows back.
	for k, t in scorched do
		if now - t >= SCORCH_DAYS * rim.ticks_per_day then
			scorched[k] = nil
		end
	end
	rim.set_data(KEY, still)
	rim.set_data(SCORCHED, scorched)
end

rim.every(STEP, fire.step)

-- Lightning: each hour, each weather's strikes by chance, somewhere on the
-- surface.
local strikes = {}
for _, def in rim.defs("lightning") do
	local owner = string.match(def.id, "^([^:]+):") or "fire"
	local id = if string.find(def.weather, ":", 1, true) then def.weather else `{owner}:{def.weather}`
	table.insert(strikes, { weather = id, per_hour = def.per_hour })
end

rim.every(rim.ticks_per_day // 24, function()
	local now = weather.current()
	if not now then
		return
	end
	for _, s in strikes do
		if s.weather == now.id and rim.random() < s.per_hour then
			local w, h = rim.map_size()
			fire.strike(rim.random_int(0, w - 1), rim.random_int(0, h - 1))
		end
	end
end)

return fire
