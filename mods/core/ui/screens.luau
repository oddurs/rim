-- Screens: the sheets a player opens by name. Registering one gives it a
-- key binding (and so a place in the command palette) and a button at the
-- right of the dock, so a mod's screen sits beside Work and Stockpiles with
-- one call:
--
--     local screens = require("@core/ui/screens")
--     ui.window("my_mod:herds", { title = "Herds", w = 640, h = 600, sheet = true }, build)
--     screens.add({ id = "my_mod:herds", label = "Herds", key = "h" })
--
-- `id` is the window's; the binding has the same id. Buttons are
-- `core:screens.<id>`, in `order` (default 100; core's are 10, 20, 30),
-- then in the order they were added.

local kit = require("@core/ui/kit")

local screens = {}

local list: { any } = {}
local added = 0

function screens.add(def)
	assert(type(def) == "table" and type(def.id) == "string", "screens.add: needs the window's id")
	assert(type(def.label) == "string", `screens.add('{def.id}'): needs a label`)
	-- A screen added again (a reloaded script) keeps its place.
	local seq = nil
	for i, s in list do
		if s.id == def.id then
			seq = s.seq
			table.remove(list, i)
			break
		end
	end
	if not seq then
		added += 1
		seq = added
	end
	table.insert(list, { id = def.id, label = def.label, key = def.key, order = def.order or 100, seq = seq })
	table.sort(list, function(a, b)
		if a.order ~= b.order then
			return a.order < b.order
		end
		return a.seq < b.seq
	end)
	ui.bind(def.id, { key = def.key, label = def.label }, function()
		ui.toggle(def.id)
	end)
end

ui.extend("core:dock.right", function(view)
	local items = {}
	for _, s in list do
		table.insert(
			items,
			kit.button({
				id = "core:screens." .. s.id,
				label = s.label,
				hint = s.key and string.upper(s.key),
				active = ui.is_open(s.id),
				on_click = function()
					ui.toggle(s.id)
				end,
			})
		)
	end
	return kit.row({ gap = "s", align = "center" }, items)
end)

return screens
