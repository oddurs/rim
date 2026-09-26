-- Context menus: one menu for anything the player can point at. A right-
-- click on a node that declares a subject (`menu = { kind, id }`), or on
-- the map, opens the rows every provider registered for that kind gives,
-- in four groups: do (act now), work (mark for work), manage (look and
-- set), damaging (take away; always last, never the default).
--
--     local menus = require("@core/ui/menus")
--     menus.add("thing", {
--         id = "my_mod:check_fuel",
--         label = "Check fuel",
--         group = "manage",
--         applies = function(ctx) return ctx.subject.def == "my_mod:kiln" end,
--         disabled = function(ctx) return #ctx.actors == 0 and "no one selected" end,
--         run = function(ctx) ui.open("my_mod:kiln") end,
--     })
--
-- A provider may give several rows at once with `rows = fn(ctx) -> { row }`
-- instead of `label` and `run` (the map's orders come that way). A kind is
-- described once with `menus.kind(name, { resolve, caption, actor })`: how
-- to find its subject from an id, and what the caption says.
--
-- Providers run once, when the menu opens; the rows are kept while it's
-- open, and a closed menu builds nothing. Ids: the menu `core:menu`, its
-- rows `core:menu.row.<n>`.

local kit = require("@core/ui/kit")

local menus = {}

local RANK = { ["do"] = 1, work = 2, manage = 3, damaging = 4 }

local kinds: { [string]: any } = {}
local providers: { [string]: { any } } = {}

-- The open menu, or nil: its subject, the context providers saw, where it
-- opened, and its rows in display order.
local open: any = nil

-- Describe a kind of subject: resolve(id) -> the subject's table or nil
-- (nil closes an open menu: the thing went away), caption(ctx) -> the
-- menu's caption, actor(ctx) -> who acts, if anyone.
function menus.kind(name: string, def)
	kinds[name] = def
end

-- Add rows to every menu about a kind. A second add with the same id
-- replaces the first, so a reloaded script doesn't double up.
function menus.add(kind: string, def)
	assert(type(def) == "table" and type(def.id) == "string", "menus.add: needs an id")
	assert(def.rows or (def.label and def.run), `menus.add('{def.id}'): needs label and run, or rows`)
	assert(RANK[def.group or "do"], `menus.add('{def.id}'): group is do, work, manage or damaging`)
	local list = providers[kind] or {}
	providers[kind] = list
	for i, p in list do
		if p.id == def.id then
			list[i] = def
			return
		end
	end
	table.insert(list, def)
end

local function text(v, ctx): string
	return if type(v) == "function" then v(ctx) else v
end

-- One provider's rows for this context.
local function rows_of(p, ctx)
	if p.applies and not p.applies(ctx) then
		return {}
	end
	if p.rows then
		return p.rows(ctx) or {}
	end
	local reason = p.disabled and p.disabled(ctx) or nil
	return {
		{
			label = text(p.label, ctx),
			group = p.group or "do",
			primary = p.primary and p.primary(ctx) or false,
			trailing = p.key and string.upper(p.key) or nil,
			disabled = if reason then reason else nil,
			run = p.run,
		},
	}
end

function menus.is_open(): boolean
	return open ~= nil
end

function menus.close()
	open = nil
end

-- Open the menu for a subject at a point (logical pixels). Nothing opens
-- when no provider has a row for it.
function menus.open(subject, x: number, y: number)
	local kind = kinds[subject.kind]
	local found = if kind and kind.resolve then kind.resolve(subject.id) else nil
	if kind and kind.resolve and found == nil then
		open = nil
		return
	end
	local ctx = { kind = subject.kind, id = subject.id, subject = found, actors = view.selection() }
	local rows = {}
	local list: { any } = providers[subject.kind] or {}
	for _, p in list do
		for _, r in rows_of(p, ctx) do
			r.group = r.group or p.group or "do"
			r.run = r.run or p.run
			table.insert(rows, r)
		end
	end
	if #rows == 0 then
		open = nil
		return
	end
	-- In group order, keeping each group's own order.
	for i, r in rows do
		r.order = i
	end
	table.sort(rows, function(a, b)
		if RANK[a.group] ~= RANK[b.group] then
			return RANK[a.group] < RANK[b.group]
		end
		return a.order < b.order
	end)
	-- One primary at most, never a damaging or disabled row: the first
	-- flagged, else the first enabled row that acts now.
	local primary = nil
	for i, r in rows do
		if r.primary and not r.disabled and r.group ~= "damaging" then
			primary = primary or i
		end
		r.primary = false
	end
	if not primary then
		for i, r in rows do
			if r.group == "do" and not r.disabled then
				primary = i
				break
			end
		end
	end
	if primary then
		rows[primary].primary = true
	end
	-- Number keys go to enabled rows, in order.
	local n = 0
	for _, r in rows do
		if not r.disabled then
			n += 1
			r.number = n
			r.trailing = r.trailing or (if n <= 9 then tostring(n) else nil)
		end
	end
	open = {
		subject = subject,
		ctx = ctx,
		rows = rows,
		x = x,
		y = y,
		caption = if kind and kind.caption then kind.caption(ctx) else nil,
		actor = if kind and kind.actor then kind.actor(ctx) else nil,
	}
	local first = primary
	if not first then
		for i, r in rows do
			if not r.disabled then
				first = i
				break
			end
		end
	end
	ui.set_state("core:menu.hi", first or 1)
end

local function pick(i: number)
	local m = open
	if not m then
		return
	end
	local r = m.rows[i]
	if not r or r.disabled then
		return
	end
	open = nil
	r.run(m.ctx, r)
end

-- Move the highlight by d, skipping rows that can't run.
local function step(d: number)
	if not open then
		return
	end
	local rows = open.rows
	local i = ui.state("core:menu.hi", 1)
	for _ = 1, #rows do
		i = ((i - 1 + d) % #rows) + 1
		if not rows[i].disabled then
			break
		end
	end
	ui.set_state("core:menu.hi", i)
end

local function on_key(key: string)
	if not open then
		return
	end
	if key == "escape" then
		open = nil
	elseif key == "down" then
		step(1)
	elseif key == "up" then
		step(-1)
	elseif key == "enter" then
		pick(ui.state("core:menu.hi", 1))
	elseif string.match(key, "^%d$") then
		local n = tonumber(key)
		for i, r in open.rows do
			if r.number == n then
				pick(i)
				return
			end
		end
	elseif #key == 1 then
		-- A first letter: the next enabled row that starts with it.
		local rows = open.rows
		local from = ui.state("core:menu.hi", 1)
		for k = 1, #rows do
			local i = ((from - 1 + k) % #rows) + 1
			local r = rows[i]
			if not r.disabled and string.lower(string.sub(r.label, 1, 1)) == key then
				ui.set_state("core:menu.hi", i)
				return
			end
		end
	end
end

ui.on_context(function(subject, x, y)
	menus.open(subject, x, y)
end)

ui.define("core:menu", function(view)
	local m = open
	if not m then
		return nil
	end
	-- The subject went away while the menu was up: nothing to act on.
	local kind = kinds[m.subject.kind]
	if kind and kind.resolve and kind.resolve(m.subject.id) == nil then
		open = nil
		return nil
	end
	local groups = { {}, {}, {}, {} }
	for i, r in m.rows do
		table.insert(groups[RANK[r.group]], {
			label = r.label,
			tone = if r.primary then "primary" elseif r.group == "damaging" then "damaging" else nil,
			trailing = r.trailing,
			disabled = r.disabled,
			on_click = function()
				pick(i)
			end,
			on_hover = function()
				if not r.disabled then
					ui.set_state("core:menu.hi", i)
				end
			end,
		})
	end
	return kit.menu({
		id = "core:menu",
		subject = m.caption,
		actor = m.actor,
		groups = groups,
		hi = ui.state("core:menu.hi", 1),
		at = { x = m.x, y = m.y },
		on_key = on_key,
		on_outside = function()
			open = nil
		end,
	})
end)
ui.mount("popup", "core:menu")

return menus
