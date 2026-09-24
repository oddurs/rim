-- Tier 2, UI: a mod adding to core's HUD without touching core's files.
-- `ui.extend` puts a readout into the top bar's extension point; players
-- see how much wildlife is around, which is what this mod is about.

local kit = require("@core/ui/kit")

ui.extend("core:topbar.right", function(view)
	return kit.label("wildlife " .. view.count_pawns("wild"), {
		id = "wildlife_plus:counter",
		size = "small",
		color = "good",
	})
end)
