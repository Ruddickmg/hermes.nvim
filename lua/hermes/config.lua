---Configuration management for Hermes (Installation-only)
---@module hermes.config
---Only stores installation-related configuration:
---  - download.version: which binary version to download
---  - download.auto: whether to auto-download or require manual build
---  - download.timeout: download timeout in seconds
---  - log.notification.level: log level for vim.notify filtering
---All other configuration is passed directly to the Rust binary

local M = {}

---@class HermesDownloadConfig
---Download configuration for binary management
---@field version? string Version to use ("latest" or specific version like "v0.1.0")
---@field auto? boolean Whether to auto-download binary (default: true)
---@field timeout? number Download timeout in seconds (default: 60)

---@class HermesInstallConfig
---Installation-specific configuration (subset of full HermesConfig)
---@field download? HermesDownloadConfig Download configuration
---@field log? {notification?: {level?: number|string}} Log configuration for notifications

---@type HermesInstallConfig
local _config = {}

---Default configuration values
---@type HermesInstallConfig
local default_config = {
  download = {
    version = "latest",
    auto = true,
    timeout = 60,
  },
  log = {
    notification = {
      level = "error",  -- Default per README.md
    },
  },
}

---Setup hermes installation configuration
---Only stores download config (version, auto, timeout) and log.notification.level settings
---All other configuration is passed directly to Rust binary
---@param opts? HermesInstallConfig User configuration options
function M.setup(opts)
  opts = opts or {}
  
  -- Initialize download config with defaults
  _config.download = {
    version = (opts.download and opts.download.version) or default_config.download.version,
    auto = (opts.download and opts.download.auto) ~= false, -- default true
    timeout = (opts.download and opts.download.timeout) or default_config.download.timeout,
  }
  
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

---Get download configuration
---@return HermesDownloadConfig Download configuration with version, auto, and timeout
function M.get_download()
  return _config.download or default_config.download
end

---Get binary version setting
---@return string Binary version to use
function M.get_version()
  return (_config.download and _config.download.version) or default_config.download.version
end

---Get auto download setting
---@return boolean Whether to auto-download binary
function M.get_auto_download()
  return (_config.download and _config.download.auto) ~= false
end

---Get download timeout
---@return number Download timeout in seconds
function M.get_download_timeout()
  return (_config.download and _config.download.timeout) or default_config.download.timeout
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
