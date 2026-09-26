-- Alerts: standing problems, each a check a mod registers. Core's own go
-- through here too (mods/core/ui/now.luau), so a mod's alert sits beside
-- them, sorted by severity, with nothing drawn by the mod.
--
--     local alerts = require("@core/ui/alerts")
--     alerts.add({
--         id = "my_mod:no_salt",
--         severity = "warn",              -- "bad", "warn" or "info"
--         check = function(view)          -- nil when all is well
--             if view.count_pawns("player") > 0 and not has_salt(view) then
--                 return "No salt: meat will spoil"   -- or { text = ..., subject = id }
--             end
--         end,
--     })
--
-- Checks run at most four times a second, however often the panel is
-- rebuilt, so one may read the world freely. A check that errors shows as
-- a bad alert naming it rather than disappearing.

local alerts = {}

-- Seconds between evaluations.
local EVERY = 0.25
local RANK = { bad = 1, warn = 2, info = 3 }

local registry: { any } = {}
local active: { any } = {}
local last = -math.huge

-- Register an alert. A second `add` with the same id replaces the first, so
-- a reloaded script doesn't double up.
function alerts.add(def)
	assert(type(def) == "table" and type(def.id) == "string", "alerts.add: needs an id")
	assert(type(def.check) == "function", `alerts.add('{def.id}'): needs check = fn(view)`)
	assert(RANK[def.severity or "warn"], `alerts.add('{def.id}'): severity is bad, warn or info`)
	for i, a in registry do
		if a.id == def.id then
			registry[i] = def
			return
		end
	end
	table.insert(registry, def)
end

local function evaluate(view)
	local out = {}
	for order, a in registry do
		local ok, r = pcall(a.check, view)
		local item = nil
		if not ok then
			item = { text = `alert {a.id}: {r}`, severity = "bad" }
		elseif type(r) == "string" then
			item = { text = r }
		elseif type(r) == "table" and r.text then
			item = { text = r.text, subject = r.subject }
		end
		if item then
			item.id = a.id
			item.severity = item.severity or a.severity or "warn"
			item.on_click = a.on_click
			item.order = order
			table.insert(out, item)
		end
	end
	table.sort(out, function(x, y)
		if RANK[x.severity] ~= RANK[y.severity] then
			return RANK[x.severity] < RANK[y.severity]
		end
		return x.order < y.order
	end)
	return out
end

-- The alerts that hold now, most severe first: { id, text, severity,
-- subject?, on_click? }. Re-checked at most every EVERY seconds.
function alerts.active(view)
	local now = view.time()
	if now - last >= EVERY or now < last then
		last = now
		active = evaluate(view)
	end
	return active
end

return alerts
