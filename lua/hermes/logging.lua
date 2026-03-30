-- luacov: disable
---Logging utilities with level filtering for internal use
---@module hermes.logging
---Provides vim.notify wrapper that respects config.log.notification.level
-- luacov: enable

local M = {}

---Normalize log level to numeric value for comparison
---Supports both vim.log.levels constants and string values
-- luacov: disable
---@param level number|string Log level
---@return number numeric_level Normalized numeric level (0-5)
-- luacov: enable
local function normalize_level(level)
	if type(level) == "number" then
		return level
	end

	-- String level mapping (case-insensitive)
	local levels = {
		trace = 0,
		debug = 1,
		info = 2,
		warn = 3,
		warning = 3,
		error = 4,
		off = 5,
	}

	return levels[level:lower()] or 4 -- Default to ERROR if unknown
end

---Internal notify wrapper that filters based on configured log level
-- luacov: disable
---Only messages with level >= configured minimum level are shown
---@param message string Message to display
---@param level? number|string Log level (vim.log.levels.* or string), defaults to ERROR
---@param opts? table Additional options for vim.notify
---@private
-- luacov: enable
function M.notify(message, level, opts)
	level = level or vim.log.levels.ERROR
	opts = opts or { title = "Hermes" }

	local config = require("hermes.config")
	local configured_level = config.get_notification_level()

	-- Convert both to numeric values for comparison
	local message_level = normalize_level(level)
	local min_level = normalize_level(configured_level)

	-- Only show if message level is >= configured minimum level
	-- Higher numbers = more severe (ERROR=4, WARN=3, INFO=2, DEBUG=1, TRACE=0)
	if message_level >= min_level then
		local notify_level = level
		if type(notify_level) == "string" then
			notify_level = normalize_level(notify_level)
		end
		vim.notify(message, notify_level, opts)
	end
end

return M
