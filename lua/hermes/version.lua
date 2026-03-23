---Version management for Hermes binaries
---@module hermes.version

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
---Caches result for 1 hour to avoid rate limiting
---@return string version Latest version tag (e.g., "v0.1.0")
function M.fetch_latest()
  -- Check cache
  if _cached_latest and (os.time() - _cache_time) < CACHE_DURATION then
    return _cached_latest
  end
  
  -- Fetch from GitHub API
  vim.notify("Fetching latest Hermes version...", vim.log.levels.INFO)
  
  local cmd = {
    "curl", "-sL", "-H", "Accept: application/vnd.github.v3+json",
    "https://api.github.com/repos/Ruddickmg/hermes.nvim/releases/latest"
  }
  
  local result = vim.fn.system(cmd)
  local exit_code = vim.v.shell_error
  
  if exit_code ~= 0 then
    vim.notify(
      "Failed to fetch latest version from GitHub. Using fallback.",
      vim.log.levels.WARN
    )
    -- Return a reasonable fallback
    return "v0.1.0"
  end
  
  -- Parse JSON response (simple pattern matching)
  local tag = result:match('"tag_name":%s*"([^"]+)"')
  
  if not tag then
    vim.notify(
      "Could not parse version from GitHub response. Using fallback.",
      vim.log.levels.WARN
    )
    return "v0.1.0"
  end
  
  -- Cache the result
  _cached_latest = tag
  _cache_time = os.time()
  
  vim.notify("Latest Hermes version: " .. tag, vim.log.levels.INFO)
  
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
  return version:match("^v%d+%.%d+%.%d+$") ~= nil
end

return M
