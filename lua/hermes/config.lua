---Configuration management for Hermes
---@module hermes.config

local M = {}

---@type hermes.Config
local _config = {}

---@type hermes.Config
local default_config = {
  root_markers = { ".git" },
  permissions = {
    fs_write_access = true,
    fs_read_access = true,
    terminal_access = true,
    request_permissions = true,
    send_notifications = true,
  },
  terminal = {
    delete = true,
    enabled = true,
    buffered = true,
  },
  buffer = {
    auto_save = false,
  },
  log = {
    stdio = {
      level = "off",
      format = "compact",
    },
    notification = {
      level = "error",
      format = "compact",
    },
    message = {
      level = "off",
      format = "compact",
    },
    quickfix = {
      level = "off",
      format = "compact",
    },
    file = {
      level = "off",
      format = "json",
      path = vim.fn.stdpath("state") .. "/hermes.log",
      max_size = 10485760,
      max_files = 5,
    },
  },
  version = "latest",
  auto_download_binary = true,
}

---Setup hermes configuration
---Multiple calls merge configurations - only specified fields are updated
---@param opts? hermes.Config User configuration options
function M.setup(opts)
  opts = opts or {}
  _config = vim.tbl_deep_extend("force", default_config, _config, opts)
end

---Get current configuration
---@return hermes.Config Current configuration
function M.get()
  return _config
end

---Get default configuration
---@return hermes.Config Default configuration
function M.get_defaults()
  return vim.deepcopy(default_config)
end

---Reset configuration to defaults
function M.reset()
  _config = vim.deepcopy(default_config)
end

---Validate configuration
---@param opts hermes.Config Configuration to validate
---@return boolean valid Whether configuration is valid
---@return string|nil error Error message if invalid
function M.validate(opts)
  -- Check permissions
  if opts.permissions then
    for key, value in pairs(opts.permissions) do
      if type(value) ~= "boolean" then
        return false, string.format("permissions.%s must be a boolean", key)
      end
    end
  end
  
  -- Check version
  if opts.version and type(opts.version) ~= "string" then
    return false, "version must be a string"
  end
  
  -- Check auto_download_binary
  if opts.auto_download_binary ~= nil and type(opts.auto_download_binary) ~= "boolean" then
    return false, "auto_download_binary must be a boolean"
  end
  
  -- Check log configuration
  if opts.log then
    for key, value in pairs(opts.log) do
      if type(value) ~= "table" then
        return false, string.format("log.%s must be a table", key)
      end
    end
  end
  
  return true, nil
end

return M
