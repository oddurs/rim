-- What the colony has, across every store (DESIGN.md §4f, Showing
-- contents): the Stores sheet, and a card on a store the cursor rests on.
-- Both read the sim's stock ledger (view.stock) and the store inspector's
-- view.store, and sort with its orders (storage.sorter).

local kit = require("@core/ui/kit")
local storage = require("@core/ui/storage")
local menus = require("@core/ui/menus")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

-- ─── The trend: a total a day, kept by the UI ──────────────────────────────
-- Client-side state: it starts when the game is opened, and nothing here can
-- reach the sim.

local DAYS = 7

-- Each day's totals by thing, oldest first, up to a week, with today's
-- recorded the first time the sheet sees it.
local function history(view, rows)
	local hist = ui.state("core:stores.history", nil) or { day = -1, days = {} }
	local day = view.day()
	if hist.day ~= day then
		local totals = {}
		for _, r in rows do
			totals[r.thing] = r.total
		end
		table.insert(hist.days, totals)
		while #hist.days > DAYS do
			table.remove(hist.days, 1)
		end
		hist.day = day
		ui.set_state("core:stores.history", hist)
	end
	return hist.days
end

-- A thing's totals over the days kept, as a row of bars: the last is today.
local function sparkline(days, thing: string, now: number)
	local values = {}
	for i, d in days do
		values[i] = if i == #days then now else (d[thing] or 0)
	end
	local most = 1
	for _, v in values do
		most = math.max(most, v)
	end
	local bars = {}
	for i, v in values do
		local h = math.max(1, math.floor(v / most * 14 + 0.5))
		table.insert(bars, { kind = "row", w = 5, h = h, bg = if i == #values then "accent" else "muted", radius = 1 })
	end
	return row({ gap = "xxs", align = "end", h = 16 }, bars)
end

-- ─── The Stores sheet ───────────────────────────────────────────────────────

ui.window("core:stores", { title = t("core:stores.title", "Stores"), w = 620, h = 560, sheet = true }, function(view)
	local rows = view.stock()
	if #rows == 0 then
		return col({ pad = "m" }, {
			label(t("core:stores.none", "The colony has nothing yet."), { color = "muted", wrap = true }),
		})
	end
	-- Grouped by category, then by name, until a column says otherwise.
	table.sort(rows, function(a, b)
		if a.category_order ~= b.category_order then
			return a.category_order < b.category_order
		end
		return a.label < b.label
	end)
	local days = history(view, rows)
	local stored, loose = 0, 0
	for _, r in rows do
		stored += r.stored
		loose += r.loose
	end
	return col({ id = "core:stores", gap = "s", pad = "m", grow = 1 }, {
		row({ gap = "l" }, {
			label(string.format(t("core:stores.stored", "%d stored"), stored), { id = "core:stores.total_stored" }),
			label(string.format(t("core:stores.loose", "%d loose"), loose), { id = "core:stores.total_loose", color = "muted" }),
		}),
		kit.table({
			id = "core:stores.table",
			rows = rows,
			row_h = 26,
			h = 460,
			columns = {
				{ label = "", w = 26, sort = false, value = function(r)
					return kit.item({ thing = r.thing, size = "s", tooltip = r.label })
				end },
				{ label = t("core:stores.col.thing", "Thing"), w = 150, key = "label" },
				{ label = t("core:stores.col.category", "Category"), w = 110, value = function(r)
					return r.category_label or t("core:store.other", "Other")
				end },
				{ label = t("core:stores.col.stored", "Stored"), w = 70, key = "stored" },
				{ label = t("core:stores.col.loose", "Loose"), w = 60, key = "loose" },
				{ label = t("core:stores.col.stores", "Stores"), w = 60, key = "stores" },
				{ label = t("core:stores.col.trend", "7 days"), w = 70, sort = false, value = function(r)
					return sparkline(days, r.thing, r.total)
				end },
			},
		}),
	})
end)

require("@core/ui/screens").add({ id = "core:stores", label = t("core:keys.stores", "Stores"), key = "k", order = 25 })

-- ─── The card on a store under the cursor ───────────────────────────────────

-- How long the cursor rests on a store before its card shows, in seconds.
local REST = 0.3
-- Tokens on a card before "+N more".
local SHOWN = 8

-- The store under the cursor, as a key and a ref.
local function hovered(view)
	local h = view.hover()
	local s = h and h.store
	if not s then
		return nil, nil
	end
	return if s.zone then "zone:" .. s.zone else "thing:" .. s.thing, s
end

ui.define("core:store.card", function(view)
	local key, ref = hovered(view)
	local rest = ui.state("core:store.card.rest", nil)
	if not key then
		if rest then
			ui.set_state("core:store.card.rest", nil)
		end
		return nil
	end
	-- The cursor arrived on this store now, or has rested here a while.
	if not rest or rest.key ~= key then
		ui.set_state("core:store.card.rest", { key = key, since = view.time() })
		return nil
	end
	if view.time() - rest.since < REST or menus.is_open() then
		return nil
	end
	local store = view.store(ref)
	if not store then
		return nil
	end
	local items = {}
	for _, r in store.contents do
		if not r.empty then
			table.insert(items, r)
		end
	end
	items = storage.sort(items, ui.state("core:store.sort", "core:category"), view)
	local tokens = {}
	for i = 1, math.min(#items, SHOWN) do
		local r = items[i]
		table.insert(tokens, kit.item({ thing = r.thing, made_of = r.made_of, count = r.count, limit = r.limit, size = "s" }))
	end
	if #items > SHOWN then
		table.insert(tokens, label(string.format(t("core:store.card.more", "+%d more"), #items - SHOWN), { size = "small", color = "muted" }))
	end
	local unit = if store.kind == "zone" then t("core:store.card.cells", "%d of %d cells") else t("core:store.card.slots", "%d of %d slots")
	return kit.panel({ id = "core:store.card", gap = "xs", pad = "s", bg = "surface_raised", border = "line", maxw = 280 }, {
		row({ gap = "s", align = "center" }, {
			label(store.name, { weight = "strong" }),
			{ kind = "spacer" },
			label(store.level_label, { size = "small", color = "muted" }),
		}),
		kit.bar({ id = "core:store.card.meter", value = store.used / math.max(1, store.capacity), color = "accent" }),
		label(string.format(unit, store.used, store.capacity), { size = "small", color = "muted" }),
		#tokens > 0 and row({ id = "core:store.card.tokens", gap = "xxs", align = "center" }, tokens)
			or label(t("core:store.holds_nothing", "Holds nothing yet."), { size = "small", color = "muted" }),
	})
end)
ui.mount("cursor", "core:store.card")
