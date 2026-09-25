-- Keys: every action core offers is a named binding, so it is reachable
-- from its key, from the command palette, and from a player's keybinds
-- file without core drawing a button for it. A mod adds its own the same
-- way; two mods on one key is reported.
local kit = require("@core/ui/kit")
local t = ui.t

ui.bind("core:pause", { key = "space", label = t("core:keys.pause", "Pause or resume") }, function()
	act.toggle_pause()
end)
ui.bind("core:speed1", { key = "1", label = t("core:keys.speed1", "Normal speed") }, function()
	act.speed(1)
end)
ui.bind("core:speed2", { key = "2", label = t("core:keys.speed2", "Fast") }, function()
	act.speed(3)
end)
ui.bind("core:speed3", { key = "3", label = t("core:keys.speed3", "Fastest") }, function()
	act.speed(6)
end)
ui.bind("core:profiler", { key = "f3", label = t("core:keys.profiler", "Profiler") }, function()
	act.toggle_profiler()
end)
ui.bind("core:devtools", { key = "f12", label = t("core:keys.devtools", "Devtools") }, function()
	act.toggle_devtools()
end)
-- Render scale: the world at fewer pixels than the screen has, for a
-- laptop whose integrated GPU can't fill a high-DPI screen. No keys; the
-- palette finds them.
for _, pct in { 100, 75, 50 } do
	ui.bind(`core:render_scale_{pct}`, { label = t(`core:keys.render_scale_{pct}`, `Render scale {pct}%`) }, function()
		act.render_scale(pct / 100)
	end)
end
ui.bind("core:overlay", { key = "o", label = t("core:keys.overlay", "Next overlay") }, function()
	act.cycle_overlay()
end)
ui.bind("core:draft", { key = "r", label = t("core:keys.draft", "Draft or undraft the selected colonist") }, function()
	local id = view.selected()
	local p = id and view.pawn(id)
	if p and p.player then
		act.draft(id, not p.drafted)
	end
end)
ui.bind("core:center", { key = "c", label = t("core:keys.center", "Centre on the selected pawn") }, function()
	local id = view.selected()
	if id then
		act.focus(id)
	end
end)
ui.bind("core:palette", { key = "ctrl+k", label = t("core:keys.palette", "Command palette") }, function()
	-- Window ops apply after the handler, so ask before toggling.
	local opening = not ui.is_open("core:palette")
	ui.toggle("core:palette")
	if opening then
		ui.set_input("core:palette.query", "")
		ui.set_state("core:palette.query", "")
		ui.set_state("core:palette.sel", 1)
		ui.focus("core:palette.query")
	end
end)

-- Every bound action, filtered by what is typed; Enter runs the first.
local function matches(query)
	local out = {}
	local q = string.lower(query)
	for _, b in view.binds() do
		if q == "" or string.find(string.lower(b.label .. " " .. b.id), q, 1, true) then
			table.insert(out, b)
		end
	end
	return out
end

local function run(id)
	ui.close("core:palette")
	ui.run(id)
end

ui.window("core:palette", { title = t("core:palette.title", "Commands"), w = 440, h = 360 }, function(view)
	local query = ui.state("core:palette.query", "")
	local found = matches(query)
	-- The arrows move a selection; Enter runs it. It stays in range as
	-- the query narrows.
	local sel = math.clamp(ui.state("core:palette.sel", 1), 1, math.max(1, #found))
	local rows = {}
	for i, b in found do
		table.insert(
			rows,
			kit.row({
				id = "core:palette." .. b.id,
				pad = "s",
				padx = "m",
				gap = "m",
				align = "center",
				radius = "radius",
				bg = if i == sel then "surface_hover" else nil,
				hover = { bg = "surface_hover" },
				on_click = function()
					run(b.id)
				end,
			}, {
				kit.label(b.label),
				{ kind = "spacer" },
				kit.label(b.key, { size = "small", color = "muted" }),
			})
		)
	end
	if #rows == 0 then
		table.insert(rows, kit.label(t("core:palette.none", "No command matches"), { size = "small", color = "muted" }))
	end
	return kit.col({ gap = "s" }, {
		kit.input({
			id = "core:palette.query",
			placeholder = t("core:palette.hint", "Type a command"),
			w = "fill",
			on_change = function(text)
				ui.set_state("core:palette.query", text)
				ui.set_state("core:palette.sel", 1)
			end,
			on_key = function(key)
				local n = #matches(ui.state("core:palette.query", ""))
				local at = ui.state("core:palette.sel", 1)
				ui.set_state("core:palette.sel", math.clamp(at + (if key == "down" then 1 else -1), 1, math.max(1, n)))
			end,
			on_submit = function(text)
				local pick = matches(text)[math.clamp(ui.state("core:palette.sel", 1), 1, math.huge)]
				if pick then
					run(pick.id)
				end
			end,
		}),
		kit.col({ gap = 0 }, rows),
	})
end)
