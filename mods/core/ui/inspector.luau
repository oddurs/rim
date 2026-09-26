-- The inspector: the selected pawn or thing, bottom left. What it shows is
-- two registries this file returns, and core's own tabs and actions go
-- through them as a mod's would:
--
--     local inspector = require("@core/ui/inspector")
--     inspector.tab({
--         id = "mood", label = "Mood",
--         applies = function(sel) return sel.kind == "pawn" and sel.player end,
--         build = function(view, sel) return kit.label("content") end,
--     })
--     inspector.action({
--         id = "my_mod:rally", label = "Rally", key = "g",
--         applies = function(sel) return sel.kind == "pawn" and sel.player end,
--         run = function(sel) act.send("my_mod:rally", sel.id) end,
--     })
--
-- `sel` is the pawn table (view.pawn) or the thing table (view.thing), with
-- `kind` set to "pawn" or "thing". A label may be a function of it; an
-- action's `active(sel)` lights its button. An action's key is a binding
-- with the action's id, so the palette lists it; it runs on the selection
-- when it applies.
--
-- With several colonists selected the panel sums them up, and shows the
-- actions marked `group = true` that apply to every one of them. Such an
-- action's label, active and run get a second argument, `all`, the list
-- of selected pawns, and run is called once for each of them.
--
-- Ids: tabs `core:inspector.tabs.<id>` (only shown when more than one
-- applies), actions `core:inspector.action.<id>`. Extension points:
-- `core:inspector.sections` (under a pawn's overview) and
-- `core:inspector.thing` (under a thing).

local kit = require("@core/ui/kit")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

local inspector = {}

local tabs: { any } = {}
local actions: { any } = {}

local function put(list, def)
	for i, d in list do
		if d.id == def.id then
			list[i] = def
			return
		end
	end
	table.insert(list, def)
	table.sort(list, function(a, b)
		return (a.order or 100) < (b.order or 100)
	end)
end

local function text_of(v, sel, all): string
	return if type(v) == "function" then v(sel, all or { sel }) else v
end

-- The selection as the registries see it, or nil.
local function selection(view)
	local id = view.selected()
	if not id then
		return nil
	end
	local p = view.pawn(id)
	if p then
		p.kind = "pawn"
		return p
	end
	local th = view.thing(id)
	if th then
		th.kind = "thing"
		th.id = id
		return th
	end
	return nil
end

function inspector.tab(def)
	assert(type(def) == "table" and type(def.id) == "string", "inspector.tab: needs an id")
	assert(type(def.build) == "function", `inspector.tab('{def.id}'): needs build = fn(view, sel)`)
	put(tabs, def)
end

-- The selected pawns when several are, else nil.
local function group(view)
	local ids = view.selection()
	if #ids < 2 then
		return nil
	end
	local all = {}
	for _, id in ids do
		local p = view.pawn(id)
		if p then
			p.kind = "pawn"
			table.insert(all, p)
		end
	end
	return all
end

local function applies_to_all(def, all): boolean
	if not def.group then
		return false
	end
	for _, p in all do
		if def.applies and not def.applies(p) then
			return false
		end
	end
	return true
end

function inspector.action(def)
	assert(type(def) == "table" and type(def.id) == "string", "inspector.action: needs an id")
	assert(type(def.run) == "function", `inspector.action('{def.id}'): needs run = fn(sel)`)
	put(actions, def)
	if def.key or def.bind_label then
		ui.bind(def.id, { key = def.key, label = def.bind_label or text_of(def.label, {}) }, function()
			local all = group(view)
			if all then
				if applies_to_all(def, all) then
					for _, p in all do
						def.run(p, all)
					end
				end
				return
			end
			local sel = selection(view)
			if sel and (not def.applies or def.applies(sel)) then
				def.run(sel, { sel })
			end
		end)
	end
end

local function applying(list, sel)
	local out = {}
	for _, d in list do
		if not d.applies or d.applies(sel) then
			table.insert(out, d)
		end
	end
	return out
end

-- ─── Core's tabs ────────────────────────────────────────────────────────────

local function colonist(sel)
	return sel.kind == "pawn" and sel.player
end

inspector.tab({
	id = "overview",
	order = 10,
	label = t("core:inspector.tab.overview", "Overview"),
	applies = function(sel)
		return sel.kind == "pawn"
	end,
	build = function(view, p)
		local bars = { kit.bar({ label = t("core:inspector.health", "health"), value = p.health, color = "good" }) }
		for _, n in p.needs do
			table.insert(bars, kit.bar({ id = "core:inspector.need." .. n.id, label = n.label, value = n.value, color = if n.low then "bad" else n.color }))
		end
		local gear = {}
		if p.hand then
			table.insert(gear, string.format(t("core:inspector.hand", "holding %s"), p.hand))
		end
		if p.carrying then
			table.insert(gear, string.format(t("core:inspector.carrying", "carrying %s"), p.carrying))
		end
		return col({ gap = "s" }, {
			col({ id = "core:inspector.bars", gap = "xs" }, bars),
			#gear > 0 and label(table.concat(gear, " · "), { id = "core:inspector.gear", size = "small", color = "muted" }),
			-- Extension point: mods add sections (mood, traits...).
			col({ id = "core:inspector.sections", gap = "s" }),
		})
	end,
})

inspector.tab({
	id = "skills",
	order = 20,
	label = t("core:inspector.tab.skills", "Skills"),
	applies = colonist,
	build = function(view, p)
		local rows = {}
		for _, s in p.skills do
			table.insert(rows, kit.bar({
				id = "core:inspector.skill." .. s.id,
				label = string.format(t("core:inspector.skill", "%s %d"), s.label, s.level),
				value = s.progress,
				color = "accent",
				label_width = 110,
				tooltip = if s.trains ~= "" then string.format(t("core:inspector.trains", "Trained by %s"), s.trains) else nil,
			}))
		end
		return col({ id = "core:inspector.skills", gap = "xs" }, rows)
	end,
})

-- Each work type: its level (the rules' reason as the tooltip) and why the
-- colonist takes it or passes it over: the why panel (DESIGN.md §4d).
inspector.tab({
	id = "work",
	order = 30,
	label = t("core:inspector.tab.work", "Work"),
	applies = colonist,
	build = function(view, p)
		local eff = view.effective(p.id) or {}
		local why = {}
		for _, x in view.explain_work(p.id) or {} do
			why[x.work] = x
		end
		local rows = {}
		for _, w in view.work_types() do
			local e = eff[w.id]
			if e then
				local x = why[w.id]
				table.insert(rows, row({ id = "core:inspector.work." .. w.id, gap = "s", tooltip = e.why }, {
					row({ w = 70 }, { label(w.label, { size = "small", weight = if x and x.picked then "strong" else nil }) }),
					row({ w = 16 }, {
						label(if e.value == 0 then "–" else tostring(e.value), { size = "small", color = if e.value == 0 then "faint" else nil }),
					}),
					x and label(x.why, { id = "core:inspector.why." .. w.id, size = "small", color = if x.picked then "good" else "muted" }),
				}))
			end
		end
		return col({ id = "core:inspector.work", gap = "xxs" }, rows)
	end,
})

-- ─── Core's actions ─────────────────────────────────────────────────────────

-- Several at once all go the same way: drafted unless every one already is.
local function all_drafted(all): boolean
	for _, p in all do
		if not p.drafted then
			return false
		end
	end
	return true
end

inspector.action({
	id = "core:draft",
	order = 10,
	key = "r",
	group = true,
	bind_label = t("core:keys.draft", "Draft or undraft the selected colonists"),
	label = function(p, all)
		return if all_drafted(all) then t("core:inspector.undraft", "Undraft") else t("core:inspector.draft", "Draft")
	end,
	applies = colonist,
	active = function(p, all)
		return all_drafted(all)
	end,
	run = function(p, all)
		act.draft(p.id, not all_drafted(all))
	end,
})

inspector.action({
	id = "core:center",
	order = 20,
	key = "c",
	bind_label = t("core:keys.center", "Centre on the selection"),
	label = t("core:inspector.center", "Centre"),
	run = function(sel)
		act.focus(sel.id)
	end,
})

-- ─── The panel ──────────────────────────────────────────────────────────────

local function action_row(sel)
	local items = {}
	for _, a in applying(actions, sel) do
		table.insert(
			items,
			kit.button({
				id = "core:inspector.action." .. a.id,
				label = text_of(a.label, sel),
				hint = a.key and string.upper(a.key),
				active = a.active and a.active(sel, { sel }) or false,
				on_click = function()
					a.run(sel, { sel })
				end,
			})
		)
	end
	return #items > 0 and row({ id = "core:inspector.actions", gap = "xs" }, items)
end

local function tab_body(view, sel)
	local shown = applying(tabs, sel)
	if #shown == 0 then
		return nil, nil
	end
	local chosen = shown[1]
	local want = ui.state("core:inspector.tab", "overview")
	for _, tab in shown do
		if tab.id == want then
			chosen = tab
		end
	end
	local bar = nil
	if #shown > 1 then
		local list = {}
		for _, tab in shown do
			table.insert(list, { key = tab.id, label = text_of(tab.label, sel) })
		end
		bar = kit.tabs({
			id = "core:inspector.tabs",
			tabs = list,
			selected = chosen.id,
			on_select = function(key)
				ui.set_state("core:inspector.tab", key)
			end,
		})
	end
	return bar, chosen.build(view, sel)
end

local function pawn_panel(view, p)
	local title = if p.founder then string.format(t("core:inspector.founder", "%s (founder)"), p.name) else p.name
	local doing = if p.drafted then t("core:inspector.drafted", "drafted") else p.job
	local hint = if p.player
		then (if p.drafted then t("core:inspector.hint_drafted", "right-click to move or attack") else t("core:inspector.hint", "right-click to order"))
		else nil
	local bar, body = tab_body(view, p)
	return kit.panel({ w = 300, gap = "m" }, {
		col({ gap = "xxs" }, {
			label(title, { size = "heading", weight = "strong" }),
			label(string.format(t("core:inspector.sub", "%s · %s · %s"), p.label, p.faction, doing), { size = "small", color = "muted" }),
		}),
		action_row(p),
		bar,
		body,
		hint and label(hint, { size = "small", color = "faint" }),
	})
end

local function thing_panel(view, th)
	local facts = {}
	if th.count > 1 then
		table.insert(facts, string.format(t("core:inspector.count", "×%d"), th.count))
	end
	if th.made_of then
		table.insert(facts, th.made_of)
	end
	if th.blueprint then
		table.insert(facts, t("core:inspector.blueprint", "planned"))
	end
	if th.designated then
		table.insert(facts, string.format(t("core:inspector.designated", "marked: %s"), th.designated))
	end
	local bar, body = tab_body(view, th)
	return kit.panel({ id = "core:inspector.panel", w = 300, gap = "m" }, {
		col({ gap = "xxs" }, {
			label(th.label, { size = "heading", weight = "strong" }),
			#facts > 0 and label(table.concat(facts, " · "), { size = "small", color = "muted" }),
		}),
		action_row(th),
		th.max_hp > 0 and kit.bar({ label = t("core:inspector.hp", "hp"), value = th.hp / th.max_hp, color = "good" }),
		th.why and label(th.why, { id = "core:inspector.why", size = "small", color = "bad", wrap = true }),
		bar,
		body,
		-- Extension point: mods add sections for things (a station's bills,
		-- a tool's wear). They read the thing with view.thing(view.selected()).
		col({ id = "core:inspector.thing", gap = "s" }),
	})
end

-- Several colonists: who they are, how they are, and what they can all do.
local function group_panel(view, all)
	local buttons = {}
	for _, a in actions do
		if applies_to_all(a, all) then
			table.insert(
				buttons,
				kit.button({
					id = "core:inspector.action." .. a.id,
					label = text_of(a.label, all[1], all),
					hint = a.key and string.upper(a.key),
					active = a.active and a.active(all[1], all) or false,
					on_click = function()
						for _, p in all do
							a.run(p, all)
						end
					end,
				})
			)
		end
	end
	local members = {}
	for _, p in all do
		table.insert(
			members,
			row({
				id = "core:inspector.member." .. p.name,
				gap = "s",
				align = "center",
				hover = { bg = "surface_hover" },
				on_click = function()
					act.select(p.id)
				end,
			}, {
				row({ w = 90 }, { label(p.name, { size = "small", color = if p.drafted then "player" else nil }) }),
				kit.bar({ value = p.health, color = if p.health > 0.5 then "good" else "bad", width = 170 }),
			})
		)
	end
	return kit.panel({ id = "core:inspector.group", w = 300, gap = "m" }, {
		label(string.format(t("core:inspector.group", "%d colonists"), #all), { size = "heading", weight = "strong" }),
		#buttons > 0 and row({ id = "core:inspector.actions", gap = "xs" }, buttons),
		col({ gap = "xxs" }, members),
		label(t("core:inspector.group_hint", "right-click to order them all · shift-click to add or remove"), { size = "small", color = "faint", wrap = true }),
	})
end

ui.define("core:inspector", function(view)
	local all = group(view)
	if all then
		return group_panel(view, all)
	end
	local sel = selection(view)
	if not sel then
		return nil
	elseif sel.kind == "pawn" then
		return pawn_panel(view, sel)
	end
	return thing_panel(view, sel)
end)
ui.mount("left", "core:inspector", { align = "end" })

return inspector
