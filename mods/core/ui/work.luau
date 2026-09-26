-- The Work Board (DESIGN.md §4d): a colonist does the work at its lowest
-- level that has something to do. A row per colonist, a column per work
-- type in tie-break order.
--
-- Painted, not typed: a press picks a value (the brush, or the cell's next
-- level) and a drag paints it; the wheel nudges a cell, shift-wheel its
-- column; over a cell, a number sets it, N sets never and A hands it back.
-- A cell is brighter the sooner it comes, carries the colonist's skill as a
-- bar, and reads `3→1` when a rule or the stance moves it; its tooltip says
-- why. A value the player set is a pin, drawn bold with a dot; it exists
-- only while it differs from what the colonist would inherit, so painting
-- the inherited value back hands the cell back. Column headers show the
-- work waiting and mark a column nobody is on at a high priority. A
-- colonist's name opens their levels as shelves, which write the same
-- settings.
--
-- The window shows one lens at a time: the board, one person's shelves,
-- and any a mod adds (DESIGN.md §4d). A lens is a view over the same
-- board data, and writes only through `act`, so no lens can disagree with
-- another:
--
--     local work = require("@core/ui/work")
--     work.lens({ id = "my_mod:crews", label = "Crews", order = 40, draw = function(view, board)
--         return kit.label(#board.rows .. " colonists")
--     end })
--
-- Tabs are `core:work.lens.<id>`, sorted by `order` (core's are 10 and
-- 20), then in the order they were added.
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

-- Set a cell, or hand it back when the value is what it would inherit:
-- the colonist keeps a setting only while it differs.
local function put(id: number, work: string, cell, level: number)
	if level == cell.inherit then
		if cell.pinned then
			act.clear_priority(id, work)
		end
	elseif level ~= cell.base or not cell.pinned then
		act.set_priority(id, work, level)
	end
end

-- The keys a hovered cell takes: a level's number, N for never, A to hand
-- it back.
local function cell_keys(levels: number): { string }
	local keys = { "n", "a" }
	for l = 1, levels do
		table.insert(keys, tostring(l))
	end
	return keys
end

-- What a level is called: its label, or "never".
local function level_name(labels: { string }, level: number): string
	return if level == 0 then t("core:work.never_short", "never") else labels[level] or tostring(level)
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
local function brush_bar(levels: number, labels: { string })
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
		add(l, labels[l] or tostring(l))
	end
	add(0, t("core:work.never", "Never"))
	return row({ gap = "xs", align = "center" }, items)
end

local work = {}

local lenses: { any } = {}
local added = 0

-- Register a lens. A second `lens` with the same id replaces the first and
-- keeps its place, so a reloaded script doesn't double up.
function work.lens(def)
	assert(type(def) == "table" and type(def.id) == "string", "work.lens: needs an id")
	assert(type(def.label) == "string", `work.lens('{def.id}'): needs a label`)
	assert(type(def.draw) == "function", `work.lens('{def.id}'): needs draw = fn(view, board)`)
	local seq = nil
	for i, l in lenses do
		if l.id == def.id then
			seq = l.seq
			table.remove(lenses, i)
			break
		end
	end
	if not seq then
		added += 1
		seq = added
	end
	table.insert(lenses, { id = def.id, label = def.label, order = def.order or 100, seq = seq, draw = def.draw })
	table.sort(lenses, function(a, b)
		if a.order ~= b.order then
			return a.order < b.order
		end
		return a.seq < b.seq
	end)
end

-- Show one colonist on the Person lens.
local function show_person(id: number)
	ui.set_state("core:work.person", id)
	ui.set_state("core:work.lens", "core:work.person")
end

-- The board: a row per colonist, a column per work type, painted.
local function board_lens(view, b)
	local levels, labels, cols, rows = b.levels, b.labels, b.cols, b.rows

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
			local tip = if x.skill then string.format(t("core:work.cell_skill", "%s (skill %d)"), x.why, x.skill) else x.why
			if x.pinned then
				tip ..= "\n" .. string.format(t("core:work.pinned", "Pinned. A hands it back to %s."), level_name(labels, x.inherit))
			end
			return {
				text = if x.base == x.value then show(x.base) else show(x.base) .. "→" .. show(x.value),
				bg = shade(x.value, levels) or "track",
				color = if x.value == 0 then "faint" else "text",
				weight = if x.pinned then "strong" else nil,
				dot = if x.pinned then "text" else nil,
				bar = x.skill_frac,
				bar_color = "good",
				tip = tip,
			}
		end,
		on_press = function(r, c)
			local brush = ui.state("core:work.brush", -1)
			return if brush >= 0 then brush else next_level(rows[r].cells[c].base, levels)
		end,
		on_paint = function(r, c, value)
			put(rows[r].id, cols[c].id, rows[r].cells[c], value)
		end,
		on_wheel = function(r, c, steps, shift)
			local targets = if shift then rows else { rows[r] }
			for _, p in targets do
				local cell = p.cells[c]
				local now = nudge(cell.base, steps, levels)
				if now ~= cell.base then
					put(p.id, cols[c].id, cell, now)
				end
			end
		end,
		keys = cell_keys(levels),
		on_key = function(r, c, key)
			local cell = rows[r].cells[c]
			if key == "a" then
				if cell.pinned then
					act.clear_priority(rows[r].id, cols[c].id)
				end
				return
			end
			put(rows[r].id, cols[c].id, cell, if key == "n" then 0 else tonumber(key) or cell.base)
		end,
	})

	-- Names open the Person lens; the job column says what each is doing.
	local names = ui.grid({
		id = "core:work.names",
		rows = #rows,
		cols = 1,
		cell_w = NAME_W,
		cell_h = CELL_H,
		gap = 2,
		cell = function(r, _c)
			return { text = rows[r].name, tip = t("core:work.rank_tip", "See their levels as shelves") }
		end,
		on_press = function(r, _c)
			show_person(rows[r].id)
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

	return col({ gap = "s" }, {
		brush_bar(levels, labels),
		row({ gap = "s" }, { row({ w = NAME_W }), header }),
		-- The window scrolls; the wheel over a cell nudges it instead.
		row({ gap = "s" }, { names, grid, jobs }),
	})
end

-- One colonist's work as shelves: a row per level, first to never, each
-- work type a token on the shelf it's on. The same settings as the board:
-- a click moves a token to the next level, bold tokens are pins, and a
-- token set back to what it would inherit is handed back.
local function person_lens(_view, b)
	if #b.rows == 0 then
		return label(t("core:work.none", "No colonists."), { color = "muted" })
	end
	local id = ui.state("core:work.person", nil)
	local who = b.rows[1]
	for _, r in b.rows do
		if r.id == id then
			who = r
		end
	end
	local pick = {}
	for _, r in b.rows do
		table.insert(
			pick,
			button({
				id = "core:work.person." .. r.id,
				label = r.name,
				active = r == who,
				on_click = function()
					show_person(r.id)
				end,
			})
		)
	end
	local items = { row({ gap = "xs", align = "center" }, pick), label(who.name, { size = "heading", weight = "strong" }) }
	for shelf = 1, b.levels + 1 do
		local level = if shelf > b.levels then 0 else shelf
		local tokens = {}
		for c, k in b.cols do
			local cell = who.cells[c]
			if cell.base == level then
				table.insert(
					tokens,
					button({
						id = "core:work.ranked." .. k.id,
						label = k.label,
						strong = cell.pinned,
						tooltip = cell.why,
						on_click = function()
							put(who.id, k.id, cell, next_level(cell.base, b.levels))
						end,
					})
				)
			end
		end
		local name = if level == 0 then t("core:work.never", "Never") else b.labels[level] or tostring(level)
		table.insert(
			items,
			row({ id = "core:work.shelf." .. level, gap = "xs", align = "center" }, {
				row({ w = 90 }, { label(name, { size = "small", color = "muted" }) }),
				row({ gap = "xs" }, tokens),
			})
		)
	end
	return col({ gap = "s" }, items)
end

work.lens({ id = "core:work.board", label = t("core:work.lens_board", "Board"), order = 10, draw = board_lens })
work.lens({ id = "core:work.person", label = t("core:work.lens_person", "Person"), order = 20, draw = person_lens })

-- The Work screen: the stance, the lenses as tabs, and the lens shown.
ui.window("core:work", { title = t("core:work.title", "Work"), w = 760, h = 860, sheet = true }, function(view)
	local b = view.board()
	local current = ui.state("core:work.lens", "core:work.board")
	local shown = lenses[1]
	local tabs = {}
	for _, l in lenses do
		if l.id == current then
			shown = l
		end
	end
	for _, l in lenses do
		table.insert(
			tabs,
			button({
				id = "core:work.lens." .. l.id,
				label = l.label,
				active = l == shown,
				on_click = function()
					ui.set_state("core:work.lens", l.id)
				end,
			})
		)
	end
	return col({ gap = "s", pad = "m" }, {
		stance_bar(view),
		row({ id = "core:work.lenses", gap = "xs", align = "center" }, tabs),
		shown.draw(view, b),
	})
end)

require("@core/ui/screens").add({ id = "core:work", label = t("core:keys.work", "Work"), key = "p", order = 10 })

return work
