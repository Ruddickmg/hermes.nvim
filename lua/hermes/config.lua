---Configuration management for Hermes (Installation-only)
---@module hermes.config
---Only stores installation-related configuration:
---  - version: which binary version to download
---  - auto_download_binary: whether to auto-download or require manual build
---  - log.notification.level: log level for vim.notify filtering
---All other configuration is passed directly to the Rust binary

local M = {}

---@class HermesInstallConfig
---Installation-specific configuration (subset of full HermesConfig)
---@field version? string Version to use ("latest" or specific version)
---@field auto_download_binary? boolean Whether to auto-download binary
---@field log? {notification?: {level?: number|string}} Log configuration for notifications

---@type HermesInstallConfig
local _config = {}

---Default configuration values
---@type HermesInstallConfig
local default_config = {
  version = "latest",
  auto_download_binary = true,
  log = {
    notification = {
      level = "error",  -- Default per README.md
    },
  },
}

---Setup hermes installation configuration
---Only stores version, auto_download_binary, and log.notification.level settings
---All other configuration is passed directly to Rust binary
---@param opts? HermesInstallConfig User configuration options
function M.setup(opts)
  opts = opts or {}
  _config.version = opts.version or default_config.version
  _config.auto_download_binary = opts.auto_download_binary ~= false -- default true
  
  -- Store log.notification.level for internal filtering
  if opts.log and opts.log.notification and opts.log.notification.level then
    _config.log = _config.log or {}
    _config.log.notification = _config.log.notification or {}
    _config.log.notification.level = opts.log.notification.level
  else
    -- Ensure default is set
    _config.log = {
      notification = {
        level = default_config.log.notification.level,
      },
    }
  end
end

---Get current installation configuration
---@return HermesInstallConfig Current configuration
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

---Get notification log level for vim.notify filtering
---@return number|string Log level (vim.log.levels.* or string)
function M.get_notification_level()
  if _config.log and _config.log.notification then
    return _config.log.notification.level
  end
  return default_config.log.notification.level
end

return M
