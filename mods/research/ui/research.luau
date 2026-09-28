-- The research screen: every project, what it unlocks and how far along it
-- is, and a button to study it next. A desk's inspector says what it
-- studies. Every change is an act.send to scripts/research.luau.

local kit = require("@core/ui/kit")
local screens = require("@core/ui/screens")
local row, col, label = kit.row, kit.col, kit.label

local function state(view)
	local s = view.data("research:state") or {}
	return s.current, s.progress or {}, s.done or {}
end

local function available(p, done): boolean
	if done[p.id] then
		return false
	end
	for _, r in p.requires do
		if not done[r] then
			return false
		end
	end
	return true
end

local function project_row(view, p, labels, current, progress, done)
	local id = "research:project." .. p.id
	local studying = current == p.id
	local open = available(p, done)
	local status = if done[p.id]
		then "done"
		elseif studying then "studying"
		elseif open then "can be studied"
		else string.format("needs %s", table.concat(labels, ", "))
	local unlocks = if #p.unlocks > 0
		then string.format("Unlocks %s", table.concat(p.unlocks, ", "))
		else nil
	return col({ id = id, gap = "xs", pady = "s" }, {
		row({ gap = "m", align = "center" }, {
			label(p.label, { weight = "strong" }),
			label(status, { size = "small", color = if done[p.id] then "good" elseif studying then "accent" else "muted" }),
			{ kind = "spacer" },
			open and kit.button({
				id = id .. ".study",
				label = if studying then "Stop" else "Study",
				active = studying,
				on_click = function()
					act.send("research:choose", { project = if studying then "" else p.id })
				end,
			}),
		}),
		p.note ~= "" and label(p.note, { color = "muted", wrap = true }),
		unlocks and label(unlocks, { size = "small", color = "muted", wrap = true }),
		not done[p.id] and kit.bar({ id = id .. ".progress", value = (progress[p.id] or 0) / p.work }),
	})
end

ui.window("research:screen", { title = "Research", w = 520, h = 520, sheet = true }, function(view)
	local catalog = view.data("research:catalog")
	if not catalog then
		return col({ pad = "m" }, { label("…", { color = "muted" }) })
	end
	local current, progress, done = state(view)
	local names = {}
	for _, p in catalog do
		names[p.id] = p.label
	end
	local rows = {}
	local desks = (view.data("research:state") or {}).desks or {}
	if #desks == 0 then
		table.insert(rows, label("Build a research desk: colonists study there.", { color = "bad", wrap = true }))
	end
	for _, p in catalog do
		local needs = {}
		for _, r in p.requires do
			if not done[r] then
				table.insert(needs, names[r] or r)
			end
		end
		table.insert(rows, project_row(view, p, needs, current, progress, done))
	end
	return col({ gap = "s", pad = "m" }, rows)
end)
screens.add({ id = "research:screen", label = "Research", order = 60 })

-- A desk: what it studies, and a way in to the screen.
ui.extend("core:inspector.thing", function(view)
	local site = view.selected()
	local th = site and view.thing(site)
	if not th or th.blueprint or th.def ~= "research:desk" then
		return nil
	end
	local catalog = view.data("research:catalog") or {}
	local current, progress = state(view)
	local what = "Nothing to study: pick a project."
	for _, p in catalog do
		if p.id == current then
			what = string.format("Studying %s: %d of %d", p.label, progress[p.id] or 0, p.work)
		end
	end
	return col({ id = "research:desk", gap = "xs" }, {
		label(what, { size = "small", color = "muted", wrap = true }),
		kit.button({
			id = "research:desk.open",
			label = "Research…",
			on_click = function()
				ui.toggle("research:screen")
			end,
		}),
	})
end)
