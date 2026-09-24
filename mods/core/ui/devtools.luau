-- Devtools (F12): what's on screen and which mod put it there.
-- Built as a core UI component, like everything else.

local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button

ui.define("core:devtools", function(view)
	if not view.show_devtools() then
		return nil
	end
	local rows = { kit.heading("Devtools", "F12 to close") }

	local i = view.inspect()
	if i then
		table.insert(rows, col({ id = "core:devtools.inspect", gap = "xxs" }, {
			label(if i.id ~= "" then i.id else "(no id)", { weight = "strong", color = "accent" }),
			label(string.format("%s · owner %s", i.kind, i.owner), { size = "small" }),
			label(string.format("x %.0f  y %.0f  w %.0f  h %.0f", i.x, i.y, i.w, i.h), { size = "small", color = "muted" }),
			label(i.path, { size = "small", color = "faint" }),
		}))
	else
		table.insert(rows, label("Point at anything to inspect it.", { size = "small", color = "muted" }))
	end

	local gallery = ui.state("core:gallery", false)
	table.insert(rows, row({ gap = "xs" }, {
		kit.toggle({ id = "core:devtools.outlines", label = "Outlines", on = view.outlines(), on_click = function()
			act.toggle_outlines()
		end }),
		kit.toggle({ id = "core:devtools.gallery", label = "Kit gallery", on = gallery, on_click = function()
			ui.set_state("core:gallery", not gallery)
		end }),
	}))

	local u = view.ui_stats()
	table.insert(rows, label(string.format("build %.0f µs · layout %.0f µs · paint %.0f µs · %d nodes · %d layouts", u.build_us, u.layout_us, u.paint_us, u.nodes, u.layouts), { size = "small", color = "muted" }))

	table.insert(rows, kit.divider())
	local tree = {}
	for n, r in view.ui_tree() do
		if n > 160 then
			table.insert(tree, label("…", { size = "small", color = "faint" }))
			break
		end
		local text = string.rep("  ", r.depth) .. r.kind .. (if r.id ~= "" then " #" .. r.id else "")
		table.insert(tree, row({ gap = "s" }, {
			label(text, { size = "small", color = if r.id ~= "" then "text" else "muted" }),
			{ kind = "spacer" },
			label(r.owner, { size = "small", color = "faint" }),
		}))
	end
	table.insert(rows, { kind = "scroll", h = 320, gap = 0, table.unpack(tree) })
	return kit.panel({ id = "core:devtools.panel", w = 400, gap = "s" }, rows)
end)
ui.mount("right", "core:devtools", { align = "start", order = -10 })

-- Every kit component in every state, for review.
ui.define("core:gallery", function(view)
	if not (view.show_devtools() and ui.state("core:gallery", false)) then
		return nil
	end
	local tab = ui.state("core:gallery.tab", "a")
	return kit.panel({ id = "core:gallery.panel", w = 520, gap = "m", bg = "surface_raised" }, {
		kit.heading("Kit gallery", "every component, every state"),
		row({ gap = "xs" }, {
			button({ label = "Button" }),
			button({ label = "Active", active = true }),
			button({ label = "Disabled", disabled = true }),
			button({ label = "Swatch", swatch = "good" }),
			button({ label = "Tooltip", tooltip = "Tooltips appear after a short pause." }),
		}),
		row({ gap = "xs" }, {
			kit.toggle({ label = "Toggle off", on = false }),
			kit.toggle({ label = "Toggle on", on = true }),
		}),
		kit.tabs({ id = "core:gallery.tabs", selected = tab, tabs = { { key = "a", label = "First" }, { key = "b", label = "Second" } }, on_select = function(k)
			ui.set_state("core:gallery.tab", k)
		end }),
		col({ gap = "xs" }, {
			kit.bar({ label = "full", value = 1, color = "good" }),
			kit.bar({ label = "half", value = 0.5 }),
			kit.bar({ label = "low", value = 0.12, color = "bad" }),
		}),
		row({ gap = "m", align = "start" }, {
			kit.menu({ items = { { label = "Menu item" }, { label = "Another" }, { label = "Disabled", disabled = true } } }),
			col({ gap = "xs" }, {
				kit.toast({ text = "An info toast", kind = "info" }),
				kit.toast({ text = "Something good happened", kind = "good" }),
				kit.toast({ text = "A threat approaches", kind = "threat" }),
				kit.toast({ text = "Something went badly", kind = "bad" }),
			}),
			kit.bubble({ text = "A speech bubble" }),
		}),
		kit.list({ "list row one", "list row two", "list row three" }, function(item)
			return label(item, { size = "small" })
		end),
	})
end)
ui.mount("windows", "core:gallery")
