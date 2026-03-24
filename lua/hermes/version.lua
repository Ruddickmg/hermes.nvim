---Version management for Hermes binaries
---@module hermes.version

local logging = require("hermes.logging")

local M = {}

---@type string|nil Cached latest version
local _cached_latest = nil

---@type number Cache timestamp
local _cache_time = 0

---Cache duration in seconds (1 hour)
local CACHE_DURATION = 3600

---Get the version to use
---Returns either user-specified version or fetches latest from GitHub
---@return string version Version string (e.g., "v0.1.0" or "latest")
function M.get_wanted()
	local config = require("hermes.config").get()
	local wanted = config.version or "latest"

	if wanted == "latest" then
		return M.fetch_latest()
	end

	-- Ensure version starts with 'v'
	if not wanted:match("^v") then
		wanted = "v" .. wanted
	end

	return wanted
end

---Fetch latest release version from GitHub
---Uses download module for cross-platform HTTP support
---Caches result for 1 hour to avoid rate limiting
---@return string version Latest version tag (e.g., "v0.1.0")
function M.fetch_latest()
	-- Check cache
	if _cached_latest and (os.time() - _cache_time) < CACHE_DURATION then
		return _cached_latest
	end

	-- Fetch from GitHub API
	logging.notify("Fetching latest Hermes version...", vim.log.levels.INFO)

	-- Use download module for cross-platform HTTP support
	local download = require("hermes.download")
	local url = "https://api.github.com/repos/Ruddickmg/hermes.nvim/releases/latest"
	
	-- Create a temporary file for the response
	local temp_file = os.tmpname()
	
	local success, err = download.download(url, temp_file)
	
	if not success then
		logging.notify("Failed to fetch latest version from GitHub: " .. (err or "Unknown error") .. ". Using fallback.", vim.log.levels.WARN)
		return "v0.1.0"
	end

	-- Read the response
	local f = io.open(temp_file, "r")
	if not f then
		logging.notify("Could not read version response. Using fallback.", vim.log.levels.WARN)
		return "v0.1.0"
	end
	
	local result = f:read("*all")
	f:close()
	
	-- Clean up temp file
	os.remove(temp_file)

	-- Parse JSON response (simple pattern matching)
	local tag = result:match('"tag_name":%s*"([^"]+)"')

	if not tag then
		logging.notify("Could not parse version from GitHub response. Using fallback.", vim.log.levels.WARN)
		return "v0.1.0"
	end

	-- Cache the result
	_cached_latest = tag
	_cache_time = os.time()

	logging.notify("Latest Hermes version: " .. tag, vim.log.levels.INFO)

	return tag
end

---Clear version cache
---Forces re-fetch on next get_wanted() call
function M.clear_cache()
	_cached_latest = nil
	_cache_time = 0
end

---Get cache status
---@return table status Cache status information
function M.get_cache_status()
	return {
		cached = _cached_latest ~= nil,
		version = _cached_latest,
		age = os.time() - _cache_time,
		valid = _cached_latest and (os.time() - _cache_time) < CACHE_DURATION,
	}
end

---Validate version string
---@param version string Version string to validate
---@return boolean valid Whether version is valid
function M.validate(version)
	if version == "latest" then
		return true
	end
	-- Must match vX.Y.Z pattern
	return version:match("^v%d+%.%d+%.%d+%$") ~= nil
end

return M
