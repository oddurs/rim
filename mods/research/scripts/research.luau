-- Research: one project at a time, studied at every desk as work orders,
-- unlocking what its modifiers hold back (DESIGN.md §4e). The engine knows
-- no research: a project's unlocks are [[modifier]]s on the stat pipeline,
-- in the group named by the project's id, and finishing it switches them
-- off with rim.set_modifiers.
--
-- State is script data (`research:state`), so it's saved and hashed:
--   current   the project being studied, or nil
--   progress  { [project] = work done }
--   done      { [project] = true }
--   desks     { site, ... }  every desk that studies
-- The screen and the desk's inspector change it with events:
--   research:choose { project }   study this next ("" for nothing)
--   research:desk   { site }      study at this desk (a built one joins itself)
-- A catalog of the projects, for the screen, is `research:catalog`.

local KEY = "research:state"
local CATALOG = "research:catalog"
-- Ticks a desk works before its study counts toward the project.
local SESSION = 200

local function owner_of(id: string): string
	return string.match(id, "^([^:]+):") or "research"
end

local function short(id: string): string
	return string.match(id, ":(.+)$") or id
end

local function qualify(id: string, owner: string): string
	if string.find(id, ":", 1, true) then
		return id
	end
	return `{owner}:{id}`
end

-- What the defs say, which never changes during a game: locals are safe.
local labels = {} -- thing id -> label
local desks = {} -- thing id -> true, for things tagged research:desk
for _, t in rim.thing_defs do
	labels[t.id] = t.label
	if table.find(t.tags, "research:desk") then
		desks[t.id] = true
	end
end

local projects = {} -- id -> { id, label, note, work, requires, unlocks }
local order = {} -- load order, so the catalog is deterministic
for _, def in rim.defs("project") do
	local owner = owner_of(def.id)
	assert(def.work > 0, `project '{def.id}': work must be more than 0`)
	local requires = {}
	for _, r in def.requires do
		table.insert(requires, qualify(r, owner))
	end
	-- Its unlocks: the buildables its modifier group holds back. The group
	-- is "research:<name>" from any mod, or this plugin's own "<name>":
	-- rim.set_modifiers switches exactly those for us.
	local unlocks = {}
	local group = short(def.id)
	for _, m in rim.modifier_defs do
		local ours = owner_of(m.id) == "research" and m.group == group
		if (ours or m.group == "research:" .. group) and m.stat == "buildable" and m.value < 0 then
			table.insert(unlocks, m.thing)
		end
	end
	projects[def.id] = {
		id = def.id,
		owner = owner,
		label = def.label,
		note = def.note,
		work = def.work,
		requires = requires,
		unlocks = unlocks,
	}
	table.insert(order, def.id)
end
for _, id in order do
	for _, r in projects[id].requires do
		assert(projects[r], `project '{id}': requires '{r}', which no mod defines`)
	end
end

local function load()
	local s = rim.get_data(KEY) or {}
	s.progress = s.progress or {}
	s.done = s.done or {}
	s.desks = s.desks or {}
	return s
end

local function available(s, id: string): boolean
	local p = projects[id]
	if not p or s.done[id] then
		return false
	end
	for _, r in p.requires do
		if not s.done[r] then
			return false
		end
	end
	return true
end

local function has_desk(s, site: number): boolean
	return table.find(s.desks, site) ~= nil
end

local function is_desk(site: number): boolean
	local t = rim.thing(site)
	return t ~= nil and desks[t.thing] == true and not t.blueprint
end

-- Done: its unlocks are let go, for good.
local function finish(s, id: string)
	local p = projects[id]
	s.done[id] = true
	s.progress[id] = nil
	if s.current == id then
		s.current = nil
	end
	rim.set_modifiers(short(id), false)
	local names = {}
	for _, t in p.unlocks do
		table.insert(names, labels[t] or t)
	end
	local what = if #names > 0 then `: {table.concat(names, ", ")} can be built` else ""
	rim.message(`Research done, {p.label}{what}.`, "good")
end

-- The catalog, for the screen: written after every load, so a mod added
-- to a saved game shows its projects.
local catalog_written = false

rim.every(60, function()
	if not catalog_written then
		catalog_written = true
		local list = {}
		for _, id in order do
			local p = projects[id]
			local unlocks = {}
			for _, t in p.unlocks do
				table.insert(unlocks, labels[t] or t)
			end
			table.insert(list, {
				id = id,
				label = p.label,
				note = p.note,
				work = p.work,
				requires = p.requires,
				unlocks = unlocks,
			})
		end
		rim.set_data(CATALOG, list)
	end
	local s = load()
	-- Desks torn down stop studying.
	local changed = false
	for i = #s.desks, 1, -1 do
		if not rim.thing(s.desks[i]) then
			table.remove(s.desks, i)
			changed = true
		end
	end
	if s.current and not available(s, s.current) then
		s.current = nil
		changed = true
	end
	if s.current then
		local p = projects[s.current]
		for _, site in s.desks do
			if is_desk(site) and not rim.order(site) then
				rim.post_order(site, {
					label = `Research {p.label}`,
					work = SESSION,
					work_type = "research:research",
					needs = {},
				})
			end
		end
	end
	if changed then
		rim.set_data(KEY, s)
	end
end)

-- A session at a desk: its work counts toward the project it was for.
rim.on("order_done", function(e)
	if e.owner ~= "research" then
		return
	end
	local s = load()
	local id = s.current
	if not id or not available(s, id) then
		return
	end
	s.progress[id] = (s.progress[id] or 0) + SESSION
	if s.progress[id] >= projects[id].work then
		finish(s, id)
	end
	rim.set_data(KEY, s)
end)

rim.on("building_complete", function(e)
	if not is_desk(e.id) then
		return
	end
	local s = load()
	if not has_desk(s, e.id) then
		table.insert(s.desks, e.id)
		rim.set_data(KEY, s)
	end
end)

rim.on("research:choose", function(e)
	local s = load()
	local id = e and e.project
	if id == "" or id == nil then
		s.current = nil
	elseif available(s, id) then
		s.current = id
	else
		return
	end
	-- A desk working on another project drops it: the next pass posts anew.
	for _, site in s.desks do
		local o = rim.order(site)
		if o and o.owner == "research" then
			rim.cancel_order(site)
		end
	end
	rim.set_data(KEY, s)
end)

rim.on("research:desk", function(e)
	local site = e and e.site
	if type(site) ~= "number" or not is_desk(site) then
		return
	end
	local s = load()
	if not has_desk(s, site) then
		table.insert(s.desks, site)
		rim.set_data(KEY, s)
	end
end)

return {}
