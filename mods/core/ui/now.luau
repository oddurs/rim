-- Now: what needs the player, down the right edge. Standing problems first
-- (core:alerts, from the alert registry), then the news (core:messages,
-- the message log, newest first). The whole log is a window away
-- (core:news), so nothing scrolls off for good when many arrive at once.

local kit = require("@core/ui/kit")
local alerts = require("@core/ui/alerts")
local menus = require("@core/ui/menus")
local row, col, label = kit.row, kit.col, kit.label
local t = ui.t

-- News lines hidden from the column (by their place in the log, counted
-- from the oldest): they stay in the full log.
local hidden: { [number]: boolean } = {}

-- ─── Context menus for alerts and news ────────────────────────────────────
menus.kind("alert", {
	resolve = function(id)
		for _, a in alerts.active(view) do
			if a.id == id then
				return a
			end
		end
		return nil
	end,
	caption = function(ctx)
		return t("core:alerts.caption", "Alert")
	end,
})
menus.add("alert", {
	id = "core:alert.go",
	label = t("core:alert.go", "Go to"),
	applies = function(ctx)
		return ctx.subject.subject ~= nil
	end,
	run = function(ctx)
		act.select(ctx.subject.subject)
		act.focus(ctx.subject.subject)
	end,
})
menus.add("alert", {
	id = "core:alert.snooze",
	label = t("core:alert.snooze", "Hide for today"),
	group = "manage",
	run = function(ctx)
		alerts.snooze(ctx.id, view.tick() + view.ticks_per_day())
	end,
})

-- A news line's id is its place in the log, counted from the oldest, so
-- it holds while newer lines arrive.
menus.kind("message", {
	resolve = function(id)
		local skip = view.message_count() - id
		return if skip >= 0 then view.messages(1, skip)[1] else nil
	end,
	caption = function(ctx)
		return t("core:news.caption", "News")
	end,
	actor = function(ctx)
		return string.format(t("core:news.when", "Day %d · %s"), ctx.subject.day, ctx.subject.clock)
	end,
})
menus.add("message", {
	id = "core:message.all",
	label = t("core:message.all", "All news…"),
	group = "manage",
	key = "n",
	primary = function()
		return true
	end,
	run = function()
		ui.open("core:news")
	end,
})
menus.add("message", {
	id = "core:message.hide",
	label = t("core:message.hide", "Hide here"),
	group = "manage",
	run = function(ctx)
		hidden[ctx.id] = true
	end,
})

local WIDTH = 300
local COMPACT_WIDTH = 240
-- Alerts past this many are counted, not listed.
local MAX_ALERTS = 8
-- News lines shown in the column; older ones are in the window.
local NEWS = 5
local COMPACT_NEWS = 3
-- A need this low is an alert, whatever the need.
local CRITICAL = 0.15

-- ─── Core's alerts, through the same registry a mod uses ───────────────────

-- Every id in a list of people: what an alert about them all names, so
-- the depth ruler marks each level one of them is on.
local function ids(list): { number }
	local out = {}
	for _, p in list do
		table.insert(out, p.id)
	end
	return out
end

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
			return { text = string.format(t("core:alerts.hurt", "%s badly hurt"), names(hurt)), subject = hurt[1].id, subjects = ids(hurt) }
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
			return { text = string.format(t("core:alerts.idle", "%s idle"), names(idle)), subject = idle[1].id, subjects = ids(idle) }
		end
		return nil
	end,
})

alerts.add({
	id = "core:no_stockpile",
	severity = "info",
	check = function(view)
		for _, z in view.zones() do
			if z.plant == nil then
				return nil
			end
		end
		return t("core:alerts.no_stockpile", "No stockpile: nothing gets hauled (Z)")
	end,
})

-- The colony isn't being saved (a full disk, the folder gone). It says so
-- until a save works again: a player trusts the game to keep the colony.
alerts.add({
	id = "core:not_saving",
	severity = "bad",
	check = function(view)
		local why = view.save_failing()
		if why then
			return string.format(t("core:alerts.not_saving", "Not being saved: %s"), why)
		end
		return nil
	end,
})

-- What a load changed (a mod updated, things dropped), through the first
-- day of play or until hidden. The F3 list keeps it after that.
local notes_until: number? = nil
alerts.add({
	id = "core:load_notes",
	severity = "info",
	check = function(view)
		local notes = view.load_notes()
		if #notes == 0 then
			return nil
		end
		notes_until = notes_until or view.tick() + view.ticks_per_day()
		if view.tick() > notes_until then
			return nil
		end
		local text = notes[1]
		if #notes > 1 then
			text = string.format(t("core:alerts.load_more", "%s, and %d more in F3"), text, #notes - 1)
		end
		return string.format(t("core:alerts.load_notes", "Loaded: %s"), text)
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
				menu = { kind = "alert", id = a.id },
				gap = "item",
				align = "center",
				radius = "radius",
				hover = if click then { bg = "surface_hover" } else nil,
				on_click = click,
			}, {
				{ kind = "row", w = 3, h = 14, bg = SWATCH[a.severity], radius = 1 },
				label(a.text, { color = if a.severity == "bad" then "bad" else nil, wrap = true, grow = 1 }),
			})
		)
	end
	return kit.panel({ w = if view.compact() then COMPACT_WIDTH else WIDTH, gap = "item" }, items)
end)
ui.mount("right", "core:alerts", { align = "start", order = 0 })

local KIND_COLOR = { good = "good", threat = "threat", bad = "bad" }

-- A message: a stripe in its kind's colour, the text, and when it happened
-- under it. Older than half a day, the text steps back to muted.
local function news_line(m, place: number)
	local old = m.age > 0.5
	return row({ id = "core:news.line." .. place, menu = { kind = "message", id = place }, gap = "item", align = "stretch" }, {
		{ kind = "row", w = 3, bg = KIND_COLOR[m.kind] or "faint", radius = 1 },
		-- minw = 0: a flex item keeps its content's width by default, and
		-- the text would never be offered a width to wrap at.
		col({ gap = "hair", grow = 1, minw = 0 }, {
			label(m.text, { color = if old then "muted" else KIND_COLOR[m.kind], wrap = true }),
			label(string.format(t("core:news.when", "Day %d · %s"), m.day, m.clock), { size = "caption", color = "faint" }),
		}),
	})
end

ui.define("core:messages", function(view)
	local lines = {}
	local total = view.message_count()
	for i, m in view.messages(if view.compact() then COMPACT_NEWS else NEWS) do
		if m.age > 1 then
			break
		end
		local place = total - i + 1
		if not hidden[place] then
			table.insert(lines, news_line(m, place))
		end
	end
	if #lines == 0 then
		return nil
	end
	-- The whole log is the section's link, not a button of its own.
	local all = row({
		id = "core:news.open",
		gap = "s",
		align = "center",
		radius = "radius",
		padx = "xs",
		hover = { bg = "surface_hover" },
		on_click = function()
			ui.toggle("core:news")
		end,
	}, {
		label(string.format(t("core:news.all", "All %d"), view.message_count()), { size = "small", color = "muted" }),
		label(t("core:news.key", "N"), { size = "caption", color = "faint" }),
	})
	return kit.panel({ w = if view.compact() then COMPACT_WIDTH else WIDTH, gap = "item" }, {
		kit.section(t("core:news.title_short", "News"), all),
		col({ gap = "item" }, lines),
	})
end)
ui.mount("right", "core:messages", { align = "start", order = 1 })

-- The whole log, newest first, as a virtual list: a colony's thousandth
-- message costs what its tenth did.
ui.window("core:news", { title = t("core:news.title", "News"), w = 560, h = 420, sheet = true }, function(view)
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

require("@core/ui/screens").add({ id = "core:news", label = t("core:keys.news", "News"), key = "n", order = 30 })
