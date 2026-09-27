-- The store inspector (DESIGN.md §4f, Showing contents): two tabs on any
-- selected store, a stockpile or a container. Contents shows what it holds
-- as item tokens: a container's slots, a stockpile's totals by thing and
-- material. Accepts shows what it takes: the category tree, materials and
-- condition. This file returns the sort-order registry:
--
--     local storage = require("@core/ui/storage")
--     storage.sorter({
--         id = "my_mod:weight", label = "Weight", order = 50,
--         key = function(row, view) return -(row.count * 2) end,
--     })
--
-- A key is a number or a string; rows sort by it, then by label. `row` is
-- one of view.store's contents (thing, label, made_of, count, value,
-- category_order...). Orders run in the UI's own VM, so none can change
-- the game.

local kit = require("@core/ui/kit")
local inspector = require("@core/ui/inspector")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

local storage = {}

-- ─── Sort orders ────────────────────────────────────────────────────────────

local sorters: { any } = {}

function storage.sorter(def)
	assert(type(def) == "table" and type(def.id) == "string", "storage.sorter: needs an id")
	assert(type(def.key) == "function", `storage.sorter('{def.id}'): needs key = fn(row, view)`)
	for i, d in sorters do
		if d.id == def.id then
			sorters[i] = def
			return
		end
	end
	table.insert(sorters, def)
	table.sort(sorters, function(a, b)
		return (a.order or 100) < (b.order or 100)
	end)
end

-- Every sort order, in order.
function storage.sorters()
	return sorters
end

-- Rows sorted by the order `id` (the first order if it's gone), then label.
function storage.sort(rows, id: string, view)
	local by = sorters[1]
	for _, s in sorters do
		if s.id == id then
			by = s
		end
	end
	local keyed = {}
	for i, r in rows do
		table.insert(keyed, { r = r, k = by and by.key(r, view) or 0, i = i })
	end
	table.sort(keyed, function(a, b)
		if a.k ~= b.k then
			if type(a.k) == type(b.k) then
				return a.k < b.k
			end
			return type(a.k) == "number"
		end
		if (a.r.label or "") ~= (b.r.label or "") then
			return (a.r.label or "") < (b.r.label or "")
		end
		return a.i < b.i
	end)
	local out = {}
	for _, k in keyed do
		table.insert(out, k.r)
	end
	return out
end

storage.sorter({
	id = "core:category",
	order = 10,
	label = t("core:store.sort.category", "Category"),
	key = function(r)
		return string.format("%09d %s", (r.category_order or 0) + 100000000, r.category_label or "")
	end,
})
storage.sorter({ id = "core:count", order = 20, label = t("core:store.sort.count", "Count"), key = function(r)
	return -(r.count or 0)
end })
storage.sorter({ id = "core:value", order = 30, label = t("core:store.sort.value", "Value"), key = function(r)
	return -(r.value or 0)
end })
storage.sorter({ id = "core:name", order = 40, label = t("core:store.sort.name", "Name"), key = function(r)
	return r.label or ""
end })

-- ─── Stores as the inspector sees them ──────────────────────────────────────

-- The store a selection is, as view.store and act.store_filter take it.
function storage.ref(sel)
	if sel.kind == "zone" then
		return { zone = sel.id }
	end
	return { thing = sel.id }
end

local function is_store(sel): boolean
	return sel.kind == "zone" or (sel.kind == "thing" and sel.store == true)
end

-- On, off or some of: the things under a category a store could take.
local function tri(filter, ids): string
	local on, off = 0, 0
	for _, id in ids do
		if filter.can[id] then
			if filter.allows[id] then
				on += 1
			else
				off += 1
			end
		end
	end
	if on > 0 and off == 0 then
		return "on"
	elseif on == 0 then
		return "off"
	end
	return "some"
end

-- What a store takes, in a line: whole categories first, then single
-- things, then refused materials and the condition; three clauses at most.
function storage.summary(store, cats): string
	local f = store.filter
	local parts = {}
	local covered = {}
	for _, root in cats.roots do
		local c = cats.by_id[root]
		local state = tri(f, c.under)
		if state == "on" then
			table.insert(parts, c.label)
			for _, id in c.under do
				covered[id] = true
			end
		end
	end
	local all, any = true, false
	for id in f.can do
		if f.allows[id] then
			any = true
		else
			all = false
		end
	end
	if not any then
		return t("core:store.takes_nothing", "Takes nothing")
	end
	if all then
		parts = { t("core:store.takes_everything", "Everything") }
	else
		local singles = {}
		for id in f.allows do
			if f.can[id] and not covered[id] then
				table.insert(singles, id)
			end
		end
		table.sort(singles)
		for _, id in singles do
			table.insert(parts, (string.gsub(id, "^[^:]*:", "")))
		end
	end
	local refused = {}
	for _, m in store.materials do
		if m.refused then
			table.insert(refused, m.label)
		end
	end
	if #refused > 0 then
		table.insert(parts, string.format(t("core:store.not", "not %s"), table.concat(refused, ", ")))
	end
	if f.min > 0 or f.max < 100 then
		table.insert(parts, if f.max >= 100 then string.format("%d%%+", f.min) else string.format("%d–%d%%", f.min, f.max))
	end
	if #parts > 3 then
		local more = #parts - 3
		parts = { parts[1], parts[2], parts[3], string.format(t("core:store.more", "+%d more"), more) }
	end
	return table.concat(parts, " · ")
end

-- ─── Contents ───────────────────────────────────────────────────────────────

local COLS = 7

-- The tokens, grouped under category captions or not, as one grid a group.
local function token_grid(id: string, rows)
	return kit.grid({
		id = id,
		rows = math.max(1, math.ceil(#rows / COLS)),
		cols = COLS,
		cell_w = 34,
		cell_h = 34,
		gap = 4,
		cell = function(r, c)
			local it = rows[(r - 1) * COLS + c]
			if not it then
				return nil
			end
			if it.empty then
				return { token = kit.item_token({ state = "empty" }), tip = t("core:store.empty_slot", "Empty slot") }
			end
			local tip = string.format("%s%s × %d", it.label, if it.made_of_label then " (" .. it.made_of_label .. ")" else "", it.count)
			return {
				token = kit.item_token({ thing = it.thing, made_of = it.made_of, count = it.count, limit = it.limit, hp = if it.hp < 1 then it.hp else nil }),
				tip = tip,
			}
		end,
	})
end

inspector.tab({
	id = "contents",
	order = 10,
	label = t("core:store.tab.contents", "Contents"),
	applies = is_store,
	build = function(view, sel)
		local store = view.store(storage.ref(sel))
		if not store then
			return nil
		end
		local zone = store.kind == "zone"
		local items, empties = {}, 0
		for _, r in store.contents do
			if r.empty then
				empties += 1
			else
				table.insert(items, r)
			end
		end
		local order = ui.state("core:store.sort", "core:category")
		local grouped = ui.state("core:store.group", true)
		items = storage.sort(items, order, view)
		-- How full: slots for a container (they're its limit), cells for a
		-- stockpile.
		local meter
		if zone then
			meter = kit.bar({ id = "core:store.meter", value = store.used / math.max(1, store.capacity), color = "accent" })
		else
			local segs = {}
			for i, r in store.contents do
				local frac = if r.empty then 0 else math.clamp(r.count / math.max(1, r.limit), 0.05, 1)
				table.insert(segs, {
					kind = "row",
					grow = 1,
					h = 6,
					bg = "track",
					radius = 1,
					frac > 0 and { kind = "row", w = (math.floor(frac * 1000) / 10) .. "%", h = "fill", bg = "accent", radius = 1 },
				})
			end
			meter = row({ id = "core:store.meter", gap = "xxs" }, segs)
		end
		local fill = if zone
			then string.format(t("core:store.cells", "%d of %d cells"), store.used, store.capacity)
			else string.format(t("core:store.slots", "%d of %d slots"), store.used, store.capacity)
		local names = {}
		for _, s in storage.sorters() do
			if s.id == order then
				names.current = s.label
			end
		end
		local body = {}
		if grouped then
			local groups, seen = {}, {}
			for _, r in items do
				local g = r.category_label or t("core:store.other", "Other")
				if not seen[g] then
					seen[g] = {}
					table.insert(groups, g)
				end
				table.insert(seen[g], r)
			end
			for i, g in groups do
				table.insert(body, col({ gap = "xxs" }, { kit.caption(g), token_grid("core:store.grid." .. i, seen[g]) }))
			end
		elseif #items > 0 then
			table.insert(body, token_grid("core:store.grid.all", items))
		end
		if empties > 0 then
			local e = {}
			for _ = 1, empties do
				table.insert(e, { empty = true })
			end
			table.insert(body, token_grid("core:store.grid.empty", e))
		end
		if #items == 0 and empties == 0 then
			table.insert(body, label(t("core:store.holds_nothing", "Holds nothing yet."), { size = "small", color = "muted" }))
		end
		return col({ id = "core:store.contents", gap = "s" }, {
			col({ gap = "xxs" }, {
				row({ gap = "s" }, {
					label(string.format(t("core:store.priority", "%s priority"), store.level_label), { id = "core:store.level", size = "small" }),
					label(fill, { size = "small", color = "muted" }),
					{ kind = "spacer" },
					store.incoming > 0 and label(string.format(t("core:store.incoming", "%d on the way"), store.incoming), { size = "small", color = "muted" }),
				}),
				meter,
			}),
			row({ gap = "xs" }, {
				kit.button({
					id = "core:store.sort",
					label = string.format(t("core:store.sort", "Sort: %s"), names.current or order),
					on_click = function()
						local list = storage.sorters()
						local next = list[1]
						for i, s in list do
							if s.id == order then
								next = list[i % #list + 1]
							end
						end
						ui.set_state("core:store.sort", next.id)
					end,
				}),
				kit.toggle({
					id = "core:store.group",
					label = t("core:store.group", "Group"),
					on = grouped,
					on_click = function()
						ui.set_state("core:store.group", not grouped)
					end,
				}),
			}),
			col({ gap = "s" }, body),
		})
	end,
})

-- ─── Accepts ────────────────────────────────────────────────────────────────

local SWATCH = { on = "accent", some = "muted", off = "faint" }

-- A category's row: its tri-state box as a button, and its children and
-- its own items when opened.
local function category_rows(out, cats, c, f, ref, depth: number)
	local state = tri(f, c.under)
	local can_any = false
	for _, id in c.under do
		if f.can[id] then
			can_any = true
		end
	end
	if not can_any then
		return
	end
	local open_key = "core:store.open." .. c.id
	local open = ui.state(open_key, false)
	local has_more = #c.children > 0 or #c.items > 0
	table.insert(
		out,
		row({ id = "core:store.cat." .. c.id, gap = "xs", align = "center" }, {
			{ kind = "row", w = depth * 14 },
			kit.button({
				id = "core:store.open." .. c.id,
				label = if not has_more then " " elseif open then "▾" else "▸",
				padx = "s",
				disabled = not has_more,
				on_click = function()
					ui.set_state(open_key, not open)
				end,
			}),
			kit.button({
				id = "core:store.take." .. c.id,
				label = c.label,
				swatch = SWATCH[state],
				active = state == "on",
				on_click = function()
					act.store_filter(ref, { category = c.id, on = state ~= "on" })
				end,
			}),
		})
	)
	if not open then
		return
	end
	for _, k in c.children do
		category_rows(out, cats, cats.by_id[k], f, ref, depth + 1)
	end
	for _, id in c.items do
		if f.can[id] then
			local on = f.allows[id] == true
			table.insert(
				out,
				row({ gap = "xs" }, {
					{ kind = "row", w = (depth + 1) * 14 + 22 },
					kit.toggle({
						id = "core:store.thing." .. id,
						label = (string.gsub(id, "^[^:]*:", "")),
						on = on,
						on_click = function()
							act.store_filter(ref, { thing = id, on = not on })
						end,
					}),
				})
			)
		end
	end
end

-- Rows of a few, since a row doesn't wrap.
local function in_rows(items, per: number)
	local out = {}
	for i = 1, #items, per do
		table.insert(out, row({ gap = "xs" }, { table.unpack(items, i, math.min(i + per - 1, #items)) }))
	end
	return out
end

local CONDITIONS = { { 0, "Any" }, { 25, "25%+" }, { 50, "50%+" }, { 75, "75%+" } }

inspector.tab({
	id = "accepts",
	order = 20,
	label = t("core:store.tab.accepts", "Accepts"),
	applies = is_store,
	build = function(view, sel)
		local ref = storage.ref(sel)
		local store = view.store(ref)
		if not store then
			return nil
		end
		local cats = view.item_categories()
		local f = store.filter
		local tree = {}
		for _, root in cats.roots do
			category_rows(tree, cats, cats.by_id[root], f, ref, 0)
		end
		local chips = {}
		for _, m in store.materials do
			table.insert(
				chips,
				kit.toggle({
					id = "core:store.material." .. m.id,
					label = m.label,
					on = not m.refused,
					on_click = function()
						act.store_filter(ref, { material = m.id, on = m.refused })
					end,
				})
			)
		end
		local conds = {}
		for _, c in CONDITIONS do
			table.insert(
				conds,
				kit.button({
					id = "core:store.condition." .. c[1],
					label = t("core:store.condition." .. c[1], c[2]),
					active = f.min == c[1] and f.max == 100,
					on_click = function()
						act.store_filter(ref, { min = c[1], max = 100 })
					end,
				})
			)
		end
		return col({ id = "core:store.accepts", gap = "s" }, {
			label(storage.summary(store, cats), { id = "core:store.summary", size = "small", wrap = true }),
			row({ gap = "xs" }, {
				kit.button({ id = "core:store.all", label = t("core:store.all", "Everything"), on_click = function()
					act.store_filter(ref, { all = true })
				end }),
				kit.button({ id = "core:store.none", label = t("core:store.none", "Nothing"), on_click = function()
					act.store_filter(ref, { all = false })
				end }),
			}),
			col({ gap = "xxs" }, tree),
			kit.caption(t("core:store.made_of", "Made of")),
			col({ gap = "xs" }, in_rows(chips, 3)),
			kit.caption(t("core:store.condition", "Condition")),
			row({ gap = "xs" }, conds),
		})
	end,
})

-- ─── Actions: level, copy and paste ─────────────────────────────────────────

local function step(by: number)
	return function(sel)
		local store = view.store(storage.ref(sel))
		if store then
			act.store_level(storage.ref(sel), math.clamp(store.level + by, 0, #store.levels - 1))
		end
	end
end

inspector.action({
	id = "core:store.lower",
	order = 40,
	key = ",",
	bind_label = t("core:keys.store_lower", "Lower the selected store's priority"),
	label = t("core:store.lower", "Lower"),
	applies = is_store,
	run = step(-1),
})

inspector.action({
	id = "core:store.raise",
	order = 41,
	key = ".",
	bind_label = t("core:keys.store_raise", "Raise the selected store's priority"),
	label = t("core:store.raise", "Raise"),
	applies = is_store,
	run = step(1),
})

-- The copied filter lives in UI state; pasting sends it as edits, so it
-- replays like any other change.
inspector.action({
	id = "core:store.copy",
	order = 42,
	key = "ctrl+c",
	bind_label = t("core:keys.store_copy", "Copy what the selected store takes"),
	label = t("core:store.copy", "Copy"),
	applies = is_store,
	run = function(sel)
		local store = view.store(storage.ref(sel))
		if not store then
			return
		end
		local allows, refuses = {}, {}
		for id in store.filter.allows do
			table.insert(allows, id)
		end
		for _, m in store.materials do
			if m.refused then
				table.insert(refuses, m.id)
			end
		end
		table.sort(allows)
		ui.set_state("core:store.clipboard", { allows = allows, refuses = refuses, min = store.filter.min, max = store.filter.max })
	end,
})

inspector.action({
	id = "core:store.paste",
	order = 43,
	key = "ctrl+v",
	bind_label = t("core:keys.store_paste", "Paste a copied filter onto the selected store"),
	label = t("core:store.paste", "Paste"),
	applies = function(sel)
		return is_store(sel) and ui.state("core:store.clipboard", nil) ~= nil
	end,
	run = function(sel)
		local clip = ui.state("core:store.clipboard", nil)
		local ref = storage.ref(sel)
		local store = view.store(ref)
		if not clip or not store then
			return
		end
		act.store_filter(ref, { all = false })
		for _, id in clip.allows do
			if store.filter.can[id] then
				act.store_filter(ref, { thing = id, on = true })
			end
		end
		local refuse = {}
		for _, id in clip.refuses do
			refuse[id] = true
		end
		for _, m in store.materials do
			if m.refused ~= (refuse[m.id] == true) then
				act.store_filter(ref, { material = m.id, on = not refuse[m.id] })
			end
		end
		act.store_filter(ref, { min = clip.min, max = clip.max })
	end,
})

return storage
