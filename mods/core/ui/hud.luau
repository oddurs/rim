-- Core's HUD, written on the UI system like any mod's would be.
--
-- Every part has a namespaced id, so other mods can extend, replace, wrap
-- or remove it (ui.extend("core:topbar.right", ...), ui.remove("core:hover")).

local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

local function pct(v: number): string
	return tostring(math.floor(v * 100 + 0.5)) .. "%"
end

-- ─── Top bar: clock, wealth, weather, colonists ─────────────────────────────

-- "Spring 9", and the year once there's more than one.
local function date(view): string
	local d = view.date()
	local s = string.upper(string.sub(d.season, 1, 1)) .. string.sub(d.season, 2) .. " " .. d.day
	return if d.year > 1 then string.format(t("core:clock.date_year", "%s, year %d"), s, d.year) else s
end

ui.define("core:clock", function(view)
	local speed = if view.paused() then t("core:clock.paused", "paused") else string.format(t("core:clock.speed", "%d×"), view.speed())
	return row({ gap = "m", align = "center" }, {
		label(string.format(t("core:clock.day", "Day %d"), view.day()), { weight = "strong" }),
		label(date(view), { id = "core:clock.date", color = "muted" }),
		label(view.clock(), { weight = "strong" }),
		label(speed, { color = if view.paused() then "bad" else "muted" }),
	})
end)

ui.define("core:status", function(view)
	local parts = { label(string.format(t("core:status.wealth", "Wealth %d"), math.floor(view.wealth())), { color = "muted" }) }
	for _, f in view.fields() do
		if f.hud then
			table.insert(parts, label(string.format(t("core:status.outside", "%.0f%s outside"), f.ambient, f.unit), { color = "muted" }))
		end
	end
	local overlay = view.overlay()
	if overlay then
		table.insert(parts, label(string.format(t("core:status.overlay", "overlay: %s"), overlay), { color = "accent" }))
	end
	return row({ gap = "l", align = "center" }, parts)
end)

-- The bar shows this many colonists and counts the rest: two hundred
-- buttons is neither readable nor within the frame budget.
local COLONIST_BAR_MAX = 24

ui.define("core:colonists", function(view)
	local items = {}
	for _, p in view.colonists(COLONIST_BAR_MAX) do
		local health = if p.health > 0.5 then "good" else "bad"
		table.insert(
			items,
			button({
				id = "core:colonists." .. p.name,
				label = p.name,
				active = p.selected,
				swatch = health,
				color = if p.drafted then "player" else nil,
				tooltip = string.format(t("core:colonists.tooltip", "%s · %s · health %s"), p.label, p.job, pct(p.health)),
				on_click = function()
					act.select(p.id)
					act.focus(p.id)
				end,
			})
		)
	end
	local total = view.count_pawns("player")
	if total > COLONIST_BAR_MAX then
		table.insert(items, label(string.format(t("core:colonists.more", "+%d"), total - COLONIST_BAR_MAX), { size = "small", color = "muted" }))
	end
	return row({ gap = "xs", align = "center" }, items)
end)

ui.define("core:topbar", function(view)
	return row({ padx = "l", pady = "s", gap = "xl", align = "center", bg = "surface", radius = 0 }, {
		ui.slot("core:clock"),
		ui.slot("core:status"),
		ui.slot("core:colonists"),
		{ kind = "spacer" },
		-- Extension point: mods add readouts here.
		row({ id = "core:topbar.right", gap = "l", align = "center" }),
		label(t("core:topbar.help", "Space pause · 1/2/3 speed · R draft · O overlay · F3 profiler · F12 devtools"), { size = "small", color = "faint" }),
	})
end)
ui.mount("top", "core:topbar")

-- ─── Toolbar: generated from designation and buildable defs ────────────────

ui.define("core:toolbar", function(view)
	local items = {}
	for _, t in view.tools() do
		table.insert(
			items,
			button({
				id = "core:toolbar." .. t.key,
				label = t.label,
				active = t.active,
				swatch = t.color,
				on_click = function()
					act.tool(t.key)
				end,
			})
		)
	end
	local buttons = row({ id = "core:toolbar.buttons", padx = "l", pady = "s", gap = "s", bg = "surface", radius = 0, align = "center" }, items)

	-- The material row: only when the active tool builds out of something.
	-- What you have none of is shown greyed, not hidden, so the row keeps
	-- its shape and a missing material is a fact rather than a mystery.
	local stuff = view.stuff()
	if #stuff == 0 then
		return buttons
	end
	local picks = {}
	local chosen = nil
	for _, m in stuff do
		if m.active then
			chosen = m
		end
		local none = m.have == 0
		table.insert(
			picks,
			button({
				id = "core:stuff." .. m.id,
				label = if none then string.format(t("core:stuff.none", "%s · none"), m.label) else string.format(t("core:stuff.have", "%s ×%d"), m.label, m.have),
				active = m.active,
				swatch = m.color,
				color = if none then "faint" else nil,
				tooltip = string.format(t("core:stuff.stats", "hp %d · %d work"), m.hp, m.work),
				on_click = function()
					act.stuff(m.id)
				end,
			})
		)
	end
	if chosen then
		table.insert(picks, label(string.format(t("core:stuff.stats", "hp %d · %d work"), chosen.hp, chosen.work), { size = "small", color = "muted" }))
	end
	local materials = row({ id = "core:stuff", padx = "l", pady = "s", gap = "s", bg = "surface", radius = 0, align = "center" }, picks)
	return col({}, { materials, buttons })
end)
ui.mount("bottom", "core:toolbar")

-- ─── Messages: the newest few, fading with age ─────────────────────────────

ui.define("core:messages", function(view)
	local items = {}
	for _, m in view.messages(8) do
		if m.age > 1 then
			break
		end
		table.insert(items, kit.toast({ text = m.text, kind = m.kind, age = m.age }))
	end
	if #items == 0 then
		return nil
	end
	return col({ gap = "xs" }, items)
end)
ui.mount("left", "core:messages", { align = "start" })

-- ─── Inspector: the selected pawn or thing ─────────────────────────────────

-- A colonist's panel has tabs: how they are, what they're good at, and
-- what they'll do next. Other pawns show only the first.
local TABS = {
	{ key = "overview", label = t("core:inspector.tab.overview", "Overview") },
	{ key = "skills", label = t("core:inspector.tab.skills", "Skills") },
	{ key = "work", label = t("core:inspector.tab.work", "Work") },
}

local function overview(p)
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
end

local function skills(p)
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
end

-- Each work type: its level (the rules' reason as the tooltip) and why the
-- colonist takes it or passes it over: the why panel (DESIGN.md §4d).
local function work(view, p)
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
end

local function pawn_panel(view, p)
	local title = if p.founder then string.format(t("core:inspector.founder", "%s (founder)"), p.name) else p.name
	local doing = if p.drafted then t("core:inspector.drafted", "drafted") else p.job
	local hint = if p.player
		then (if p.drafted then t("core:inspector.hint_drafted", "R undraft · right-click move/attack") else t("core:inspector.hint", "R draft · right-click to order"))
		else nil
	local tab = if p.player then ui.state("core:inspector.tab", "overview") else "overview"
	local body = if tab == "skills" then skills(p) elseif tab == "work" then work(view, p) else overview(p)
	return kit.panel({ w = 300, gap = "m" }, {
		col({ gap = "xxs" }, {
			label(title, { size = "heading", weight = "strong" }),
			label(string.format(t("core:inspector.sub", "%s · %s · %s"), p.label, p.faction, doing), { size = "small", color = "muted" }),
		}),
		p.player and kit.tabs({
			id = "core:inspector.tabs",
			tabs = TABS,
			selected = tab,
			on_select = function(key)
				ui.set_state("core:inspector.tab", key)
			end,
		}),
		body,
		hint and label(hint, { size = "small", color = "faint" }),
	})
end

local function thing_panel(th)
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
	return kit.panel({ id = "core:inspector.panel", w = 300, gap = "m" }, {
		col({ gap = "xxs" }, {
			label(th.label, { size = "heading", weight = "strong" }),
			#facts > 0 and label(table.concat(facts, " · "), { size = "small", color = "muted" }),
		}),
		th.max_hp > 0 and kit.bar({ label = t("core:inspector.hp", "hp"), value = th.hp / th.max_hp, color = "good" }),
		th.why and label(th.why, { id = "core:inspector.why", size = "small", color = "bad", wrap = true }),
		-- Extension point: mods add sections for things (a station's bills,
		-- a tool's wear). They read the thing with view.thing(view.selected()).
		col({ id = "core:inspector.thing", gap = "s" }),
	})
end

ui.define("core:inspector", function(view)
	local id = view.selected()
	if not id then
		return nil
	end
	local p = view.pawn(id)
	if p then
		return pawn_panel(view, p)
	end
	local th = view.thing(id)
	if th then
		return thing_panel(th)
	end
	return nil
end)
ui.mount("left", "core:inspector", { align = "end" })

-- ─── Hover readout: what's under the cursor ────────────────────────────────

ui.define("core:hover", function(view)
	local h = view.hover()
	if not h then
		return nil
	end
	local lines = {
		label(string.format(t("core:hover.cell", "%s (%d, %d)  %s"), h.terrain, h.x, h.y, h.shelter), { size = "small", color = "muted" }),
	}
	if #h.readings > 0 then
		table.insert(lines, label(table.concat(h.readings, "   "), { size = "small" }))
	end
	for _, t in h.things do
		table.insert(lines, label(t))
	end
	if h.takes then
		table.insert(lines, label(h.takes, { id = "core:hover.takes", size = "small", color = "accent" }))
	end
	return kit.panel({ gap = "xxs", maxw = 460 }, lines)
end)
ui.mount("right", "core:hover", { align = "end" })

-- ─── Order hint: what a right-click would do, beside the cursor ────────────

ui.define("core:hint", function(view)
	local text = view.hint()
	if not text then
		return nil
	end
	return kit.panel({ pad = "s", padx = "l", bg = "surface_raised", border = "accent" }, { label(text) })
end)
ui.mount("cursor", "core:hint")

-- ─── Profiler (F3) ─────────────────────────────────────────────────────────

ui.define("core:profiler", function(view)
	if not view.show_profiler() then
		return nil
	end
	local rows = { kit.heading(t("core:profiler.title", "Profiler"), t("core:profiler.note", "µs per call, smoothed")) }
	local entries = view.profile()
	table.sort(entries, function(a, b)
		if a.mod ~= b.mod then
			return not a.mod
		end
		return a.us > b.us
	end)
	for _, e in entries do
		table.insert(
			rows,
			row({ gap = "l" }, {
				label(e.name, { size = "small", color = if e.mod then "accent" else "text" }),
				{ kind = "spacer" },
				label(string.format("%.1f", e.us), { size = "small" }),
			})
		)
	end
	table.insert(rows, kit.divider())
	for _, s in view.stats() do
		table.insert(rows, label(s, { size = "small", color = "muted" }))
	end
	local u = view.ui_stats()
	table.insert(rows, label(string.format(t("core:profiler.ui", "ui: build %.0f · layout %.0f · paint %.0f µs · %d nodes"), u.build_us, u.layout_us, u.paint_us, u.nodes), { size = "small", color = "muted" }))
	table.insert(rows, label(string.format(t("core:profiler.font", "font: %s"), u.font), { size = "small", color = "faint" }))
	table.insert(rows, kit.divider())
	table.insert(rows, label(t("core:profiler.mods", "Mods (load order)"), { weight = "strong" }))
	for _, m in view.mods() do
		table.insert(rows, label(string.format(t("core:profiler.mod", "%s %s (%s)"), m.id, m.version, m.name), { size = "small" }))
	end
	local warnings = view.warnings()
	if #warnings > 0 then
		table.insert(rows, kit.divider())
		table.insert(rows, label(t("core:profiler.warnings", "Warnings"), { weight = "strong", color = "bad" }))
		for _, w in warnings do
			table.insert(rows, label(w, { size = "small", color = "bad", wrap = true }))
		end
	end
	return kit.panel({ id = "core:profiler.panel", w = 360, gap = "xs" }, rows)
end)
ui.mount("right", "core:profiler", { align = "start", order = 10 })

-- ─── Colony lost ───────────────────────────────────────────────────────────

ui.define("core:lost", function(view)
	if not view.colony_lost() then
		return nil
	end
	return kit.panel({ pad = "xl", gap = "s", align = "center", bg = "surface_raised" }, {
		label(t("core:lost.title", "The colony is lost"), { size = "title", weight = "strong", color = "threat" }),
		label(string.format(t("core:lost.days", "It lasted %d days."), view.day()), { color = "muted" }),
	})
end)
ui.mount("modal", "core:lost")
