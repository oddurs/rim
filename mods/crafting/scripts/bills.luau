-- Bills: what each station is told to make, turned into work orders.
--
-- A station is any built thing whose tags name a recipe's `station`. Its
-- bills live in script data (`crafting:bills`), so they're saved and hashed
-- with the world, and the bills panel reads them with view.data. Every so
-- often each station with no order takes the first bill that can run and
-- posts it as one work order; when the order is done, the outputs appear at
-- the station.
--
-- A station is { site, bills, making }, where `making` is { bill, recipe }
-- while its order is up. A bill is { id, recipe, mode, target, done,
-- suspended, why, filters }:
--   mode "count"   make `target` more, then the bill is finished
--   mode "until"   make while there are fewer than `target` of the first output
--   mode "forever" keep making
-- `why` says what stops it, for the panel ("no flint").
-- `filters` narrows a tag input to some of its things, by input index:
-- { [2] = { allows = { "primitive:flint" } } } is flint only. It is the
-- engine's item filter (DESIGN.md §4f), passed with the order's need.
--
-- An input taken by several comes in one material unless the recipe says
-- `mix = true`: the engine fills it with pieces matching the first. So a
-- bill can run only when one allowed thing has enough, not when the tag
-- has enough in all.
--
-- The panel changes bills by sending these, which arrive here as events and
-- call the functions this module exports (add, set, move, remove):
--   crafting:add_bill    { site, recipe }
--   crafting:set_bill    { site, bill, mode?, target?, suspended?,
--                          ingredient? = { input, thing, on } }
--   crafting:move_bill   { site, bill, by }      -- -1 up, +1 down
--   crafting:remove_bill { site, bill }

local KEY = "crafting:bills"
local NEXT = "crafting:next_bill"
local CATALOG = "crafting:catalog"
local MODES = { count = true, ["until"] = true, forever = true }
local MAX_TARGET = 999

-- What the defs say, which never changes during a game: locals are safe.
local things = {} -- id -> { label, item, tags = { [tag] = true } }
local tagged = {} -- tag -> true, if any thing carries it
for _, t in rim.thing_defs do
	local tags = {}
	for _, tag in t.tags do
		tags[tag] = true
		tagged[tag] = true
	end
	things[t.id] = { label = t.label, item = t.item, tags = tags }
end

local recipes = {} -- id -> recipe, references qualified
local recipe_order = {} -- load order, so the catalog is deterministic
local speeds = {} -- station thing id -> speed

-- A def reference in an entry: bare names are the declaring mod's own.
local function qualify(id: string, owner: string): string
	if string.find(id, ":", 1, true) then
		return id
	end
	return `{owner}:{id}`
end

local function owner_of(id: string): string
	return string.match(id, "^([^:]+):") or "crafting"
end

local function thing_ref(id: any, owner: string, what: string): string
	assert(type(id) == "string", `{what} needs a thing`)
	local full = qualify(id, owner)
	assert(things[full], `{what}: no thing '{full}'`)
	return full
end

local function count_of(n: any, what: string): number
	n = n or 1
	assert(type(n) == "number" and n >= 1 and n == math.floor(n), `{what} needs a whole count of at least 1`)
	return n
end

for _, def in rim.defs("recipe") do
	local owner = owner_of(def.id)
	local where = `recipe '{def.id}'`
	local inputs = {}
	for i, input in def.inputs do
		local n = count_of(input.count, `{where} input {i}`)
		local mix = input.mix == true
		if input.tag then
			if not tagged[input.tag] then
				rim.log(`{where}: nothing is tagged '{input.tag}', so it can't be made`)
			end
			table.insert(inputs, { tag = input.tag, count = n, mix = mix })
		else
			table.insert(inputs, { thing = thing_ref(input.thing, owner, `{where} input {i}`), count = n, mix = mix })
		end
	end
	local outputs = {}
	for i, output in def.outputs do
		local n = count_of(output.count, `{where} output {i}`)
		table.insert(outputs, { thing = thing_ref(output.thing, owner, `{where} output {i}`), count = n })
	end
	assert(#outputs > 0, `{where} makes nothing: give it outputs`)
	assert(def.work > 0, `{where}: work must be more than 0`)
	if not tagged[def.station] then
		rim.log(`{where}: no thing is a '{def.station}' station yet`)
	end
	recipes[def.id] = {
		id = def.id,
		label = def.label,
		station = def.station,
		inputs = inputs,
		outputs = outputs,
		work = def.work,
		requires = def.requires,
		work_type = qualify(def.work_type, owner),
		stuff = def.stuff,
	}
	table.insert(recipe_order, def.id)
end

for _, def in rim.defs("station") do
	local t = thing_ref(def.thing, owner_of(def.id), `station '{def.id}'`)
	assert(def.speed > 0, `station '{def.id}': speed must be more than 0`)
	speeds[t] = def.speed
end

-- The recipes a thing can make, in load order.
local function recipes_at(thing: string): { string }
	local out = {}
	local t = things[thing]
	if t then
		for _, id in recipe_order do
			if t.tags[recipes[id].station] then
				table.insert(out, id)
			end
		end
	end
	return out
end

local function label_of(input): string
	if input.thing then
		return things[input.thing].label
	end
	return input.tag
end

-- The things that could fill an input, in load order.
local function candidates(input): { string }
	if input.thing then
		return { input.thing }
	end
	local out = {}
	for _, t in rim.thing_defs do
		if things[t.id].tags[input.tag] then
			table.insert(out, t.id)
		end
	end
	return out
end

-- The things a bill lets fill input `i`, and the filter's allow list.
local function allowed(b, i: number, input): ({ string }, { [string]: boolean }?)
	local f = b.filters and b.filters[i]
	if not f then
		return candidates(input), nil
	end
	local set = {}
	for _, id in f.allows do
		set[id] = true
	end
	local out = {}
	for _, id in candidates(input) do
		if set[id] then
			table.insert(out, id)
		end
	end
	return out, set
end

-- How an input reads in a reason: its one allowed thing, if it's narrowed
-- to one, or its tag.
local function short_label(input, ids: { string }): string
	if #ids == 1 then
		return things[ids[1]].label
	end
	return label_of(input)
end

-- ─── State ──────────────────────────────────────────────────────────────────

local function load(): { any }
	return rim.get_data(KEY) or {}
end

local function station_of(state, site: number)
	for _, st in state do
		if st.site == site then
			return st
		end
	end
	return nil
end

local function bill_index(st, id: number): number?
	for i, b in st.bills do
		if b.id == id then
			return i
		end
	end
	return nil
end

-- Items lying about, counted once per hook however many bills ask.
local function counter()
	local seen = {}
	return function(q): number
		local key = if q.thing then "thing " .. q.thing else "tag " .. q.tag
		if seen[key] == nil then
			seen[key] = rim.count_items(q)
		end
		return seen[key]
	end
end

-- Why input `i` can't be filled now, or nil. One material for the whole
-- input, so it's the allowed thing with the most that counts: one flint
-- and one bone are not two of anything. A mixed input counts them all.
local function short(b, i: number, input, count): string?
	local ids = allowed(b, i, input)
	local best, most, all = nil, 0, 0
	for _, id in ids do
		local have = count({ thing = id })
		all += have
		if have > most then
			best, most = id, have
		end
	end
	if input.mix then
		if all >= input.count then
			return nil
		end
		return if all == 0 then `no {short_label(input, ids)}` else `{all} of {input.count} {short_label(input, ids)}`
	end
	if most >= input.count then
		return nil
	end
	return if most == 0 then `no {short_label(input, ids)}` else `{most} of {input.count} {things[best].label}`
end

-- What stops a bill now, or nil if it can run.
local function blocked(b, count): string?
	local r = recipes[b.recipe]
	if not r then
		return "its recipe is gone"
	end
	-- A bill waiting on a tool nobody has would hold the station: the
	-- bill below it might be the one that makes the tool.
	if #r.requires > 0 and not rim.has_tool(r.requires) then
		return `needs a {table.concat(r.requires, " and ")} tool`
	end
	if b.mode == "until" then
		local have = count({ thing = r.outputs[1].thing })
		if have >= b.target then
			return `have {have}`
		end
	end
	for i, input in r.inputs do
		local why = short(b, i, input, count)
		if why then
			return why
		end
	end
	return nil
end

-- What the order a bill is being made by waits for, or nil: a piece that
-- matches the first one in, or, before any is in, one thing with enough.
local function waiting(o, b, count): string?
	local r = recipes[b.recipe]
	for i, n in o.needs do
		-- Short, but someone is bringing it.
		if n.have >= n.count or n.coming then
			continue
		end
		if n.match then
			if count({ thing = n.match }) == 0 then
				return `{n.have} of {n.count} {things[n.match].label}`
			end
		elseif r and r.inputs[i] then
			local why = short(b, i, r.inputs[i], count)
			if why then
				return why
			end
		end
	end
	return nil
end

-- A recipe's inputs as the order's needs, each with the bill's filter.
local function needs_of(b, r): { any }
	local out = {}
	for i, input in r.inputs do
		local f = b.filters and b.filters[i]
		table.insert(out, { thing = input.thing, tag = input.tag, count = input.count, filter = f, alike = not input.mix })
	end
	return out
end

-- The first line of an engine error, without Luau's wrapping.
local function reason(err: any): string
	local line = string.match(tostring(err), "^[^\n]*") or ""
	return (string.gsub(line, "^runtime error: ", ""))
end

-- Give an idle station the first bill that can run, and say what stops
-- the ones above it. True if anything changed.
local function run(st, thing: string, count): boolean
	-- Busy: ours, or another mod's, which we leave alone. Ours says what
	-- it waits for.
	local o = rim.order(st.site)
	if o ~= nil then
		local i = st.making and o.owner == "crafting" and bill_index(st, st.making.bill)
		local b = i and st.bills[i]
		if b then
			local why = waiting(o, b, count)
			if b.why ~= why then
				b.why = why
				return true
			end
		end
		return false
	end
	if st.making then
		-- Our order is gone and its end not yet heard. Hooks run before
		-- events in a tick, so it may have finished this very tick, and
		-- order_done will say so. If it's still unheard at the next hook,
		-- a load dropped it (its inputs' mod is gone).
		if not st.stale then
			st.stale = true
			return true
		end
		st.making, st.stale = nil, nil
	end
	local changed = false
	for _, b in st.bills do
		if b.suspended then
			continue
		end
		-- Below the one chosen, a bill is only waiting its turn.
		local why = if st.making then nil else blocked(b, count)
		if why == nil and not st.making then
			local r = recipes[b.recipe]
			local ok, err = pcall(rim.post_order, st.site, {
				label = r.label,
				needs = needs_of(b, r),
				work = math.max(1, math.floor(r.work / (speeds[thing] or 1) + 0.5)),
				work_type = r.work_type,
				requires = r.requires,
			})
			if ok then
				st.making = { bill = b.id, recipe = r.id }
				changed = true
			else
				-- A recipe the engine refuses (a tool tag no tool has) would
				-- fail every time: pause it and say why.
				b.suspended = true
				why = reason(err)
			end
		end
		changed = changed or b.why ~= why
		b.why = why
	end
	return changed
end

local catalog_written = false

rim.every(60, function()
	-- The catalog, for the panel: which recipes each station makes. Written
	-- after every load, so a mod added to a saved game shows its recipes.
	if not catalog_written then
		catalog_written = true
		local stations, labels = {}, {}
		for _, t in rim.thing_defs do
			local list = recipes_at(t.id)
			if #list > 0 then
				stations[t.id] = list
			end
		end
		for _, id in recipe_order do
			local r = recipes[id]
			local ins, choices = {}, {}
			for i, input in r.inputs do
				table.insert(ins, `{input.count} {label_of(input)}`)
				-- A tag input the player can narrow: its things, for the panel.
				local ids = candidates(input)
				if input.tag and #ids > 1 then
					local list = {}
					for _, t in ids do
						table.insert(list, { id = t, label = things[t].label })
					end
					table.insert(choices, { input = i, tag = input.tag, things = list })
				end
			end
			labels[id] =
				{ label = r.label, inputs = table.concat(ins, ", "), output = r.outputs[1].thing, choices = choices }
		end
		rim.set_data(CATALOG, { stations = stations, recipes = labels })
	end

	local state = load()
	if #state == 0 then
		return
	end
	local count = counter()
	local changed = false
	for i = #state, 1, -1 do
		local st = state[i]
		local t = rim.thing(st.site)
		if t == nil or (#st.bills == 0 and not st.making) then
			table.remove(state, i)
			changed = true
		elseif run(st, t.thing, count) then
			changed = true
		end
	end
	if changed then
		rim.set_data(KEY, state)
	end
end)

-- ─── The order's end ────────────────────────────────────────────────────────

-- The outputs are the recipe's the order was posted for, even if its bill
-- was removed as it finished.
rim.on("order_done", function(e)
	if e.owner ~= "crafting" then
		return
	end
	local state = load()
	local st = station_of(state, e.site)
	local m = st and st.making
	if not m then
		return
	end
	st.making, st.stale = nil, nil
	local r = recipes[m.recipe]
	if r and e.x then
		local stuff = if r.stuff then e.stuff else nil
		for _, out in r.outputs do
			rim.spawn_item(out.thing, e.x, e.y, out.count, stuff)
			rim.emit("crafting:made", { site = e.site, recipe = r.id, thing = out.thing, count = out.count, made_of = stuff })
		end
	end
	local i = bill_index(st, m.bill)
	local b = i and st.bills[i]
	if b then
		b.done += 1
		if b.mode == "count" then
			b.target -= 1
			if b.target <= 0 then
				table.remove(st.bills, i)
			end
		end
	end
	rim.set_data(KEY, state)
end)

-- The station is gone, and with it its bills.
rim.on("order_lost", function(e)
	if e.owner ~= "crafting" then
		return
	end
	local state = load()
	for i, st in state do
		if st.site == e.site then
			table.remove(state, i)
			rim.set_data(KEY, state)
			return
		end
	end
end)

-- ─── Changing bills ─────────────────────────────────────────────────────────

-- Take down the order a bill is being made by, putting its inputs back.
-- If the order has already finished this tick, order_done still makes it.
local function stop(st, id: number)
	if not st.making or st.making.bill ~= id then
		return
	end
	local o = rim.order(st.site)
	if o and o.owner == "crafting" then
		rim.cancel_order(st.site)
		st.making, st.stale = nil, nil
	end
end

local function whole(n: any, lo: number, hi: number): number?
	if type(n) ~= "number" then
		return nil
	end
	return math.clamp(math.floor(n), lo, hi)
end

-- Let a bill's input `i` take `thing`, or stop it. The last thing allowed
-- stays: a bill that can take nothing would only wait. True if it changed.
local function narrow(b, i: any, thing: any, on: any): boolean
	local r = recipes[b.recipe]
	local input = r and type(i) == "number" and r.inputs[i]
	if not input or not input.tag or type(thing) ~= "string" or type(on) ~= "boolean" then
		return false
	end
	local all = candidates(input)
	if not table.find(all, thing) then
		return false
	end
	local ids, set = allowed(b, i, input)
	local now = set == nil or set[thing] == true
	if now == on or (not on and #ids <= 1) then
		return false
	end
	local next = {}
	for _, id in all do
		if (id == thing and on) or (id ~= thing and table.find(ids, id)) then
			table.insert(next, id)
		end
	end
	b.filters = b.filters or {}
	-- Everything allowed is no filter at all, so a thing a mod adds later
	-- is taken too.
	b.filters[i] = if #next == #all then nil else { allows = next }
	return true
end

local bills = {}

-- Add a bill for `recipe` at the bottom of a station's list: make one.
-- Returns its id, or nil if the thing can't make that recipe.
function bills.add(site: number, recipe: string): number?
	local t = type(site) == "number" and rim.thing(site)
	local r = recipes[recipe]
	if not t or t.blueprint or things[t.thing].item or not r or not things[t.thing].tags[r.station] then
		return nil
	end
	local state = load()
	local st = station_of(state, site)
	if not st then
		st = { site = site, bills = {} }
		table.insert(state, st)
	end
	local id = (rim.get_data(NEXT) or 0) + 1
	rim.set_data(NEXT, id)
	table.insert(st.bills, { id = id, recipe = recipe, mode = "count", target = 1, done = 0, suspended = false })
	rim.set_data(KEY, state)
	return id
end

-- Change a bill: { mode?, target?, suspended? }. Pausing the bill being
-- made takes its order down.
function bills.set(site: number, bill: number, changes: { [string]: any })
	local state = load()
	local st = station_of(state, site)
	local i = st and bill_index(st, bill)
	if not i then
		return
	end
	local b = st.bills[i]
	if MODES[changes.mode] then
		b.mode = changes.mode
	end
	b.target = whole(changes.target, 1, MAX_TARGET) or b.target
	local ing = changes.ingredient
	if type(ing) == "table" and narrow(b, ing.input, ing.thing, ing.on) then
		-- The order up was asked for what was allowed before: post it again.
		b.why = nil
		stop(st, b.id)
	end
	if type(changes.suspended) == "boolean" then
		b.suspended = changes.suspended
		b.why = nil
		if changes.suspended then
			stop(st, b.id)
		end
	end
	rim.set_data(KEY, state)
end

-- Move a bill up (-1) or down (+1) the list.
function bills.move(site: number, bill: number, by: number)
	local state = load()
	local st = station_of(state, site)
	local i = st and bill_index(st, bill)
	local step = whole(by, -1, 1)
	local j = i and step and i + step
	if not j or j < 1 or j > #st.bills or j == i then
		return
	end
	st.bills[i], st.bills[j] = st.bills[j], st.bills[i]
	rim.set_data(KEY, state)
end

-- Remove a bill, taking down its order if it's being made.
function bills.remove(site: number, bill: number)
	local state = load()
	local st = station_of(state, site)
	local i = st and bill_index(st, bill)
	if not i then
		return
	end
	stop(st, bill)
	table.remove(st.bills, i)
	rim.set_data(KEY, state)
end

-- The recipes a thing can make: ids, in load order.
function bills.recipes_at(thing: string): { string }
	return recipes_at(thing)
end

-- A station's bills, or an empty list.
function bills.of(site: number): { any }
	local st = station_of(load(), site)
	return if st then st.bills else {}
end

rim.on("crafting:add_bill", function(e)
	bills.add(e.site, e.recipe)
end)
rim.on("crafting:set_bill", function(e)
	bills.set(e.site, e.bill, e)
end)
rim.on("crafting:move_bill", function(e)
	bills.move(e.site, e.bill, e.by)
end)
rim.on("crafting:remove_bill", function(e)
	bills.remove(e.site, e.bill)
end)

return bills
