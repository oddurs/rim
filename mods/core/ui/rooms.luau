-- Room names on the plan (DESIGN.md §6c): what a room is for, how big it
-- is and how warm, set on the free cell nearest the middle of the room as
-- far as the screen shows it. A room that is walled in but too wide for its
-- roof says so in red: add a pillar.
--
-- Labels go on the anchored layer at the lowest priority, so pawn names
-- and speech win when they crowd.

local kit = require("@core/ui/kit")
local t = ui.t

ui.define("core:rooms", function(view)
	local out = {}
	for _, r in view.visible_rooms() do
		local name = r.role or t("core:rooms.room", "Room")
		local sub
		if r.open then
			sub = t("core:rooms.open", "open to the sky")
		elseif r.temperature then
			sub = string.format(t("core:rooms.size_temp", "%d cells · %s"), r.cells, r.temperature)
		else
			sub = string.format(t("core:rooms.size", "%d cells"), r.cells)
		end
		table.insert(out, {
			kind = "anchored",
			cell = { r.x, r.y },
			priority = -2,
			kit.col({ id = "core:rooms." .. r.id, align = "center", gap = "hair" }, {
				kit.label(string.upper(name), {
					size = "caption",
					tracking = "caption",
					weight = "strong",
					color = if r.open then "threat" else "text",
				}),
				kit.label(sub, { size = "caption", color = if r.open then "threat" else "muted" }),
			}),
		})
	end
	return kit.col({}, out)
end)
ui.mount("anchored", "core:rooms")
