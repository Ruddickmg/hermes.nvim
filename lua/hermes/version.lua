-- luacov: disable
---Version management for Hermes binaries
---@module hermes.version
-- luacov: enable

local M = {}

-- luacov: disable
---Get the version to use
---Returns either user-specified version or "latest" as default
---@return string version Version string (e.g., "v0.1.0" or "latest")
---@private
-- luacov: enable
function M.get_wanted()
	local config = require("hermes.config")
	local wanted = config.get_version()

	-- Special case: "source" means build from local source
	if wanted == "source" then
		return "source"
	end

	if wanted == "latest" then
		return "latest"
	end

	-- Ensure version starts with 'v'
	if not wanted:match("^v") then
		wanted = "v" .. wanted
	end

	return wanted
end

-- luacov: disable
---Fetch latest release version from GitHub
---@return string version Latest version tag (e.g., "v0.1.0")
---@private
-- luacov: enable
function M.fetch_latest()
	local logging = require("hermes.logging")
	local fallback_version = "v0.1.0"

	logging.notify("Fetching latest Hermes version from GitHub...", vim.log.levels.INFO)

	-- Use download module for cross-platform HTTP support
	local download = require("hermes.download")
	local url = "https://api.github.com/repos/Ruddickmg/hermes.nvim/releases/latest"
	
	-- Create a temporary file for the response
	local temp_file = os.tmpname()
	
	local success, err = download.download(url, temp_file)
	
	if not success then
		-- Handle both structured error tables and plain strings
		local err_msg = err
		if type(err) == "table" and err.message then
			err_msg = err.message
		end
		logging.notify("Failed to fetch latest version: " .. (err_msg or "Unknown error") .. ". Using fallback.", vim.log.levels.WARN)
		os.remove(temp_file)
		return fallback_version
	end

	-- Read the response
	local f = io.open(temp_file, "r")
	if not f then
		logging.notify("Could not read version response. Using fallback.", vim.log.levels.WARN)
		os.remove(temp_file)
		return fallback_version
	end
	
	local result = f:read("*all")
	f:close()
	
	-- Clean up temp file
	os.remove(temp_file)

	-- Parse JSON response (simple pattern matching)
	local tag = result:match('"tag_name":%s*"([^"]+)"')

	if not tag then
		logging.notify("Could not parse version from response. Using fallback.", vim.log.levels.WARN)
		return fallback_version
	end

	logging.notify("Latest version: " .. tag, vim.log.levels.INFO)

	return tag
end

return M
