-- The weather machine: a registry of weather types and a forecast queue.
--
-- The queue holds the current weather and the next three. Each entry is
-- picked with the world RNG when it joins the queue, so the forecast is the
-- future that will actually happen, unless something forces a change. The
-- queue lives in script data (`weather:forecast`), so it's part of the saved,
-- hashed world, and the UI reads it with view.data.
--
-- Weather sets channels, never visuals: it pushes a "weather" contribution to
-- core's cloud, precipitation, wind, wind_dir, fog and temperature fields,
-- and the renderer draws whatever those say. Precipitation below freezing
-- falls as snow, so winter rain *is* snow.
--
-- Weather types are data: [[type]] in defs/types.toml, [[weather.type]] from
-- other mods (see docs/modding/weather.md). A type that needs code, such as a
-- weight no table can express, registers from a script instead:
--
--   local weather = require("@weather/scripts/weather")
--   weather.register {
--     id = "drizzle", label = "drizzle", cold_label = "flurries",
--     hours = { 3, 8 },            -- how long it lasts
--     blend = 1.5,                 -- hours to ease in from the previous weather
--     weight = function(ctx) return 1 end,  -- or a number; ctx below
--     set = {                      -- a number, or { min, max } picked per spell
--       cloud = { 70, 90 }, precipitation = { 0.3, 1 }, wind = { 1, 4 },
--       fog = 0, temperature = -1,
--     },
--     message = "It's drizzling.", -- optional, posted when it starts
--   }
--
-- ctx = { season, year_fraction, previous, first_day }

local QUEUE = 4 -- the current weather plus three to come
local CHANNELS = { "cloud", "precipitation", "wind", "fog", "temperature" }
local KEY = "weather:forecast"

local types = {}
local order = {} -- registration order, so picking is deterministic

-- Weather's own types can be named bare ("storm" for "weather:storm").
local function type_id(id: string?): string?
	if id == nil or types[id] or string.find(id, ":", 1, true) then
		return id
	end
	return `weather:{id}`
end

local weather = {}

function weather.register(def)
	assert(type(def.id) == "string", "weather needs an id")
	assert(types[def.id] == nil, `weather '{def.id}' is registered twice`)
	assert(type(def.hours) == "table" and #def.hours == 2, `weather '{def.id}' needs hours = \{ min, max \}`)
	def.label = def.label or def.id
	def.blend = def.blend or 1
	def.weight = def.weight or 1
	def.set = def.set or {}
	for k in def.set do
		assert(table.find(CHANNELS, k), `weather '{def.id}': unknown channel '{k}'`)
	end
	types[def.id] = def
	table.insert(order, def.id)
end

function weather.types()
	return order
end

local function tpd()
	return rim.ticks_per_day
end

local function hours(h: number): number
	return math.floor(h * tpd() / 24 + 0.5)
end

-- Where in the year a future tick falls.
local function year_at(tick: number): number
	local d = rim.date()
	local ahead = (tick - rim.tick()) / (tpd() * d.year_days)
	return (d.year_fraction + ahead) % 1
end

local function season_at(tick: number): string
	local seasons = rim.seasons
	return seasons[math.floor(year_at(tick) * #seasons) + 1]
end

local function sample(v): number
	if type(v) == "table" then
		return v[1] + (v[2] - v[1]) * rim.random()
	end
	return v or 0
end

-- Each type's weight to come after `previous`, starting at `start`.
local function weights(previous: string?, start: number)
	local ctx = {
		season = season_at(start),
		year_fraction = year_at(start),
		previous = previous,
		first_day = start < tpd(),
	}
	local pool = {}
	for _, id in order do
		local t = types[id]
		local w = if type(t.weight) == "function" then t.weight(ctx) else t.weight
		if w > 0 then
			table.insert(pool, { id = id, w = w })
		end
	end
	return pool
end

-- Pick what comes after `previous`, starting at `start`.
local function pick(previous: string?, start: number)
	local pool = weights(previous, start)
	local total = 0
	for _, e in pool do
		total += e.w
	end
	assert(total > 0, "no weather can happen now: every weight is zero")
	local r = rim.random() * total
	local chosen = pool[#pool].id
	for _, e in pool do
		r -= e.w
		if r <= 0 then
			chosen = e.id
			break
		end
	end
	local t = types[chosen]
	local len = hours(sample(t.hours))
	local set = {}
	for _, ch in CHANNELS do
		set[ch] = sample(t.set[ch])
	end
	-- Labels travel with the entry so the UI (which can't see this registry)
	-- can show them.
	return {
		id = chosen,
		label = t.label,
		cold_label = t.cold_label,
		start = start,
		ends = start + math.max(len, 1),
		set = set,
	}
end

local function fill(q)
	while #q < QUEUE do
		local last = q[#q]
		table.insert(q, pick(last and last.id, if last then last.ends else rim.tick()))
	end
	-- Wind direction drifts from spell to spell (degrees it blows toward).
	for i = 2, #q do
		if q[i].wind_dir == nil then
			q[i].wind_dir = (q[i - 1].wind_dir or 0) + (rim.random() * 120 - 60)
		end
	end
end

local function label_of(e): string
	local t = types[e.id]
	if t and t.cold_label and e.set.precipitation > 0 and rim.ambient("core:temperature") <= 0 then
		return t.cold_label
	end
	return if t then t.label else e.id
end

-- Push the current weather's channels, easing in over its blend time.
local function apply(e, previous: string?)
	local t = types[e.id]
	for _, ch in CHANNELS do
		rim.push_ambient("core:" .. ch, "weather", e.set[ch], nil, t.blend)
	end
	rim.push_ambient("core:wind_dir", "weather", e.wind_dir or 0, nil, t.blend)
	local label = label_of(e)
	rim.emit("weather:changed", { from = previous, to = e.id, label = label })
	if t.message then
		local text = if type(t.message) == "function" then t.message(label) else t.message
		if text then
			rim.message(text, t.message_kind or "info")
		end
	end
end

local function load_queue()
	return rim.get_data(KEY)
end

-- When the current weather ends: a cache of the queue's head, so the hook
-- does nothing between changes. Derived from script data, never the other
-- way round, so saves and determinism are unaffected.
local next_change: number? = nil

local function save(q)
	rim.set_data(KEY, q)
	next_change = q[1].ends
end

rim.every(20, function()
	if next_change and rim.tick() < next_change then
		return
	end
	local q = load_queue()
	if q == nil then
		-- The types, for the devtools panel (the UI can't see this registry).
		rim.set_data("weather:types", order)
		q = { pick(nil, rim.tick()) }
		q[1].wind_dir = rim.random() * 360
		fill(q)
		apply(q[1], nil)
		save(q)
		return
	end
	local now = rim.tick()
	if now < q[1].ends then
		next_change = q[1].ends
		return
	end
	local previous = q[1].id
	table.remove(q, 1)
	fill(q)
	apply(q[1], previous)
	save(q)
end)

-- ─── API for other mods ─────────────────────────────────────────────────────

-- { id, label, start, ends, set } for the current weather, or nil before the
-- first tick.
function weather.current()
	local q = load_queue()
	if not q then
		return nil
	end
	local e = table.clone(q[1])
	e.label = label_of(e)
	return e
end

-- The current weather and what follows, in order.
function weather.forecast()
	local q = load_queue() or {}
	local out = {}
	for _, e in q do
		table.insert(out, table.clone(e))
	end
	return out
end

-- For tools and tests: the weights and a pick (drawing from the world RNG)
-- for what would follow `previous` at tick `start`.
function weather.weights(previous: string?, start: number)
	local out = {}
	for _, e in weights(type_id(previous), start) do
		out[e.id] = e.w
	end
	return out
end

function weather.pick(previous: string?, start: number)
	return pick(type_id(previous), start)
end

-- Replace the current weather with `id` for `hours`. What was forecast
-- after it still comes, later.
function weather.force(id: string, h: number)
	id = type_id(id) :: string
	local t = types[id]
	assert(t, `unknown weather '{id}'`)
	local q = load_queue() or {}
	local now = rim.tick()
	local previous = q[1] and q[1].id
	local set = {}
	for _, ch in CHANNELS do
		set[ch] = sample(t.set[ch])
	end
	local e = {
		id = id,
		label = t.label,
		cold_label = t.cold_label,
		start = now,
		ends = now + hours(h),
		set = set,
		wind_dir = if q[1] then q[1].wind_dir else rim.random() * 360,
	}
	q[1] = e
	for i = 2, #q do
		local len = q[i].ends - q[i].start
		q[i].start = q[i - 1].ends
		q[i].ends = q[i].start + len
	end
	fill(q)
	apply(e, previous)
	save(q)
end

-- Devtools: the weather plugin's own UI asks for a weather type through a
-- command (act.send), so forcing weather replays and stays in lockstep.
rim.on("weather:force", function(e)
	if types[type_id(e.id)] then
		weather.force(e.id, e.hours or 12)
	end
end)

-- Every [[type]] from any mod, in load order. Bare ids in `follows` are the
-- defining mod's own, like any def reference.
for _, def in rim.defs("type") do
	local owner = string.match(def.id, "^([^:]+):") or "weather"
	local follows = {}
	for k, v in def.follows do
		follows[if string.find(k, ":", 1, true) then k else `{owner}:{k}`] = v
	end
	local season, base, first_day = def.season, def.weight, def.first_day
	local cold_label = if def.cold_label ~= "" then def.cold_label else nil
	local message, cold_message = def.message, def.cold_message
	weather.register({
		id = def.id,
		label = def.label,
		cold_label = cold_label,
		hours = def.hours,
		blend = def.blend,
		set = def.set,
		message_kind = def.message_kind,
		weight = function(ctx)
			if ctx.first_day and not first_day then
				return 0
			end
			return base * (season[ctx.season] or season.default or 1) * (follows[ctx.previous] or 1)
		end,
		message = if message ~= "" or cold_message ~= "" then function(label)
			if label == cold_label and cold_message ~= "" then
				return cold_message
			end
			return if message ~= "" then message else nil
		end else nil,
	})
end

return weather
