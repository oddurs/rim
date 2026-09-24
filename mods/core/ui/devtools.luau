-- Devtools (F12): what's on screen and which mod put it there.
-- Built as a core UI component, like everything else.

local kit = require("@core/ui/kit")
local row, col, label, button = kit.row, kit.col, kit.label, kit.button
local t = ui.t

ui.define("core:devtools", function(view)
	if not view.show_devtools() then
		return nil
	end
	local rows = { kit.heading(t("core:devtools.title", "Devtools"), t("core:devtools.note", "F12 to close")) }

	local i = view.inspect()
	if i then
		table.insert(rows, col({ id = "core:devtools.inspect", gap = "xxs" }, {
			label(if i.id ~= "" then i.id else t("core:devtools.no_id", "(no id)"), { weight = "strong", color = "accent" }),
			label(string.format(t("core:devtools.owner", "%s · owner %s"), i.kind, i.owner), { size = "small" }),
			label(string.format(t("core:devtools.rect", "x %.0f  y %.0f  w %.0f  h %.0f"), i.x, i.y, i.w, i.h), { size = "small", color = "muted" }),
			label(i.path, { size = "small", color = "faint" }),
		}))
	else
		table.insert(rows, label(t("core:devtools.point", "Point at anything to inspect it."), { size = "small", color = "muted" }))
	end

	local gallery = ui.state("core:gallery", false)
	table.insert(rows, row({ gap = "xs" }, {
		kit.toggle({ id = "core:devtools.outlines", label = t("core:devtools.outlines", "Outlines"), on = view.outlines(), on_click = function()
			act.toggle_outlines()
		end }),
		kit.toggle({ id = "core:devtools.gallery", label = t("core:devtools.gallery", "Kit gallery"), on = gallery, on_click = function()
			ui.set_state("core:gallery", not gallery)
		end }),
	}))

	local u = view.ui_stats()
	table.insert(rows, label(string.format(t("core:devtools.stats", "build %.0f µs · layout %.0f µs · paint %.0f µs · %d nodes · %d layouts"), u.build_us, u.layout_us, u.paint_us, u.nodes, u.layouts), { size = "small", color = "muted" }))

	table.insert(rows, kit.divider())
	local tree = {}
	for n, r in view.ui_tree() do
		if n > 160 then
			table.insert(tree, label(t("core:devtools.more", "…"), { size = "small", color = "faint" }))
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
		kit.heading(t("core:gallery.title", "Kit gallery"), t("core:gallery.note", "every component, every state")),
		row({ gap = "xs" }, {
			button({ label = t("core:gallery.button", "Button") }),
			button({ label = t("core:gallery.active", "Active"), active = true }),
			button({ label = t("core:gallery.disabled", "Disabled"), disabled = true }),
			button({ label = t("core:gallery.swatch", "Swatch"), swatch = "good" }),
			button({ label = t("core:gallery.tooltip", "Tooltip"), tooltip = t("core:gallery.tooltip_text", "Tooltips appear after a short pause.") }),
		}),
		row({ gap = "xs" }, {
			kit.toggle({ label = t("core:gallery.toggle_off", "Toggle off"), on = false }),
			kit.toggle({ label = t("core:gallery.toggle_on", "Toggle on"), on = true }),
		}),
		kit.tabs({ id = "core:gallery.tabs", selected = tab, tabs = { { key = "a", label = t("core:gallery.first", "First") }, { key = "b", label = t("core:gallery.second", "Second") } }, on_select = function(k)
			ui.set_state("core:gallery.tab", k)
		end }),
		col({ gap = "xs" }, {
			kit.bar({ label = t("core:gallery.full", "full"), value = 1, color = "good" }),
			kit.bar({ label = t("core:gallery.half", "half"), value = 0.5 }),
			kit.bar({ label = t("core:gallery.low", "low"), value = 0.12, color = "bad" }),
		}),
		row({ gap = "m", align = "start" }, {
			kit.menu({ items = { { label = t("core:gallery.menu_item", "Menu item") }, { label = t("core:gallery.another", "Another") }, { label = t("core:gallery.disabled", "Disabled"), disabled = true } } }),
			col({ gap = "xs" }, {
				kit.toast({ text = t("core:gallery.toast_info", "An info toast"), kind = "info" }),
				kit.toast({ text = t("core:gallery.toast_good", "Something good happened"), kind = "good" }),
				kit.toast({ text = t("core:gallery.toast_threat", "A threat approaches"), kind = "threat" }),
				kit.toast({ text = t("core:gallery.toast_bad", "Something went badly"), kind = "bad" }),
			}),
			kit.bubble({ text = t("core:gallery.bubble", "A speech bubble") }),
		}),
		kit.list({ t("core:gallery.row1", "list row one"), t("core:gallery.row2", "list row two"), t("core:gallery.row3", "list row three") }, function(item)
			return label(item, { size = "small" })
		end),
	})
end)
ui.mount("windows", "core:gallery")
