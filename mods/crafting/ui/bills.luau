-- The bills panel: a section in the thing inspector when a station is
-- selected. Every change goes through act.send to the bills script
-- (scripts/bills.luau), so it's a command like any other player input.

local kit = require("@core/ui/kit")
local row, col, label = kit.row, kit.col, kit.label

local MODES = { "count", "until", "forever" }
local MODE_LABEL = { count = "Do %d more", ["until"] = "Until %d", forever = "Forever" }

local function next_mode(mode: string): string
	local i = table.find(MODES, mode) or 0
	return MODES[i % #MODES + 1]
end

local function small(text: string, id: string, on_click)
	return kit.button({ id = id, label = text, padx = "s", pady = "xs", on_click = on_click })
end

local function bill_row(site: number, b, info, first: boolean, last: boolean, making: boolean)
	local id = "crafting:bill." .. b.id
	local function set(changes)
		changes.site, changes.bill = site, b.id
		act.send("crafting:set_bill", changes)
	end
	local mode = string.format(MODE_LABEL[b.mode] or b.mode, b.target)
	local status = if b.suspended
		then b.why or "paused"
		elseif making then "making"
		elseif b.why then b.why
		else "waiting"
	return col({ id = id, gap = "xxs" }, {
		row({ gap = "s", align = "center" }, {
			label(if info then info.label else b.recipe, { weight = "strong" }),
			{ kind = "spacer" },
			not first and small("↑", id .. ".up", function()
				act.send("crafting:move_bill", { site = site, bill = b.id, by = -1 })
			end),
			not last and small("↓", id .. ".down", function()
				act.send("crafting:move_bill", { site = site, bill = b.id, by = 1 })
			end),
			small("×", id .. ".remove", function()
				act.send("crafting:remove_bill", { site = site, bill = b.id })
			end),
		}),
		row({ gap = "xs", align = "center" }, {
			kit.button({
				id = id .. ".mode",
				label = mode,
				padx = "s",
				pady = "xs",
				tooltip = "Change how many",
				on_click = function()
					set({ mode = next_mode(b.mode) })
				end,
			}),
			b.mode ~= "forever" and small("−", id .. ".less", function()
				set({ target = b.target - 1 })
			end),
			b.mode ~= "forever" and small("+", id .. ".more", function()
				set({ target = b.target + 1 })
			end),
			kit.toggle({
				id = id .. ".suspend",
				label = "Pause",
				on = b.suspended,
				on_click = function()
					set({ suspended = not b.suspended })
				end,
			}),
		}),
		label(status, {
			id = id .. ".why",
			size = "small",
			color = if making then "good" elseif b.why and not b.suspended then "bad" else "muted",
			wrap = true,
		}),
	})
end

ui.extend("core:inspector.thing", function(view)
	local site = view.selected()
	local th = site and view.thing(site)
	local catalog = view.data("crafting:catalog")
	if not th or th.blueprint or not catalog then
		return nil
	end
	local makes = catalog.stations[th.def]
	if not makes then
		return nil
	end
	local st
	for _, s in view.data("crafting:bills") or {} do
		if s.site == site then
			st = s
		end
	end
	local rows = { kit.divider(), kit.heading("Bills") }
	local list = if st then st.bills else {}
	for i, b in list do
		table.insert(rows, bill_row(site, b, catalog.recipes[b.recipe], i == 1, i == #list, st.making ~= nil and st.making.bill == b.id))
	end
	if #list == 0 then
		table.insert(rows, label("Nothing to make yet.", { size = "small", color = "muted" }))
	end
	-- Two to a row: the inspector is narrow, and rows don't wrap.
	local add = {}
	for n, id in makes do
		local r = catalog.recipes[id]
		if n % 2 == 1 then
			table.insert(add, row({ gap = "xs" }))
		end
		table.insert(add[#add], kit.button({
			id = "crafting:add." .. id,
			label = "+ " .. r.label,
			padx = "s",
			pady = "xs",
			tooltip = r.inputs,
			on_click = function()
				act.send("crafting:add_bill", { site = site, recipe = id })
			end,
		}))
	end
	table.insert(rows, col({ gap = "xs" }, add))
	return col({ id = "crafting:bills", gap = "s" }, rows)
end)
