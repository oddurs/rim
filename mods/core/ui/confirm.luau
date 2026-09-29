-- The confirm dialog, for what can't be undone: one question at a time,
-- over everything. Its title is the question, its buttons say the verb,
-- and the safe answer has focus, so Enter and Esc never do the harm.
--
--   kit.confirm({ title = "Delete Crew?", body = "Its members go to Auto.",
--       verb = "Delete", danger = true, on_confirm = function() … end })

local kit = require("@core/ui/kit")
local t = ui.t

local confirm = {}

local asking: any = nil

-- Ask. A question already up is answered no first: a new one replaces it.
function confirm.ask(props)
	if asking and asking.on_cancel then
		asking.on_cancel()
	end
	asking = props
	ui.focus("core:confirm.cancel")
end

local function answer(yes: boolean)
	local a = asking
	asking = nil
	if not a then
		return
	end
	local f = if yes then a.on_confirm else a.on_cancel
	if f then
		f()
	end
end

ui.define("core:confirm", function(view)
	local a = asking
	if not a then
		return nil
	end
	return kit.panel({
		id = "core:confirm",
		w = 380,
		pad = "xl",
		gap = "l",
		bg = "surface_raised",
		border = "line_strong",
		-- Enter is the engine's: it presses what has focus, the safe answer
		-- unless the player tabbed to the other.
		on_key = function(key: string)
			if key == "escape" then
				answer(false)
			end
		end,
		on_outside = function()
			answer(false)
		end,
	}, {
		kit.label(a.title, { size = "heading", weight = "strong", wrap = true }),
		a.body and kit.label(a.body, { color = "muted", wrap = true }),
		kit.row({ gap = "s" }, {
			{ kind = "spacer" },
			kit.button({
				id = "core:confirm.cancel",
				label = a.cancel or t("core:confirm.cancel", "Cancel"),
				on_click = function()
					answer(false)
				end,
			}),
			kit.button({
				id = "core:confirm.ok",
				label = a.verb,
				kind = if a.danger then "danger" else "primary",
				on_click = function()
					answer(true)
				end,
			}),
		}),
	})
end)
ui.mount("modal", "core:confirm")

return confirm
