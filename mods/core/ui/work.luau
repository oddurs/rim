-- The Work Board (DESIGN.md §4d): a colonist does the work at its lowest
-- level that has something to do. A row per colonist, a column per work
-- type in tie-break order.
--
-- Painted, not typed: a press picks a value (the brush, or the cell's next
-- level) and a drag paints it; the wheel nudges a cell, shift-wheel its
-- column. A cell is brighter the sooner it comes, carries the colonist's
-- skill as a bar, and reads `3→1` when a rule or the stance moves it; its
-- tooltip says why. Column headers show the work waiting and mark a column
-- nobody is on at a high priority. A colonist's name opens their ranked
-- view, which writes the same numbers.
local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

local NAME_W = 110
local CELL_W = 56
local CELL_H = 20
local JOB_W = 150

local function show(n: number): string
	return if n == 0 then "–" else tostring(n)
end

local function next_level(level: number, levels: number): number
	if level == 0 then
		return 1
	end
	return if level >= levels then 0 else level + 1
end

-- Whole wheel steps (the engine adds up a trackpad's fractions): up is
-- sooner. Never stays never; the brush sets it.
local function nudge(level: number, steps: number, levels: number): number
	if level == 0 then
		return 0
	end
	return math.clamp(level - steps, 1, levels)
end

-- Brighter the sooner it comes: the accent at an alpha by level.
local function shade(value: number, levels: number): string?
	if value == 0 then
		return nil
	end
	local a = if levels <= 1 then 0x90 else math.floor(0x90 - (value - 1) * (0x70 / (levels - 1)))
	return string.format("#5ab4ff%02x", a)
end

local function stance_bar(view)
	local bar = { label(t("core:work.stance", "Stance"), { size = "small", color = "muted" }) }
	for _, st in view.stances() do
		table.insert(
			bar,
			button({
				id = "core:work.stance." .. st.id,
				label = st.label,
				active = st.active,
				on_click = function()
					act.set_stance(st.id)
				end,
			})
		)
	end
	return row({ gap = "xs", align = "center" }, bar)
end

-- The brush: what a press paints. Unset, a press steps the cell.
local function brush_bar(levels: number)
	local brush = ui.state("core:work.brush", -1)
	local items = { label(t("core:work.paint", "Paint"), { size = "small", color = "muted" }) }
	local function add(value: number, text: string)
		table.insert(
			items,
			button({
				id = "core:work.brush." .. value,
				label = text,
				active = brush == value,
				on_click = function()
					ui.set_state("core:work.brush", if brush == value then -1 else value)
				end,
			})
		)
	end
	add(-1, t("core:work.step", "Step"))
	for l = 1, levels do
		add(l, tostring(l))
	end
	add(0, "–")
	return row({ gap = "xs", align = "center" }, items)
end

ui.window("core:work", { title = t("core:work.title", "Work"), w = 760, h = 860, resizable = true }, function(view)
	local b = view.board()
	local levels, cols, rows = b.levels, b.cols, b.rows

	-- Column headers: the label, then what's waiting.
	local header = ui.grid({
		id = "core:work.header",
		rows = 2,
		cols = #cols,
		cell_w = CELL_W,
		cell_h = CELL_H,
		gap = 2,
		cell = function(r, c)
			local k = cols[c]
			if r == 1 then
				local tip = if k.skill then string.format(t("core:work.col_skill", "%s · uses %s"), k.label, k.skill) else k.label
				return { text = k.label, color = "muted", tip = tip }
			end
			local starved = k.waiting > 0 and k.high == 0
			return {
				text = tostring(k.waiting),
				color = if starved then "bad" elseif k.waiting == 0 then "faint" else "text",
				bg = if starved then "#ffb35a2e" else nil,
				tip = string.format(
					t("core:work.demand", "%s: %d waiting · %d on it, %d at 1–%d"),
					k.label,
					k.waiting,
					k.on,
					k.high,
					b.high
				),
			}
		end,
	})

	local grid = ui.grid({
		id = "core:work.grid",
		rows = #rows,
		cols = #cols,
		cell_w = CELL_W,
		cell_h = CELL_H,
		gap = 2,
		cell = function(r, c)
			local x = rows[r].cells[c]
			return {
				text = if x.base == x.value then show(x.base) else show(x.base) .. "→" .. show(x.value),
				bg = shade(x.value, levels) or "track",
				color = if x.value == 0 then "faint" else "text",
				bar = x.skill_frac,
				bar_color = "good",
				tip = if x.skill then string.format(t("core:work.cell_skill", "%s (skill %d)"), x.why, x.skill) else x.why,
			}
		end,
		on_press = function(r, c)
			local brush = ui.state("core:work.brush", -1)
			return if brush >= 0 then brush else next_level(rows[r].cells[c].base, levels)
		end,
		on_paint = function(r, c, value)
			act.set_priority(rows[r].id, cols[c].id, value)
		end,
		on_wheel = function(r, c, steps, shift)
			local targets = if shift then rows else { rows[r] }
			for _, p in targets do
				local base = p.cells[c].base
				local now = nudge(base, steps, levels)
				if now ~= base then
					act.set_priority(p.id, cols[c].id, now)
				end
			end
		end,
	})

	-- Names open the ranked view; the job column says what each is doing.
	local names = ui.grid({
		id = "core:work.names",
		rows = #rows,
		cols = 1,
		cell_w = NAME_W,
		cell_h = CELL_H,
		gap = 2,
		cell = function(r, _c)
			return { text = rows[r].name, tip = t("core:work.rank_tip", "Rank their work") }
		end,
		on_press = function(r, _c)
			ui.set_state("core:work.ranked", rows[r].id)
			ui.open("core:work.ranked")
			return nil
		end,
	})
	local jobs = ui.grid({
		id = "core:work.jobs",
		rows = #rows,
		cols = 1,
		cell_w = JOB_W,
		cell_h = CELL_H,
		gap = 2,
		cell = function(r, _c)
			return { text = rows[r].job, color = "muted" }
		end,
	})

	return col({ gap = "s", pad = "m" }, {
		stance_bar(view),
		brush_bar(levels),
		row({ gap = "s" }, { row({ w = NAME_W }), header }),
		-- The window scrolls; the wheel over a cell nudges it instead.
		row({ gap = "s" }, { names, grid, jobs }),
	})
end)

-- One colonist's work in order: the same numbers as the board, read as a
-- list. ▲ moves a work type a level sooner, ▼ later; – is never.
ui.window("core:work.ranked", { title = t("core:work.ranked", "Ranked"), w = 280, h = 380, resizable = true }, function(view)
	local id = ui.state("core:work.ranked", nil)
	local b = view.board()
	local who
	for _, r in b.rows do
		if r.id == id then
			who = r
		end
	end
	if not who then
		return col({ pad = "m" }, { label(t("core:work.pick", "Pick a colonist on the Work board."), { color = "muted" }) })
	end
	local order = {}
	for c, k in b.cols do
		table.insert(order, { col = k, cell = who.cells[c], rank = c })
	end
	table.sort(order, function(x, y)
		local a = if x.cell.base == 0 then 99 else x.cell.base
		local z = if y.cell.base == 0 then 99 else y.cell.base
		if a ~= z then
			return a < z
		end
		return x.rank < y.rank
	end)
	local items = { label(who.name, { size = "heading", weight = "strong" }) }
	local last = nil
	for _, o in order do
		local base = o.cell.base
		if base ~= last then
			table.insert(items, label(if base == 0 then t("core:work.never", "Never") else string.format(t("core:work.level", "Level %d"), base), { size = "small", color = "muted" }))
			last = base
		end
		local set = function(level: number)
			act.set_priority(who.id, o.col.id, level)
		end
		table.insert(
			items,
			row({ id = "core:work.ranked." .. o.col.id, gap = "xs", align = "center" }, {
				row({ w = 110 }, { label(o.col.label) }),
				button({ id = "core:work.ranked." .. o.col.id .. ".up", label = "▲", disabled = base == 1, on_click = function()
					set(if base == 0 then b.levels else base - 1)
				end }),
				button({ id = "core:work.ranked." .. o.col.id .. ".down", label = "▼", disabled = base == 0 or base == b.levels, on_click = function()
					set(base + 1)
				end }),
				button({ id = "core:work.ranked." .. o.col.id .. ".never", label = "–", disabled = base == 0, on_click = function()
					set(0)
				end }),
			})
		)
	end
	return ui.scroll({ pad = "m", gap = "xs", table.unpack(items) })
end)

ui.bind("core:work", { key = "p", label = t("core:keys.work", "Work priorities") }, function()
	ui.toggle("core:work")
end)
