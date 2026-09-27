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
local menus = require("@core/ui/menus")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

local NAME_W = 110
local ROLE_W = 76
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
	-- The standing orders, behind one button that counts those acting.
	local orders = view.standing()
	if #orders > 0 then
		local acting = 0
		for _, o in orders do
			if o.acting then
				acting += 1
			end
		end
		local open = ui.state("core:work.orders", false)
		table.insert(bar, row({ w = 12 }))
		table.insert(
			bar,
			button({
				id = "core:work.orders",
				label = if acting > 0
					then string.format(t("core:work.orders_on", "Orders · %d on"), acting)
					else t("core:work.orders_none", "Orders"),
				active = open,
				tooltip = t("core:work.orders_tip", "Standing orders: rules that switch on when a colony reading crosses a mark"),
				on_click = function()
					ui.set_state("core:work.orders", not open)
				end,
			})
		)
	end
	return row({ gap = "xs", align = "center" }, bar)
end

-- A reading as the panel shows it: whole numbers plain, the rest to one
-- decimal.
local function reading_text(v: number?): string
	if v == nil then
		return t("core:work.order_unread", "no reading yet")
	end
	return if v == math.floor(v) then tostring(v) else string.format("%.1f", v)
end

-- The standing orders panel: each order, what it does, where its reading
-- is, and a switch.
local function orders_panel(view)
	if not ui.state("core:work.orders", false) then
		return nil
	end
	local lines = {}
	for _, o in view.standing() do
		local state = if not o.enabled
			then t("core:work.order_off", "switched off")
			elseif o.acting then string.format(t("core:work.order_acting", "on · %s"), reading_text(o.value))
			elseif o.crossed and o.season then string.format(t("core:work.order_season", "waiting for %s · %s"), o.season, reading_text(o.value))
			else string.format(t("core:work.order_waiting", "waiting · %s"), reading_text(o.value))
		table.insert(
			lines,
			row({ id = "core:work.order." .. o.id, gap = "s", align = "center" }, {
				button({
					id = "core:work.order." .. o.id .. ".switch",
					label = if o.enabled then t("core:work.order_on_btn", "On") else t("core:work.order_off_btn", "Off"),
					active = o.enabled,
					on_click = function()
						act.set_rule_enabled(o.id, not o.enabled)
					end,
				}),
				col({ gap = "hair" }, {
					label(o.label, { weight = "strong" }),
					label(string.format("%s · %s", o.effect, o.band), { size = "small", color = "muted" }),
				}),
				{ kind = "spacer" },
				label(state, { size = "small", color = if o.acting then "accent" else "muted" }),
			})
		)
	end
	return col({ id = "core:work.orders_panel", gap = "xs", pad = "s", border = "line", radius = "radius" }, lines)
end

-- What Focus is doing now: the stance if it's not the first, and each
-- order acting. Nil when nothing is.
local function focus_now(view)
	local names = {}
	local stances = view.stances()
	for i, st in stances do
		if st.active and i > 1 then
			table.insert(names, st.label)
		end
	end
	for _, o in view.standing() do
		if o.acting then
			table.insert(names, o.label)
		end
	end
	return if #names > 0 then names else nil
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

-- The board's rows grouped by role, in the roles' order; within a role,
-- as the colony lists them.
local function by_role(b)
	local rank = {}
	for i, r in b.roles do
		rank[r.index] = r.order * 1000 + i
	end
	local rows = {}
	for i, r in b.rows do
		table.insert(rows, { row = r, key = (rank[r.role or 0] or 0) * 1000 + i })
	end
	table.sort(rows, function(a, z)
		return a.key < z.key
	end)
	local out = {}
	for _, x in rows do
		table.insert(out, x.row)
	end
	return out
end

-- A colonist's pins: work id to level.
local function pins_of(b, r)
	local pins, n = {}, 0
	for c, k in b.cols do
		if r.cells[c].pinned then
			pins[k.id] = r.cells[c].base
			n += 1
		end
	end
	return pins, n
end

-- Two colonists in the same role pinned into the same shape: at least two
-- pins each, differing in at most one work type. The first such pair the
-- player hasn't said "Not now" to.
local function alike(b)
	local no = ui.state("core:work.alike_no", {})
	for i = 1, #b.rows do
		local a = b.rows[i]
		local pa, na = pins_of(b, a)
		if na >= 2 then
			for j = i + 1, #b.rows do
				local z = b.rows[j]
				local pz, nz = pins_of(b, z)
				if nz >= 2 and z.role == a.role and not no[a.id .. ":" .. z.id] then
					local differ = 0
					for _, k in b.cols do
						if pa[k.id] ~= pz[k.id] then
							differ += 1
						end
					end
					if differ <= 1 then
						return a, z
					end
				end
			end
		end
	end
	return nil, nil
end

-- The offer to make a role from two colonists who work alike.
local function alike_offer(b)
	local a, z = alike(b)
	if not a then
		return nil
	end
	local n = #b.roles + 1
	return row({ id = "core:work.alike", gap = "s", align = "center" }, {
		label(string.format(t("core:work.alike", "%s and %s work alike."), a.name, z.name), { weight = "strong" }),
		label(t("core:work.alike_why", "A role changes them together, and new settlers can join it."), { size = "small", color = "muted" }),
		button({
			id = "core:work.alike.make",
			label = t("core:work.alike_make", "Make a role from them"),
			on_click = function()
				act.role_from_colonist(string.format(t("core:work.crew", "Crew %d"), n), a.id)
				act.assign_role(a.id, n)
				act.assign_role(z.id, n)
			end,
		}),
		button({
			id = "core:work.alike.no",
			label = t("core:work.alike_no", "Not now"),
			on_click = function()
				local no = table.clone(ui.state("core:work.alike_no", {}))
				no[a.id .. ":" .. z.id] = true
				ui.set_state("core:work.alike_no", no)
			end,
		}),
	})
end

-- Show one colonist on the Person lens.
local function show_person(id: number)
	ui.set_state("core:work.person", id)
	ui.set_state("core:work.lens", "core:work.person")
end

-- One colonist is a plan, not a grid (DESIGN.md §4d): their levels as
-- shelves, each work type with Auto's reason and any rule that moved it.
-- A click pins a level; "Show the board" goes to the grid.
local function plan_view(b)
	local who = b.rows[1]
	local planned = false
	for _, role in b.roles do
		if role.index == who.role and role.planned then
			planned = true
		end
	end
	local items = {
		row({ gap = "s", align = "center" }, {
			label(
				if planned then string.format(t("core:work.plan_title", "Auto is running %s's work"), who.name) else string.format(t("core:work.plan_title_fixed", "%s's work"), who.name),
				{ size = "heading", weight = "strong" }
			),
			{ kind = "spacer" },
			button({
				id = "core:work.show_board",
				label = t("core:work.show_board", "Show the board"),
				on_click = function()
					ui.set_state("core:work.show_board", true)
				end,
			}),
		}),
		label(
			if planned
				then string.format(t("core:work.plan_lead", "%s does everything, so the plan follows what's waiting and what the colony needs. Click any work to set it yourself."), who.name)
				else t("core:work.plan_lead_fixed", "Click any work to move it a level."),
			{ size = "small", color = "muted", wrap = true }
		),
	}
	for shelf = 1, b.levels + 1 do
		local level = if shelf > b.levels then 0 else shelf
		local lines = {}
		for c, k in b.cols do
			local x = who.cells[c]
			if x.value == level then
				local why = if x.pinned then t("core:work.plan_pinned", "you set this") else x.reason or ""
				if x.value ~= x.base then
					why = why .. (if why ~= "" then " · " else "") .. string.format(t("core:work.plan_moved", "moved from %s"), level_name(b.labels, x.base))
				end
				table.insert(
					lines,
					row({ gap = "s", align = "center" }, {
						button({
							id = "core:work.plan." .. k.id,
							label = k.label,
							strong = x.pinned,
							tooltip = x.why,
							on_click = function()
								put(who.id, k.id, x, next_level(x.base, b.levels))
							end,
						}),
						label(why, { size = "small", color = "muted" }),
					})
				)
			end
		end
		if #lines > 0 then
			local name = if level == 0 then t("core:work.never", "Never") else b.labels[level] or tostring(level)
			table.insert(
				items,
				row({ id = "core:work.plan.shelf." .. level, gap = "s", align = "start" }, {
					row({ w = 90 }, { label(name, { size = "small", color = "faint" }) }),
					col({ gap = "xs" }, lines),
				})
			)
		end
	end
	return col({ gap = "s" }, items)
end

-- The board: a row per colonist, a column per work type, painted.
local function board_lens(view, b)
	-- A castaway on Auto sees a plan; one the player put in a fixed role
	-- sees the board they set it up on.
	if #b.rows == 1 and not ui.state("core:work.show_board", false) then
		for _, role in b.roles do
			if role.index == b.rows[1].role and role.planned then
				return plan_view(b)
			end
		end
	end
	local levels, labels, cols = b.levels, b.labels, b.cols
	local rows = by_role(b)

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
				ring = if x.planned then "accent" else nil,
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
	-- Each role's name where its group starts.
	local role_col = ui.grid({
		id = "core:work.rolecol",
		rows = #rows,
		cols = 1,
		cell_w = ROLE_W,
		cell_h = CELL_H,
		gap = 2,
		cell = function(r, _c)
			local label = rows[r].role_label or ""
			local first = r == 1 or rows[r - 1].role ~= rows[r].role
			return { text = if first then label else "", color = "muted", tip = label }
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

	-- What Focus is doing to the board: its rules, and how many cells.
	local focus = focus_now(view)
	local moved, total = 0, 0
	for _, r in rows do
		for _, x in r.cells do
			total += 1
			if x.value ~= x.base then
				moved += 1
			end
		end
	end
	return col({ gap = "s" }, {
		alike_offer(b),
		focus and label(
			string.format(t("core:work.focus_line", "Focus: %s · moves %d of %d settings"), table.concat(focus, " + "), moved, total),
			{ id = "core:work.focus", size = "small", color = "accent" }
		),
		brush_bar(levels, labels),
		row({ gap = "s" }, { row({ w = ROLE_W + NAME_W + 4 }), header }),
		-- The window scrolls; the wheel over a cell nudges it instead.
		row({ gap = "s" }, { role_col, names, grid, jobs }),
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
			row({ id = "core:work.shelf." .. level, gap = "xs", align = "start" }, {
				row({ w = 90 }, { label(name, { size = "small", color = "muted" }) }),
				row({ gap = "xs", wrap = true, grow = 1 }, tokens),
			})
		)
	end
	return col({ gap = "s" }, items)
end

-- Deleting a role asks first, in place: "Delete" in the card's head, then
-- the question on its own line under it (rows don't wrap). The ask holds
-- the label too, since indices shift when a role goes. Returns the head's
-- button, or nil and the question.
local function delete_role(b, role)
	local asking = ui.state("core:work.deleting", nil)
	if not (asking and asking.index == role.index and asking.label == role.label) then
		return button({
			id = "core:work.role.delete." .. role.index,
			label = t("core:work.role_delete", "Delete"),
			on_click = function()
				ui.set_state("core:work.deleting", { index = role.index, label = role.label })
			end,
		}), nil
	end
	local home = t("core:work.role_default_name", "the default role")
	for _, r in b.roles do
		if r.default then
			home = r.label
		end
	end
	return nil, row({ gap = "xs", align = "center" }, {
		label(string.format(t("core:work.role_delete_ask", "Delete %s? Its members go to %s and keep their pins."), role.label, home), { size = "small" }),
		button({
			id = "core:work.role.delete_yes." .. role.index,
			label = t("core:work.role_delete_yes", "Delete"),
			on_click = function()
				act.delete_role(role.index)
				ui.set_state("core:work.deleting", nil)
			end,
		}),
		button({
			id = "core:work.role.delete_no." .. role.index,
			label = t("core:work.role_delete_no", "Keep"),
			on_click = function()
				ui.set_state("core:work.deleting", nil)
			end,
		}),
	})
end

-- Roles: a card per role. A fixed role's levels as shelves of tokens (a
-- click moves one a level on; italic ones are the work type's default); a
-- planned role says what its planner gave each member first. Members are
-- buttons: pick one up, then put them in another role.
local function roles_lens(_view, b)
	local carry = ui.state("core:work.carry", nil)
	local carried = nil
	for _, r in b.rows do
		if r.id == carry then
			carried = r
		end
	end
	local order = table.clone(b.roles)
	table.sort(order, function(a, z)
		if a.order ~= z.order then
			return a.order < z.order
		end
		return a.index < z.index
	end)
	local cards = {}
	for _, role in order do
		local members = {}
		for _, r in b.rows do
			if r.role == role.index then
				table.insert(members, r)
			end
		end
		local head = {
			label(role.label, { weight = "strong" }),
			label(
				if role.planned
					then t("core:work.role_planned", "planned hourly")
					elseif role.id == nil then t("core:work.role_own", "yours")
					elseif role.edited then t("core:work.role_edited", "edited")
					else "",
				{ size = "small", color = "muted" }
			),
			label(string.format(t("core:work.role_members", "%d"), #members), { size = "small", color = "faint" }),
		}
		if carried and carried.role ~= role.index then
			table.insert(
				head,
				button({
					id = "core:work.role.put." .. role.index,
					label = string.format(t("core:work.role_put", "Put %s here"), carried.name),
					on_click = function()
						act.assign_role(carried.id, role.index)
						ui.set_state("core:work.carry", nil)
					end,
				})
			)
		end
		-- The player's own roles can go; a mod's would only come back.
		local ask = nil
		if role.id == nil and not carried then
			local del
			del, ask = delete_role(b, role)
			if del then
				table.insert(head, del)
			end
		end
		local body = { row({ gap = "s", align = "center" }, head), ask }
		if role.planned then
			for _, m in members do
				local firsts = {}
				for c, k in b.cols do
					if m.cells[c].base == 1 then
						table.insert(firsts, k.label)
					end
				end
				table.insert(
					body,
					label(
						string.format(
							t("core:work.role_first", "%s: %s"),
							m.name,
							if #firsts > 0 then table.concat(firsts, ", ") .. " " .. t("core:work.first", "first") else t("core:work.nothing_first", "nothing at First")
						),
						{ size = "small", color = "muted" }
					)
				)
			end
			table.insert(body, label(t("core:work.role_planned_how", "Levels come from skill and what's waiting. Pin a cell on the board to overrule one."), { size = "small", color = "faint" }))
		else
			for shelf = 1, b.levels + 1 do
				local level = if shelf > b.levels then 0 else shelf
				local tokens = {}
				for _, k in b.cols do
					local set = role.levels[k.id]
					local at = set or k.default
					if at == level then
						table.insert(
							tokens,
							button({
								id = "core:work.role." .. role.index .. "." .. k.id,
								label = k.label,
								strong = set ~= nil,
								tooltip = if set then t("core:work.role_set", "Set by the role") else t("core:work.role_default", "The work type's default"),
								on_click = function()
									local nxt = next_level(at, b.levels)
									act.set_role_priority(role.index, k.id, if nxt == k.default then nil else nxt)
								end,
							})
						)
					end
				end
				local name = if level == 0 then t("core:work.never", "Never") else b.labels[level] or tostring(level)
				table.insert(
					body,
					row({ gap = "xs", align = "start" }, {
						row({ w = 90 }, { label(name, { size = "small", color = "muted" }) }),
						row({ gap = "xs", wrap = true, grow = 1 }, tokens),
					})
				)
			end
		end
		local chips = {}
		for _, m in members do
			local _, n = pins_of(b, m)
			table.insert(
				chips,
				button({
					id = "core:work.member." .. m.id,
					label = if n > 0 then string.format("%s •%d", m.name, n) else m.name,
					active = carry == m.id,
					tooltip = if n > 0 then string.format(t("core:work.member_pins", "%d pinned; the role doesn't move those"), n) else nil,
					on_click = function()
						ui.set_state("core:work.carry", if carry == m.id then nil else m.id)
					end,
				})
			)
		end
		table.insert(body, row({ id = "core:work.members." .. role.index, gap = "xs", align = "center", wrap = true }, chips))
		table.insert(cards, col({ id = "core:work.role." .. role.index, gap = "xs", pad = "s", border = "line", radius = "radius" }, body))
	end
	local hint = if carried
		then string.format(t("core:work.carrying", "Moving %s: pick their new role."), carried.name)
		else t("core:work.roles_hint", "Click a colonist to move them to another role.")
	return col({ gap = "s" }, { label(hint, { size = "small", color = "muted" }), table.unpack(cards) })
end

work.lens({ id = "core:work.board", label = t("core:work.lens_board", "Board"), order = 10, draw = board_lens })
work.lens({ id = "core:work.roles", label = t("core:work.lens_roles", "Roles"), order = 15, draw = roles_lens })
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
		orders_panel(view),
		row({ id = "core:work.lenses", gap = "xs", align = "center" }, tabs),
		shown.draw(view, b),
	})
end)

require("@core/ui/screens").add({ id = "core:work", label = t("core:keys.work", "Work"), key = "p", order = 10 })

-- A colonist's menu: their role, their pins, a role made from them.
menus.add("colonist", {
	id = "core:colonist.roles",
	group = "manage",
	rows = function(ctx)
		local b = view.board()
		local me = nil
		for _, r in b.rows do
			if r.id == ctx.id then
				me = r
			end
		end
		if not me then
			return {}
		end
		local rows = {}
		for _, role in b.roles do
			if role.index ~= me.role then
				table.insert(rows, {
					label = string.format(t("core:colonist.role", "Role: %s"), role.label),
					run = function()
						act.assign_role(me.id, role.index)
					end,
				})
			end
		end
		local pins, n = pins_of(b, me)
		if n > 0 then
			table.insert(rows, {
				label = t("core:colonist.hand_back", "Hand everything back"),
				run = function()
					for work in pins do
						act.clear_priority(me.id, work)
					end
				end,
			})
		end
		for _, role in b.roles do
			if role.index == me.role and role.id == nil then
				table.insert(rows, {
					label = string.format(t("core:colonist.delete_role", "Delete role: %s…"), role.label),
					run = function()
						ui.set_state("core:work.lens", "core:work.roles")
						ui.set_state("core:work.deleting", { index = role.index, label = role.label })
						ui.open("core:work")
					end,
				})
			end
		end
		table.insert(rows, {
			label = t("core:colonist.make_role", "Make a role from this colonist"),
			run = function()
				act.role_from_colonist(string.format(t("core:work.crew", "Crew %d"), #b.roles + 1), me.id)
			end,
		})
		return rows
	end,
})
-- On the HUD: a stance left on in spring is a trap, so a Focus that isn't
-- Normal, and any standing order acting, stays lit in the top bar. A click
-- opens the Work screen.
ui.extend("core:topbar.right", function(view)
	local names = focus_now(view)
	if not names then
		return nil
	end
	return button({
		id = "core:focus.hud",
		label = table.concat(names, " + "),
		active = true,
		tooltip = t("core:focus.hud_tip", "Focus: what's moving priorities now. Open Work (P) to change it."),
		on_click = function()
			ui.open("core:work")
		end,
	})
end)

-- What core's interface has shown already, kept in the save by core's
-- seen script: { hint = true, settlers = { [id] = true } }.
local function saved(view)
	return view.data("core:seen") or {}
end

-- A settler joining: which role, Auto already chosen. It asks without
-- pausing and goes by itself after a while, so ignoring it is a choice.
-- It waits while a sheet is open, since it would sit under it, and its time
-- starts when it's first seen. Once seen, the save remembers them as asked.
local SETTLER_TICKS = 2500
-- How far back a join still gets asked about, if a sheet hid it that long.
local SETTLER_WAIT = 20000

ui.define("core:settler", function(view)
	local now = view.tick()
	local answered = ui.state("core:settler.seen", {})
	local shown = ui.state("core:settler.shown", {})
	local asked = saved(view).settlers or {}
	local who = nil
	for _, ev in view.events(math.max(1, now - SETTLER_WAIT)) do
		if ev.kind == "joined" and ev.tick > 0 and not answered[ev.id] then
			-- Shown here: until its time runs out. Not yet: unless the save
			-- says it was asked before a load.
			local since = shown[ev.id]
			local open = if since then now < since + SETTLER_TICKS else not asked[ev.id]
			if open then
				who = ev
			end
		end
	end
	if not who or ui.sheet() then
		return nil
	end
	if not shown[who.id] then
		local s = table.clone(shown)
		s[who.id] = now
		ui.set_state("core:settler.shown", s)
		act.send("core:seen", { settler = who.id })
	end
	local p = view.pawn(who.id)
	if not p then
		return nil
	end
	local b = view.board()
	local mine = nil
	for _, r in b.rows do
		if r.id == who.id then
			mine = r
		end
	end
	local done = function()
		local s = table.clone(ui.state("core:settler.seen", {}))
		s[who.id] = true
		ui.set_state("core:settler.seen", s)
	end
	-- What they're good at: their two best skills.
	local skills = table.clone(p.skills)
	table.sort(skills, function(a, z)
		return a.level > z.level
	end)
	local best = {}
	for i = 1, math.min(2, #skills) do
		table.insert(best, string.format("%s %d", skills[i].label, skills[i].level))
	end
	local order = table.clone(b.roles)
	table.sort(order, function(a, z)
		return a.order < z.order
	end)
	local choices = {}
	for _, role in order do
		if role.planned or role.id ~= "core:hand" then
			table.insert(
				choices,
				button({
					id = "core:settler.role." .. role.index,
					label = if role.planned then string.format(t("core:settler.auto", "%s (recommended)"), role.label) else role.label,
					active = mine ~= nil and mine.role == role.index,
					on_click = function()
						act.assign_role(who.id, role.index)
						done()
					end,
				})
			)
		end
	end
	return kit.panel({ id = "core:settler", pad = "m", gap = "s", bg = "surface_raised", border = "line" }, {
		label(string.format(t("core:settler.joined", "%s joined the colony"), p.name), { weight = "strong" }),
		label(table.concat(best, " · "), { size = "small", color = "muted" }),
		label(t("core:settler.ask", "How should they work?"), { size = "small" }),
		-- As many roles to a line as fit a narrow panel over the map.
		row({ id = "core:settler.roles", gap = "xs", wrap = true, maxw = 340 }, choices),
	})
end)
ui.mount("float", "core:settler", { align = "start", order = 5 })

-- The first day's hint: once, for a castaway on Auto.
ui.define("core:auto_hint", function(view)
	-- Behind the Work screen it would only be in the way: that is the plan.
	-- For the first few hours only: after that the castaway is under way.
	local early = view.day() == 1 and view.tick() < view.ticks_per_day() // 8
	local done = ui.state("core:auto_hint.done", false) or saved(view).hint
	if done or not early or view.count_pawns("player") ~= 1 or ui.is_open("core:work") then
		return nil
	end
	local b = view.board()
	local who = b.rows[1]
	if not who then
		return nil
	end
	local planned = false
	for _, role in b.roles do
		if role.index == who.role and role.planned then
			planned = true
		end
	end
	if not planned then
		return nil
	end
	-- At once here, and in the save once the command lands.
	local function dismiss()
		ui.set_state("core:auto_hint.done", true)
		act.send("core:seen", { hint = true })
	end
	return kit.panel({ id = "core:auto_hint", pad = "m", gap = "s", bg = "surface_raised", border = "accent" }, {
		label(string.format(t("core:auto_hint.title", "%s works on Auto"), who.name), { weight = "strong" }),
		label(
			string.format(t("core:auto_hint.text", "Auto picks work from what's waiting and what %s is good at."), who.name),
			{ size = "small", color = "muted", wrap = true }
		),
		row({ gap = "xs" }, {
			button({
				id = "core:auto_hint.plan",
				label = t("core:auto_hint.see", "See the plan"),
				hint = "P",
				on_click = function()
					dismiss()
					ui.open("core:work")
				end,
			}),
			button({
				id = "core:auto_hint.ok",
				label = t("core:auto_hint.ok", "Got it"),
				on_click = dismiss,
			}),
		}),
	})
end)
ui.mount("float", "core:auto_hint", { align = "start", order = 4 })

return work
