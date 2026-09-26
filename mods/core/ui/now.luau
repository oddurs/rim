-- Now: what needs the player, down the right edge. Standing problems first
-- (core:alerts, from the alert registry), then the news (core:messages,
-- the message log, newest first). The whole log is a window away
-- (core:news), so nothing scrolls off for good when many arrive at once.

local kit = require("@core/ui/kit")
local alerts = require("@core/ui/alerts")
local row, label = kit.row, kit.label
local t = ui.t

local WIDTH = 300
-- Alerts past this many are counted, not listed.
local MAX_ALERTS = 8
-- News lines shown in the column; older ones are in the window.
local NEWS = 5
-- A need this low is an alert, whatever the need.
local CRITICAL = 0.15

-- ─── Core's alerts, through the same registry a mod uses ───────────────────

-- "Asa", "Asa and Bo", "Asa and 3 others".
local function names(list): string
	if #list == 1 then
		return list[1].name
	elseif #list == 2 then
		return string.format(t("core:alerts.two", "%s and %s"), list[1].name, list[2].name)
	end
	return string.format(t("core:alerts.many", "%s and %d others"), list[1].name, #list - 1)
end

alerts.add({
	id = "core:hurt",
	severity = "bad",
	check = function(view)
		local hurt = {}
		for _, p in view.people() do
			if p.health < 0.3 then
				table.insert(hurt, p)
			end
		end
		if #hurt > 0 then
			return { text = string.format(t("core:alerts.hurt", "%s badly hurt"), names(hurt)), subject = hurt[1].id }
		end
		return nil
	end,
})

-- Any need, core's or a mod's, run nearly dry.
alerts.add({
	id = "core:needs",
	severity = "warn",
	check = function(view)
		for _, p in view.colonists() do
			for _, n in p.needs do
				if n.value < CRITICAL then
					return { text = string.format(t("core:alerts.need", "%s: %s at %d%%"), p.name, n.label, math.floor(n.value * 100)), subject = p.id }
				end
			end
		end
		return nil
	end,
})

alerts.add({
	id = "core:idle",
	severity = "info",
	check = function(view)
		local idle = {}
		for _, p in view.people() do
			if p.idle and not p.drafted then
				table.insert(idle, p)
			end
		end
		if #idle > 0 then
			return { text = string.format(t("core:alerts.idle", "%s idle"), names(idle)), subject = idle[1].id }
		end
		return nil
	end,
})

alerts.add({
	id = "core:no_stockpile",
	severity = "info",
	check = function(view)
		if #view.zones() == 0 then
			return t("core:alerts.no_stockpile", "No stockpile: nothing gets hauled (Z)")
		end
		return nil
	end,
})

-- ─── The column ─────────────────────────────────────────────────────────────

local SWATCH = { bad = "bad", warn = "threat", info = "accent" }

ui.define("core:alerts", function(view)
	local list = alerts.active(view)
	if #list == 0 then
		return nil
	end
	local items = {}
	for i, a in list do
		if i > MAX_ALERTS then
			table.insert(items, label(string.format(t("core:alerts.more", "+%d more"), #list - MAX_ALERTS), { size = "small", color = "muted" }))
			break
		end
		local click = a.on_click
		if not click and a.subject then
			click = function()
				act.select(a.subject)
				act.focus(a.subject)
			end
		end
		table.insert(
			items,
			row({
				id = "core:alerts." .. a.id,
				padx = "s",
				pady = "xs",
				gap = "s",
				align = "center",
				hover = if click then { bg = "surface_hover" } else nil,
				on_click = click,
			}, {
				{ kind = "row", w = 3, h = 14, bg = SWATCH[a.severity], radius = 1 },
				label(a.text, { color = if a.severity == "bad" then "bad" else nil, wrap = true, grow = 1 }),
			})
		)
	end
	return kit.panel({ w = WIDTH, pad = "s", gap = "xxs" }, items)
end)
ui.mount("right", "core:alerts", { align = "start", order = 0 })

local KIND_COLOR = { good = "good", threat = "threat", bad = "bad" }

local function news_line(m)
	local old = m.age > 0.5
	return label(m.text, { color = if old then "muted" else KIND_COLOR[m.kind], wrap = true })
end

ui.define("core:messages", function(view)
	local items = {}
	for _, m in view.messages(NEWS) do
		if m.age > 1 then
			break
		end
		table.insert(items, news_line(m))
	end
	if #items == 0 then
		return nil
	end
	local total = view.message_count()
	table.insert(
		items,
		kit.button({
			id = "core:news.open",
			label = string.format(t("core:news.all", "All news · %d"), total),
			hint = "N",
			on_click = function()
				ui.toggle("core:news")
			end,
		})
	)
	return kit.panel({ w = WIDTH, pad = "s", gap = "xs" }, items)
end)
ui.mount("right", "core:messages", { align = "start", order = 1 })

-- The whole log, newest first, as a virtual list: a colony's thousandth
-- message costs what its tenth did.
ui.window("core:news", { title = t("core:news.title", "News"), w = 460, h = 420, resizable = true }, function(view)
	local total = view.message_count()
	if total == 0 then
		return label(t("core:news.none", "Nothing has happened yet."), { color = "muted" })
	end
	return {
		kind = "list",
		id = "core:news.list",
		count = total,
		row_h = 22,
		grow = 1,
		row = function(i)
			local m = view.messages(1, i - 1)[1]
			return row({ gap = "s", align = "center" }, {
				label(string.format(t("core:news.day", "day %d"), m.day), { size = "small", color = "faint", minw = 48 }),
				label(m.text, { color = KIND_COLOR[m.kind] }),
			})
		end,
	}
end)

ui.bind("core:news", { key = "n", label = t("core:keys.news", "All news") }, function()
	ui.toggle("core:news")
end)
