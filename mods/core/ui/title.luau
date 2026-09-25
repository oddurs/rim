-- The title screen: continue the last colony, pick another, or start a new
-- one. It's the only thing built before a colony exists (the "title" layer),
-- so a mod can replace it like any other component.

local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

-- "5h ago": the largest unit that fits.
local function ago(seconds: number): string
	for _, u in { { 86400, "d" }, { 3600, "h" }, { 60, "m" } } do
		if seconds >= u[1] then
			return string.format(t("core:title.ago", "%d%s ago"), math.floor(seconds / u[1]), u[2])
		end
	end
	return t("core:title.just_now", "just now")
end

-- Who the first colonist has with them: "with Mira, Oskar and 2 more".
local function company(names: { string }): string
	local n = #names - 1
	if #names == 0 then
		return t("core:title.nobody", "nobody left")
	elseif n == 0 then
		return t("core:title.alone", "alone")
	elseif n == 1 then
		return string.format(t("core:title.with_one", "with %s"), names[2])
	elseif n == 2 then
		return string.format(t("core:title.with_two", "with %s and %s"), names[2], names[3])
	end
	return string.format(t("core:title.with_more", "with %s, %s and %d more"), names[2], names[3], n - 2)
end

-- A colony goes by its first colonist, the founder while they live.
local function name(s): string
	return s.colonists[1] or s.file
end

local function save_row(s)
	return row({
		id = "core:title.save." .. s.file,
		padx = "m",
		pady = "s",
		gap = "l",
		align = "center",
		bg = "surface_raised",
		border = "line",
		hover = if s.error then nil else { bg = "surface_hover", border = "line_strong" },
		press = { bg = "surface_press" },
		on_click = if s.error then nil else function()
			act.load(s.path)
		end,
		tooltip = s.file,
	}, {
		col({ gap = "xs", grow = 1 }, {
			label(name(s), { weight = "strong" }),
			if s.error
				then label(s.error, { size = "small", color = "bad", wrap = true })
				else label(company(s.colonists), { size = "small", color = "muted" }),
		}),
		not s.error and label(string.format(t("core:title.day", "Day %d"), s.day), { color = "muted" }),
		label(ago(s.age), { size = "small", color = "faint" }),
	})
end

ui.define("core:title", function(view)
	local saves = view.saves()
	local last = saves[1]
	local rows = {}
	for _, s in saves do
		table.insert(rows, save_row(s))
	end
	return kit.panel({ id = "core:title.panel", w = 480, pad = "xxl", gap = "l", bg = "surface_raised" }, {
		label(t("core:title.name", "rim"), { size = "title", weight = "strong" }),
		row({ gap = "s" }, {
			last and not last.error and button({
				id = "core:title.continue",
				label = string.format(t("core:title.continue", "Continue: %s, day %d"), name(last), last.day),
				strong = true,
				on_click = function()
					act.load(last.path)
				end,
			}),
			button({ id = "core:title.new", label = t("core:title.new", "New colony"), on_click = act.new_colony }),
		}),
		#rows > 0 and kit.divider(),
		#rows > 0 and label(t("core:title.colonies", "Colonies"), { size = "small", color = "muted" }),
		#rows > 0 and { kind = "scroll", maxh = 360, gap = "xs", table.unpack(rows) },
	})
end)
ui.mount("title", "core:title")
