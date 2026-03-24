---Configuration management for Hermes (Installation-only)
---@module hermes.config
---Only stores installation-related configuration:
---  - version: which binary version to download
---  - auto_download_binary: whether to auto-download or require manual build
---All other configuration is passed directly to the Rust binary

local M = {}

---@type table
local _config = {}

---Default configuration values
local default_config = {
  version = "latest",
  auto_download_binary = true,
}

---Setup hermes installation configuration
---Only stores version and auto_download_binary settings
---All other configuration is passed directly to Rust binary
---@param opts? table User configuration options { version?, auto_download_binary? }
function M.setup(opts)
  opts = opts or {}
  _config.version = opts.version or default_config.version
  _config.auto_download_binary = opts.auto_download_binary ~= false -- default true
end

---Get current installation configuration
---@return table Current configuration { version, auto_download_binary }
function M.get()
  return _config
end

---Get binary version setting
---@return string Binary version to use
function M.get_version()
  return _config.version or default_config.version
end

---Get auto_download_binary setting
---@return boolean Whether to auto-download binary
function M.get_auto_download()
  return _config.auto_download_binary ~= false
end

return M
