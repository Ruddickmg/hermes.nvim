---@brief [[
--- Hermes - ACP (Agent Client Protocol) client for Neovim
--- Provides APIs for communicating with AI assistants
---
--- Usage:
---   require("hermes").setup()  -- Initialize with defaults
---   require("hermes").connect("opencode")  -- Connect to agent
---
---@brief ]]

local M = {}

-- Lazy-loaded native module
local _native = nil

-- Load native module on first use
---@return table native_module The loaded native Hermes module
local function get_native()
  if not _native then
    local binary = require("hermes.binary")
    local config = require("hermes.config")
    
    -- Determine whether auto-download is enabled from config module API
    local auto_download = true
    if type(config.get_auto_download) == "function" then
      auto_download = config.get_auto_download()
    elseif type(config.get) == "function" then
      local cfg = config.get()
      if cfg and cfg.auto_download_binary ~= nil then
        auto_download = cfg.auto_download_binary
      end
    end

    local ok, result
    if auto_download == false then
      -- User disabled auto-download, only load existing binary
      ok, result = pcall(function()
        local bin_path = binary.load_existing_binary()
        local lib, err = package.loadlib(bin_path, "luaopen_hermes")
        if not lib then
          error(string.format(
            "Failed to load native module from: %s\nError: %s",
            bin_path,
            tostring(err)
          ))
        end
        return lib()
      end)
    else
      -- Default: auto-download if needed
      ok, result = pcall(binary.load_or_build)
    end
    
    if not ok then
      -- Format detailed error message
      local error_msg = string.format(
        "Failed to load Hermes binary.\n\n" ..
        "Error: %s\n\n" ..
        "This is likely due to:\n" ..
        "1. Your platform is not supported (only Linux x86_64/aarch64, macOS x86_64/arm64, Windows x86_64)\n" ..
        "2. Network issues preventing download\n" ..
        "3. Local environment issues (e.g. permissions or missing system dependencies)\n\n" ..
        "If you believe this is a bug, please create an issue at:\n" ..
        "https://github.com/Ruddickmg/hermes.nvim/issues\n\n" ..
        "When reporting, please include:\n" ..
        "- Your operating system and version\n" ..
        "- Output of :lua print(vim.loop.os_uname().sysname .. ' ' .. vim.loop.os_uname().machine)\n" ..
        "- The full error message above",
        tostring(result)
      )
      error(error_msg)
    end
    _native = result
  end
  return _native
end

---Setup hermes plugin with configuration
---All configuration is passed to the Rust binary
---@param opts? table User configuration options
--- TODO: ensure config is passed to the rust binary if it is loaded after setup is called.
function M.setup(opts)
  opts = opts or {}
  
  -- Store installation-related config locally
  require("hermes.config").setup({
    version = opts.version,
    auto_download_binary = opts.auto_download_binary,
    log = opts.log,
  })
  
  -- Pass all config to Rust binary if it's already loaded
  if _native then
    _native.setup(opts)
  end
end

-- ============================================================================
-- API Exports (match Rust API exactly per README.md)
-- ============================================================================

---Connect to an ACP agent
---@param agent string Agent name ("opencode", "copilot", "gemini", or custom)
---@param opts? table Connection options { protocol?, command?, args? }
function M.connect(agent, opts)
  return get_native().connect(agent, opts or {})
end

---Disconnect from agent(s)
---@param agents? string|string[] Agent name(s) to disconnect, nil for all
function M.disconnect(agents)
  return get_native().disconnect(agents)
end

---Authenticate with an agent
---@param auth_method_id string Authentication method ID from ConnectionInitialized
function M.authenticate(auth_method_id)
  return get_native().authenticate(auth_method_id)
end

---Create a new session
---@param opts? table Session configuration { cwd?, mcpServers? }
function M.create_session(opts)
  return get_native().create_session(opts or {})
end

---Load an existing session
---@param session_id string Session ID to load
---@param opts? table Session configuration { cwd?, mcpServers? }
function M.load_session(session_id, opts)
  return get_native().load_session(session_id, opts or {})
end

---Send a prompt to the agent
---@param session_id string Session ID
---@param content table|string Content to send
function M.prompt(session_id, content)
  return get_native().prompt(session_id, content)
end

---Cancel current operation
---@param session_id string Session ID
function M.cancel(session_id)
  return get_native().cancel(session_id)
end

---Set session mode
---@param session_id string Session ID
---@param mode_id string Mode ID
function M.set_mode(session_id, mode_id)
  return get_native().set_mode(session_id, mode_id)
end

---Respond to a request
---@param request_id string Request ID from autocommand
---@param response? any Response data
function M.respond(request_id, response)
  return get_native().respond(request_id, response)
end

return M
