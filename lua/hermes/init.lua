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
    local ok, result = pcall(binary.load_or_build)
    if not ok then
      -- Format detailed error message
      local error_msg = string.format(
        "Failed to load Hermes binary.\n\n" ..
        "Error: %s\n\n" ..
        "This is likely due to:\n" ..
        "1. Your platform is not supported (only Linux x86_64/aarch64, macOS x86_64/arm64, Windows x86_64)\n" ..
        "2. Network issues preventing download\n" ..
        "3. Build toolchain not available for fallback compilation\n\n" ..
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

---@class hermes.Config
---@field root_markers? string[] Root directory markers (default: {".git"})
---@field permissions? hermes.Permissions Permission settings
---@field terminal? hermes.TerminalConfig Terminal configuration
---@field buffer? hermes.BufferConfig Buffer settings
---@field log? hermes.LogConfig Logging configuration
---@field version? string Binary version to use (default: "latest")

---@class hermes.Permissions
---@field fs_write_access? boolean Allow file writes (default: true)
---@field fs_read_access? boolean Allow file reads (default: true)
---@field terminal_access? boolean Allow terminal access (default: true)
---@field request_permissions? boolean Allow permission requests (default: true)
---@field send_notifications? boolean Allow notifications (default: true)

---@class hermes.TerminalConfig
---@field delete? boolean Auto-delete on exit (default: true)
---@field enabled? boolean Enable terminals (default: true)
---@field buffered? boolean Buffer output (default: true)

---@class hermes.BufferConfig
---@field auto_save? boolean Auto-save after writes (default: false)

---@class hermes.LogConfig
---@field stdio? hermes.LogTargetConfig Stdio logging
---@field notification? hermes.LogTargetConfig Notification logging
---@field message? hermes.LogTargetConfig Message logging
---@field quickfix? hermes.LogTargetConfig Quickfix logging
---@field file? hermes.LogFileConfig File logging

---@class hermes.LogTargetConfig
---@field level? string|number Log level (default: "off")
---@field format? string Log format: "pretty", "compact", "full", "json"

---@class hermes.LogFileConfig
---@field level? string|number Log level (default: "off")
---@field format? string Log format (default: "json")
---@field path? string Log file path
---@field max_size? number Max file size in bytes (default: 10485760)
---@field max_files? number Max backup files (default: 5)

---Setup hermes plugin with configuration
---See |hermes-setup| for detailed documentation
---@param opts? hermes.Config User configuration options
function M.setup(opts)
  require("hermes.config").setup(opts)
end

-- ============================================================================
-- API Exports (mirror Rust API exactly)
-- ============================================================================

---@class hermes.ConnectOptions
---@field protocol? string Connection protocol: "stdio", "http", "socket" (default: "stdio")
---@field command? string Custom agent command (for custom agents)
---@field args? string[] Command arguments (for custom agents)

---Connect to an ACP agent
---See |hermes-connect| for detailed documentation
---@param agent string Agent name ("opencode", "copilot", "gemini", or custom)
---@param opts? hermes.ConnectOptions Connection options
---@return boolean success Whether connection succeeded
---@trigger ConnectionInitialized
function M.connect(agent, opts)
  return get_native().connect(agent, opts or {})
end

---Disconnect from agent(s)
---See |hermes-disconnect| for detailed documentation
---@param agents? string|string[] Agent name(s) to disconnect, nil for all
function M.disconnect(agents)
  return get_native().disconnect(agents)
end

---Authenticate with an agent
---See |hermes-authenticate| for detailed documentation
---@param auth_method_id string Authentication method ID from ConnectionInitialized
function M.authenticate(auth_method_id)
  return get_native().authenticate(auth_method_id)
end

---Create a new session
---See |hermes-create-session| for detailed documentation
---@param opts? table Session configuration { cwd?, mcpServers? }
function M.create_session(opts)
  return get_native().create_session(opts or {})
end

---Load an existing session
---See |hermes-load-session| for detailed documentation
---@param session_id string Session ID to load
---@param opts? table Session configuration { cwd?, mcpServers? }
function M.load_session(session_id, opts)
  return get_native().load_session(session_id, opts or {})
end

---Send a prompt to the agent
---See |hermes-prompt| for detailed documentation
---@param session_id string Session ID
---@param content table|string Content to send (string for text, table for structured content)
function M.prompt(session_id, content)
  return get_native().prompt(session_id, content)
end

---Cancel current operation
---See |hermes-cancel| for detailed documentation
---@param session_id string Session ID
function M.cancel(session_id)
  return get_native().cancel(session_id)
end

---Set session mode
---See |hermes-set-mode| for detailed documentation
---@param session_id string Session ID
---@param mode_id string Mode ID
function M.set_mode(session_id, mode_id)
  return get_native().set_mode(session_id, mode_id)
end

---Respond to a request
---See |hermes-respond| for detailed documentation
---@param request_id string Request ID from autocommand
---@param response? any Response data
function M.respond(request_id, response)
  return get_native().respond(request_id, response)
end

---List available sessions
---See |hermes-list-sessions| for detailed documentation
---@return table sessions List of sessions
function M.list_sessions()
  return get_native().list_sessions()
end

---Fork a session
---See |hermes-fork-session| for detailed documentation
---@param session_id string Session ID to fork
---@param opts? table Fork options
function M.fork_session(session_id, opts)
  return get_native().fork_session(session_id, opts or {})
end

---Resume a session
---See |hermes-resume-session| for detailed documentation
---@param session_id string Session ID to resume
function M.resume_session(session_id)
  return get_native().resume_session(session_id)
end

---Set session model
---See |hermes-set-model| for detailed documentation
---@param session_id string Session ID
---@param model_id string Model ID
function M.set_session_model(session_id, model_id)
  return get_native().set_session_model(session_id, model_id)
end

---Set session configuration option
---See |hermes-set-config| for detailed documentation
---@param session_id string Session ID
---@param option_id string Option ID
---@param value any Option value
function M.set_session_config(session_id, option_id, value)
  return get_native().set_session_config(session_id, option_id, value)
end

return M
