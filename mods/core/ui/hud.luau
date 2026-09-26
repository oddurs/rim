-- Core's HUD, written on the UI system like any mod's would be.
--
-- Every part has a namespaced id, so other mods can extend, replace, wrap
-- or remove it (ui.extend("core:topbar.right", ...), ui.remove("core:hover")).

local kit = require("@core/ui/kit")
local row, label = kit.row, kit.label
local t = ui.t

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

ui.define("core:topbar", function(view)
	return row({ padx = "l", pady = "s", gap = "xl", align = "center", bg = "surface", radius = 0 }, {
		ui.slot("core:clock"),
		ui.slot("core:status"),
		{ kind = "spacer" },
		-- Extension point: mods add readouts here.
		row({ id = "core:topbar.right", gap = "l", align = "center" }),
		label(t("core:topbar.help", "Space pause · 1/2/3 speed · R draft · O overlay · F3 profiler · F12 devtools"), { size = "small", color = "faint" }),
	})
end)
ui.mount("top", "core:topbar")


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
