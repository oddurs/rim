-- Work priorities: a colonist does the work at its lowest level that has
-- something to do (DESIGN.md §4d). A plain grid until the Work Board lands:
-- a row per colonist, a column per work type, and a click steps a cell
-- through 1, 2, … up to the scale's levels, then 0 (never), then 1 again.
local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

local NAME_W = 110
local CELL_W = 64

-- A fixed-width cell, so the columns line up.
local function cell(w: number, child)
	return row({ w = w, align = "center" }, { child })
end

local function next_level(level: number, levels: number): number
	if level == 0 then
		return 1
	end
	return if level >= levels then 0 else level + 1
end

ui.window("core:work", { title = t("core:work.title", "Work"), w = 520, h = 360, resizable = true }, function(view)
	local types = view.work_types()
	local levels = view.priority_levels()
	local header = { cell(NAME_W, label("")) }
	for _, w in types do
		table.insert(header, cell(CELL_W, label(w.label, { size = "small", color = "muted" })))
	end
	local rows = { row({ gap = "xs" }, header) }
	for _, p in view.colonists() do
		local prio = view.priorities(p.id) or {}
		local cells = { cell(NAME_W, label(p.name)) }
		for _, w in types do
			local level = prio[w.id] or 0
			table.insert(
				cells,
				cell(CELL_W, button({
					id = "core:work." .. p.name .. "." .. w.id,
					label = if level == 0 then "–" else tostring(level),
					active = level == 1,
					color = if level == 0 then "faint" else nil,
					tooltip = string.format(t("core:work.cell", "%s: %s"), p.name, w.label),
					on_click = function()
						act.set_priority(p.id, w.id, next_level(level, levels))
					end,
				}))
			)
		end
		table.insert(rows, row({ gap = "xs", align = "center" }, cells))
	end
	return col({ gap = "xs", pad = "m" }, rows)
end)

ui.bind("core:work", { key = "p", label = t("core:keys.work", "Work priorities") }, function()
	ui.toggle("core:work")
end)
