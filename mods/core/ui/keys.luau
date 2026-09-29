-- Keys: every action core offers is a named binding, so it is reachable
-- from its key, from the command palette, and from a player's keybinds
-- file without core drawing a button for it. A mod adds its own the same
-- way; two mods on one key is reported.
local kit = require("@core/ui/kit")
-- 1 to 3 pick a thing while a dock tray is open, and set the speed otherwise.
local dock = require("@core/ui/toolbar")
local t = ui.t

ui.bind("core:pause", { key = "space", label = t("core:keys.pause", "Pause or resume") }, function()
	act.toggle_pause()
end)
ui.bind("core:speed1", { key = "1", label = t("core:keys.speed1", "Normal speed") }, function()
	if not dock.number(1) then
		act.speed(1)
	end
end)
ui.bind("core:speed2", { key = "2", label = t("core:keys.speed2", "Fast") }, function()
	if not dock.number(2) then
		act.speed(3)
	end
end)
ui.bind("core:speed3", { key = "3", label = t("core:keys.speed3", "Fastest") }, function()
	if not dock.number(3) then
		act.speed(6)
	end
end)
ui.bind("core:profiler", { key = "f3", label = t("core:keys.profiler", "Profiler") }, function()
	act.toggle_profiler()
end)
ui.bind("core:devtools", { key = "f12", label = t("core:keys.devtools", "Devtools") }, function()
	act.toggle_devtools()
end)
-- The map's zoom, a wheel notch at a time, about the middle of the screen.
ui.bind("core:zoom_in", { key = "=", label = t("core:keys.zoom_in", "Zoom in") }, function()
	act.zoom(1.12)
end)
ui.bind("core:zoom_out", { key = "-", label = t("core:keys.zoom_out", "Zoom out") }, function()
	act.zoom(1 / 1.12)
end)
-- What a scroll does on the map. Auto sorts a trackpad (pans) from a wheel
-- (zooms); the others are for a device that reads as the wrong one.
ui.bind("core:scroll_auto", { label = t("core:keys.scroll_auto", "Scroll: trackpad pans, wheel zooms") }, function()
	act.scroll_mode("auto")
end)
ui.bind("core:scroll_zoom", { label = t("core:keys.scroll_zoom", "Scroll: always zooms") }, function()
	act.scroll_mode("zoom")
end)
ui.bind("core:scroll_pan", { label = t("core:keys.scroll_pan", "Scroll: always pans") }, function()
	act.scroll_mode("pan")
end)

-- UI scale, on top of the display's: saved with the player's settings.
local UI_STEP = 0.125
ui.bind("core:ui_bigger", { key = "ctrl+=", label = t("core:keys.ui_bigger", "Bigger interface") }, function()
	act.ui_scale(view.ui_scale() + UI_STEP)
end)
ui.bind("core:ui_smaller", { key = "ctrl+-", label = t("core:keys.ui_smaller", "Smaller interface") }, function()
	act.ui_scale(view.ui_scale() - UI_STEP)
end)
ui.bind("core:ui_normal", { key = "ctrl+0", label = t("core:keys.ui_normal", "Interface at normal size") }, function()
	act.ui_scale(1)
end)
-- Render scale: the world at fewer pixels than the screen has, for a
-- laptop whose integrated GPU can't fill a high-DPI screen. No keys; the
-- palette finds them.
ui.bind("core:render_scale_100", { label = t("core:keys.render_scale_100", "Render scale 100%") }, function()
	act.render_scale(1)
end)
ui.bind("core:render_scale_75", { label = t("core:keys.render_scale_75", "Render scale 75%") }, function()
	act.render_scale(0.75)
end)
ui.bind("core:render_scale_50", { label = t("core:keys.render_scale_50", "Render scale 50%") }, function()
	act.render_scale(0.5)
end)
-- Lighting presets, for the machine the game runs on. No keys either.
ui.bind("core:lighting_low", { label = t("core:keys.lighting_low", "Lighting: low") }, function()
	act.lighting("low")
end)
ui.bind("core:lighting_medium", { label = t("core:keys.lighting_medium", "Lighting: medium") }, function()
	act.lighting("medium")
end)
ui.bind("core:lighting_high", { label = t("core:keys.lighting_high", "Lighting: high") }, function()
	act.lighting("high")
end)
ui.bind("core:lighting_ultra", { label = t("core:keys.lighting_ultra", "Lighting: ultra") }, function()
	act.lighting("ultra")
end)
ui.bind("core:lighting_auto", { label = t("core:keys.lighting_auto", "Lighting: auto") }, function()
	act.lighting("auto")
end)
-- Out of the colony, saved, to the title. No key: a stray press shouldn't
-- end a game; the palette and the system menu reach it.
ui.bind("core:leave", { label = t("core:keys.leave", "Leave to title") }, function()
	act.leave()
end)
ui.bind("core:overlay", { key = "o", label = t("core:keys.overlay", "Next overlay") }, function()
	act.cycle_overlay()
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

-- Motion on the map: still for players who'd rather. No keys; the
-- palette finds them.
ui.bind("core:reduce_motion_on", { label = t("core:keys.reduce_motion_on", "Reduce motion: on") }, function()
	act.reduce_motion(true)
end)
ui.bind("core:reduce_motion_off", { label = t("core:keys.reduce_motion_off", "Reduce motion: off") }, function()
	act.reduce_motion(false)
end)

-- The measuring grid: every fifth line heavier, for counting cells.
ui.bind("core:measure", { key = "g", label = t("core:keys.measure", "Measure grid") }, function()
	act.toggle_measure()
end)

-- Turn a bed, a bench or anything with a facing before placing it.
ui.bind("core:turn", { key = "t", label = t("core:keys.turn", "Turn what you're placing") }, function()
	act.turn()
end)
