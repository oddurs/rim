-- Core's HUD, written on the UI system like any mod's would be.
--
-- Every part has a namespaced id, so other mods can extend, replace, wrap
-- or remove it (ui.extend("core:topbar.right", ...), ui.remove("core:hover")).

local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button

local function pct(v: number): string
	return tostring(math.floor(v * 100 + 0.5)) .. "%"
end

-- ─── Top bar: clock, wealth, weather, colonists ─────────────────────────────

-- "Spring 9", and the year once there's more than one.
local function date(view): string
	local d = view.date()
	local s = string.upper(string.sub(d.season, 1, 1)) .. string.sub(d.season, 2) .. " " .. d.day
	return if d.year > 1 then s .. ", year " .. d.year else s
end

ui.define("core:clock", function(view)
	local speed = if view.paused() then "paused" else view.speed() .. "×"
	return row({ gap = "m", align = "center" }, {
		label("Day " .. view.day(), { weight = "strong" }),
		label(date(view), { id = "core:clock.date", color = "muted" }),
		label(view.clock(), { weight = "strong" }),
		label(speed, { color = if view.paused() then "bad" else "muted" }),
	})
end)

ui.define("core:status", function(view)
	local parts = { label("Wealth " .. math.floor(view.wealth()), { color = "muted" }) }
	for _, f in view.fields() do
		if f.hud then
			table.insert(parts, label(string.format("%.0f%s outside", f.ambient, f.unit), { color = "muted" }))
		end
	end
	local overlay = view.overlay()
	if overlay then
		table.insert(parts, label("overlay: " .. overlay, { color = "accent" }))
	end
	return row({ gap = "l", align = "center" }, parts)
end)

ui.define("core:colonists", function(view)
	local items = {}
	for _, p in view.colonists() do
		local health = if p.health > 0.5 then "good" else "bad"
		table.insert(
			items,
			button({
				id = "core:colonists." .. p.name,
				label = p.name,
				active = p.selected,
				swatch = health,
				color = if p.drafted then "player" else nil,
				tooltip = p.label .. " · " .. p.job .. " · health " .. pct(p.health),
				on_click = function()
					act.select(p.id)
					act.focus(p.id)
				end,
			})
		)
	end
	return row({ gap = "xs", align = "center" }, items)
end)

ui.define("core:topbar", function(view)
	return row({ padx = "m", pady = "xs", gap = "xl", align = "center", bg = "surface", radius = 0 }, {
		ui.slot("core:clock"),
		ui.slot("core:status"),
		ui.slot("core:colonists"),
		{ kind = "spacer" },
		-- Extension point: mods add readouts here.
		row({ id = "core:topbar.right", gap = "l", align = "center" }),
		label("Space pause · 1/2/3 speed · R draft · O overlay · F3 profiler · F12 devtools", { size = "small", color = "faint" }),
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
	local buttons = row({ id = "core:toolbar.buttons", pad = "s", gap = "xs", bg = "surface", radius = 0, align = "center" }, items)

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
				label = if none then m.label .. " · none" else `{m.label} ×{m.have}`,
				active = m.active,
				swatch = m.color,
				color = if none then "faint" else nil,
				tooltip = `hp {m.hp} · {m.work} work`,
				on_click = function()
					act.stuff(m.id)
				end,
			})
		)
	end
	if chosen then
		table.insert(picks, label(`hp {chosen.hp} · {chosen.work} work`, { size = "small", color = "muted" }))
	end
	local materials = row({ id = "core:stuff", pad = "s", gap = "xs", bg = "surface", radius = 0, align = "center" }, picks)
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

-- ─── Inspector: the selected pawn ──────────────────────────────────────────

ui.define("core:inspector", function(view)
	local id = view.selected()
	local p = id and view.pawn(id)
	if not p then
		return nil
	end
	local title = if p.founder then p.name .. " (founder)" else p.name
	local doing = if p.drafted then "drafted" else p.job
	local bars = { kit.bar({ label = "health", value = p.health, color = "good" }) }
	for _, n in p.needs do
		table.insert(bars, kit.bar({ id = "core:inspector.need." .. n.id, label = n.label, value = n.value, color = if n.low then "bad" else n.color }))
	end
	local hint = if p.player
		then (if p.drafted then "R undraft · right-click move/attack" else "R draft · right-click to order")
		else nil
	return kit.panel({ w = 280, gap = "s" }, {
		col({ gap = "xxs" }, {
			label(title, { size = "heading", weight = "strong" }),
			label(p.label .. " · " .. p.faction .. " · " .. doing, { size = "small", color = "muted" }),
		}),
		col({ id = "core:inspector.bars", gap = "xs" }, bars),
		-- Extension point: mods add sections (mood, skills...).
		col({ id = "core:inspector.sections", gap = "s" }),
		hint and label(hint, { size = "small", color = "faint" }),
	})
end)
ui.mount("left", "core:inspector", { align = "end" })

-- ─── Hover readout: what's under the cursor ────────────────────────────────

ui.define("core:hover", function(view)
	local h = view.hover()
	if not h then
		return nil
	end
	local lines = {
		label(string.format("%s (%d, %d)  %s", h.terrain, h.x, h.y, h.shelter), { size = "small", color = "muted" }),
	}
	if #h.readings > 0 then
		table.insert(lines, label(table.concat(h.readings, "   "), { size = "small" }))
	end
	for _, t in h.things do
		table.insert(lines, label(t))
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
	return kit.panel({ pad = "xs", padx = "m", bg = "surface_raised", border = "accent" }, { label(text) })
end)
ui.mount("cursor", "core:hint")

-- ─── Profiler (F3) ─────────────────────────────────────────────────────────

ui.define("core:profiler", function(view)
	if not view.show_profiler() then
		return nil
	end
	local rows = { kit.heading("Profiler", "µs per call, smoothed") }
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
	table.insert(rows, label(string.format("ui: build %.0f · layout %.0f · paint %.0f µs · %d nodes", u.build_us, u.layout_us, u.paint_us, u.nodes), { size = "small", color = "muted" }))
	table.insert(rows, label("font: " .. u.font, { size = "small", color = "faint" }))
	table.insert(rows, kit.divider())
	table.insert(rows, label("Mods (load order)", { weight = "strong" }))
	for _, m in view.mods() do
		table.insert(rows, label(string.format("%s %s (%s)", m.id, m.version, m.name), { size = "small" }))
	end
	local warnings = view.warnings()
	if #warnings > 0 then
		table.insert(rows, kit.divider())
		table.insert(rows, label("Warnings", { weight = "strong", color = "bad" }))
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
		label("The colony is lost", { size = "title", weight = "strong", color = "threat" }),
		label("It lasted " .. view.day() .. " days.", { color = "muted" }),
	})
end)
ui.mount("modal", "core:lost")
