-- Windows: the engine keeps where each one is, stacks them and remembers
-- the layout; core draws the chrome. A mod declares one with
--
--   ui.window("my_mod:settings", { title = "Settings", w = 420, h = 320, resizable = true }, function(view)
--       return ui.col({ ... })
--   end)
--
-- and opens it with ui.open / ui.toggle from any handler.
local kit = require("@core/ui/kit")

ui.window_chrome(kit.window)
