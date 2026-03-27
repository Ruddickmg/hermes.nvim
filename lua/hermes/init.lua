---@brief [[
--- Hermes - ACP (Agent Client Protocol) client for Neovim
--- Provides APIs for communicating with AI assistants
---
--- Usage:
---   require("hermes").setup()  -- Initialize with defaults
---   require("hermes").connect("opencode")  -- Connect to agent
---
---@brief ]]

-- ============================================================================
-- Type Definitions for IDE/LSP Support
-- ============================================================================

---@class HermesConfig
---Hermes plugin configuration options
---@field version? string Version to use ("latest" or specific version like "v0.1.0")
---@field auto_download_binary? boolean Whether to auto-download pre-built binary (default: true)
---@field root_markers? string[] Files/directories to identify project root (default: {".git"})
---@field permissions? HermesPermissions Permission settings for agent operations
---@field terminal? HermesTerminalConfig Terminal configuration
---@field buffer? HermesBufferConfig Buffer configuration
---@field log? HermesLogConfig Logging configuration

---@class HermesPermissions
---Permission settings for agent operations
---@field fs_write_access? boolean Allow agent to write files (default: true)
---@field fs_read_access? boolean Allow agent to read files (default: true)
---@field terminal_access? boolean Allow agent to execute terminal commands (default: true)
---@field request_permissions? boolean Allow agent to send permission requests (default: true)
---@field send_notifications? boolean Allow agent to send notifications (default: true)

---@class HermesTerminalConfig
---Terminal configuration options
---@field delete? boolean Auto-delete terminals on exit (default: true)
---@field enabled? boolean Enable terminal functionality (default: true)
---@field buffered? boolean Buffer terminal output (default: true)

---@class HermesBufferConfig
---Buffer configuration options
---@field auto_save? boolean Auto-save modified files after writing (default: false)

---@class HermesLogConfig
---Logging configuration options
---@field stdio? HermesLogTargetConfig Stdio logging settings
---@field notification? HermesLogTargetConfig Notification logging settings
---@field message? HermesLogTargetConfig Message logging settings (for :messages)
---@field file? HermesLogFileConfig File logging settings

---@class HermesLogTargetConfig
---Log target configuration (stdio, notification, message)
---@field level? number|string Log level (vim.log.levels.* or "trace", "debug", "info", "warn", "error", "off")
---@field format? "compact"|"pretty"|"full"|"json" Log format (default: "compact")

---@class HermesLogFileConfig
---File logging configuration
---@field level? number|string Log level (vim.log.levels.* or string)
---@field format? "compact"|"pretty"|"full"|"json" Log format (default: "json")
---@field path? string Path to log file (default: vim.fn.stdpath('state') .. "/nvim/hermes.log")
---@field max_size? number Maximum file size in bytes (default: 10485760 = 10MB)
---@field max_files? number Maximum number of log files to keep (default: 5)

---@class ConnectionOptions
---Options for connecting to an agent
---@field protocol? "stdio"|"http"|"socket" Connection protocol (default: "stdio")
---@field command? string Command to run for stdio connections
---@field args? string[] Command arguments for stdio connections
---@field url? string URL for http/sse connections

---@class EnvVar
---Environment variable entry
---@field name string Environment variable name
---@field value string Environment variable value

---@class McpServer
---MCP (Model Context Protocol) server configuration
---@field type "http"|"sse"|"stdio" Server type
---@field name string Human-readable server name
---@field url? string URL for http/sse servers (required for http/sse types)
---@field headers? table<string, string>[] HTTP headers as array of key-value tables
---@field command? string Command to execute for stdio servers (required for stdio type)
---@field args? string[] Arguments for stdio server command
---@field env? EnvVar[] Environment variables as array of {name: string, value: string}

---@class SessionOptions
---Options for creating or loading a session
---@field cwd? string Working directory for the session
---@field mcpServers? McpServer[] Array of MCP server configurations

---@class ListSessionsOptions
---Options for listing sessions
---@field cwd? string Filter by working directory path
---@field cursor? string Pagination cursor for fetching next page

---@class TextContent
---Text content for prompts
---@field type "text" Type identifier
---@field text string The text content to send

---@class LinkContent
---Resource link content for prompts
---@field type "link" Type identifier
---@field name string Human-readable resource name
---@field uri string Resource URI (file path or URL)

---@class EmbeddedResource
---Embedded resource data
---@field uri string Resource URI
---@field mimeType? string MIME type of the resource (e.g., "text/x-python", "application/pdf")
---@field text? string Text content (for text resources)
---@field blob? string Base64-encoded binary data (for blob resources)

---@class EmbeddedContent
---Embedded resource content for prompts
---@field type "embedded" Type identifier
---@field resource EmbeddedResource The embedded resource

---@class ImageContent
---Image content for prompts
---@field type "image" Type identifier
---@field data string Base64-encoded image data
---@field mimeType string MIME type (e.g., "image/png", "image/jpeg")

---@class AudioContent
---Audio content for prompts
---@field type "audio" Type identifier
---@field data string Base64-encoded audio data
---@field mimeType string MIME type (e.g., "audio/wav", "audio/mpeg")

---@alias PromptContent TextContent|LinkContent|EmbeddedContent|ImageContent|AudioContent|table

local M = {}

-- ============================================================================
-- Module State (all testable sync operations)
-- ============================================================================

-- Lazy-loaded native module
local _native = nil

-- Loading state: NOT_LOADED, DOWNLOADING, LOADING, READY, FAILED
local _loading_state = "NOT_LOADED"
local _loading_error = nil
local _download_timeout = 60

-- ============================================================================
-- Pure Sync State Management (fully testable)
-- ============================================================================

-- Get current loading state
function M.get_loading_state()
  return _loading_state
end

-- Get loading error if any
function M.get_loading_error()
  return _loading_error
end

-- Check if binary is already ready (sync)
local function is_ready()
  return _loading_state == "READY"
end

-- Check if binary is currently loading (sync)
local function is_loading()
  return _loading_state == "DOWNLOADING" or _loading_state == "LOADING"
end

-- Check if loading previously failed (sync)
local function is_failed()
  return _loading_state == "FAILED"
end

-- Set loading state (sync, for testing)
local function set_loading_state(state)
  _loading_state = state
end

-- Set loading error (sync, for testing)
local function set_loading_error(err)
  _loading_error = err
end

-- Get auto-download setting from config (sync)
local function should_auto_download()
  local config = require("hermes.config")
  if type(config.get_auto_download) == "function" then
    return config.get_auto_download()
  elseif type(config.get) == "function" then
    local cfg = config.get()
    if cfg and cfg.auto_download_binary ~= nil then
      return cfg.auto_download_binary
    end
  end
  return true
end

-- ============================================================================
-- State Transition Functions (pure sync, fully testable)
-- ============================================================================

-- Handle READY state: execute function immediately
local function handle_ready_state(fn)
  fn()
  return true
end

-- Handle loading states: show warning and return
local function handle_loading_state()
  vim.notify("Hermes: Binary is still loading. Check :Hermes status for progress.", vim.log.levels.WARN)
  return false
end

-- Handle FAILED state: show error and return
local function handle_failed_state()
  vim.notify("Hermes: Failed to load. Run :Hermes status for details or :Hermes log for errors.", vim.log.levels.ERROR)
  return false
end

-- Handle successful load completion (sync state update)
local function handle_load_success(loaded_module, fn)
  _native = loaded_module
  _loading_state = "READY"
  vim.notify("Hermes: Ready", vim.log.levels.INFO)
  fn()
end

-- Handle load failure (sync state update)
local function handle_load_failure(err_msg, context)
  _loading_state = "FAILED"
  _loading_error = err_msg
  vim.notify("Hermes: " .. context .. ". Run :Hermes status or :Hermes log for details.", vim.log.levels.ERROR)
end

-- Handle download completion and trigger load (async entry point)
local function handle_download_complete(success, result, fn)
  if not success then
    handle_load_failure(tostring(result), "Binary download failed")
    return
  end
  
  -- Download successful, now load the binary
  _loading_state = "LOADING"
  
  -- Use vim.schedule for the actual loading (only async part)
  vim.schedule(function()
    local ok, loaded = pcall(M._load_native_sync)
    
    if not ok then
      handle_load_failure(tostring(loaded), "Failed to load binary")
      return
    end
    
    handle_load_success(loaded, fn)
  end)
end

-- Handle auto-download disabled path (async entry point)
local function handle_auto_download_disabled(fn)
  _loading_state = "LOADING"
  vim.notify("Hermes: Loading binary...", vim.log.levels.INFO)
  
  -- Use vim.schedule for the actual loading (only async part)
  vim.schedule(function()
    local binary = require("hermes.binary")
    
    local ok, loaded = pcall(function()
      local bin_path = binary.load_existing_binary()
      local lib, err = package.loadlib(bin_path, "luaopen_hermes")
      if not lib then
        error(string.format("Failed to load: %s", tostring(err)))
      end
      return lib()
    end)
    
    if ok then
      handle_load_success(loaded, fn)
    else
      handle_load_failure(tostring(loaded), "Failed to load binary (auto-download disabled)")
    end
  end)
end

-- ============================================================================
-- Binary Loading (sync version for testing, async wrapper for production)
-- ============================================================================

-- Load native module synchronously (can be tested with mocked deps)
function M._load_native_sync()
  if not _native then
    local binary = require("hermes.binary")
    local ok, result = pcall(binary.load_or_build)
    
    if not ok then
      error(result)
    end
    
    _native = result
  end
  return _native
end

-- Main async executor - minimal async code, delegates to sync functions
local function execute_async(fn)
  -- Check states in priority order (all sync checks)
  if is_ready() then
    return handle_ready_state(fn)
  end
  
  if is_loading() then
    return handle_loading_state()
  end
  
  if is_failed() then
    return handle_failed_state()
  end
  
  -- NOT_LOADED: Need to start loading
  local config = require("hermes.config")
  local download_cfg = config.get_download and config.get_download() or { auto_download_binary = true, timeout = 60 }
  _download_timeout = download_cfg.timeout or 60
  
  if not should_auto_download() then
    return handle_auto_download_disabled(fn)
  end
  
  -- Start async download
  _loading_state = "DOWNLOADING"
  vim.notify("Hermes: Downloading binary...", vim.log.levels.INFO)
  
  local binary = require("hermes.binary")
  binary.ensure_binary_async(_download_timeout, function(success, result)
    handle_download_complete(success, result, fn)
  end)
end

-- ============================================================================
-- API Exports (match Rust API exactly per README.md)
-- ============================================================================

---Setup hermes plugin with configuration
---All configuration is passed to the Rust binary
---@param opts? HermesConfig User configuration options
function M.setup(opts)
  opts = opts or {}
  
  -- Store installation-related config locally
  require("hermes.config").setup({
    version = opts.version,
    auto_download_binary = opts.auto_download_binary,
    log = opts.log,
  })
  
  -- Execute async with loading state management
  execute_async(function()
    M._load_native_sync().setup(opts)
  end)
end

---Connect to an ACP agent
---@param agent "opencode"|"copilot"|"gemini"|string Agent name (predefined or custom)
---@param opts? ConnectionOptions Connection options
function M.connect(agent, opts)
  execute_async(function()
    M._load_native_sync().connect(agent, opts or {})
  end)
end

---Disconnect from agent(s)
---@param agents? string|string[] Agent name(s) to disconnect, nil for all
function M.disconnect(agents)
  execute_async(function()
    M._load_native_sync().disconnect(agents)
  end)
end

---Authenticate with an agent
---@param auth_method_id string Authentication method ID from ConnectionInitialized
function M.authenticate(auth_method_id)
  execute_async(function()
    M._load_native_sync().authenticate(auth_method_id)
  end)
end

---Create a new session
---@param opts? SessionOptions Session configuration options
function M.create_session(opts)
  execute_async(function()
    M._load_native_sync().create_session(opts or {})
  end)
end

---Load an existing session
---@param session_id string Session ID to load
---@param opts? SessionOptions Session configuration options
function M.load_session(session_id, opts)
  execute_async(function()
    M._load_native_sync().load_session(session_id, opts or {})
  end)
end

---List sessions with optional filtering
---@param opts? ListSessionsOptions Filter options
function M.list_sessions(opts)
  execute_async(function()
    M._load_native_sync().list_sessions(opts)
  end)
end

---Send a prompt to the agent
---@param session_id string Session ID
---@param content PromptContent|PromptContent[] Content to send (single item or array)
function M.prompt(session_id, content)
  execute_async(function()
    M._load_native_sync().prompt(session_id, content)
  end)
end

---Cancel current operation
---@param session_id string Session ID
function M.cancel(session_id)
  execute_async(function()
    M._load_native_sync().cancel(session_id)
  end)
end

---Set session mode
---@param session_id string Session ID
---@param mode_id string Mode ID
function M.set_mode(session_id, mode_id)
  execute_async(function()
    M._load_native_sync().set_mode(session_id, mode_id)
  end)
end

---Respond to a request
---@param request_id string Request ID from autocommand
---@param response? any Response data
function M.respond(request_id, response)
  execute_async(function()
    M._load_native_sync().respond(request_id, response)
  end)
end

-- ============================================================================
-- User Commands (:Hermes status, :Hermes log)
-- ============================================================================

-- Create user commands with space-separated names
vim.api.nvim_create_user_command("Hermes", function(opts)
  local subcommand = opts.args
  local state = M.get_loading_state()
  local error_msg = M.get_loading_error()
  local config = require("hermes.config")
  local download_cfg = config.get_download and config.get_download() or {}
  
  if subcommand == "status" then
    local status_lines = {
      "Hermes Status",
      "=============",
      "",
      "State: " .. state,
    }
    
    if state == "NOT_LOADED" then
      table.insert(status_lines, "The binary has not been loaded yet. Run any Hermes API method to start loading.")
    elseif state == "DOWNLOADING" then
      table.insert(status_lines, "The binary is being downloaded...")
      table.insert(status_lines, "Timeout: " .. tostring(_download_timeout) .. " seconds")
    elseif state == "LOADING" then
      table.insert(status_lines, "The binary has been downloaded and is being loaded...")
    elseif state == "READY" then
      table.insert(status_lines, "Hermes is ready to use!")
    elseif state == "FAILED" then
      table.insert(status_lines, "Loading failed with error:")
      table.insert(status_lines, error_msg or "Unknown error")
    end
    
    table.insert(status_lines, "")
    table.insert(status_lines, "Configuration:")
    table.insert(status_lines, "  Auto-download: " .. tostring(download_cfg.auto ~= false))
    table.insert(status_lines, "  Version: " .. tostring(download_cfg.version or "latest"))
    table.insert(status_lines, "  Timeout: " .. tostring(download_cfg.timeout or 60) .. " seconds")
    
    vim.notify(table.concat(status_lines, "\n"), vim.log.levels.INFO)
    
  elseif subcommand == "log" or subcommand == "logs" then
    -- Show recent log messages
    local log_lines = {
      "Hermes Log",
      "==========",
      "",
      "Recent log messages will appear here.",
      "Use :messages to see all notifications.",
      "",
      "Current State: " .. state,
    }
    
    if error_msg then
      table.insert(log_lines, "Last Error: " .. error_msg)
    end
    
    vim.notify(table.concat(log_lines, "\n"), vim.log.levels.INFO)
    
  else
    vim.notify("Unknown Hermes command: " .. tostring(subcommand) .. ". Use :Hermes status or :Hermes log", vim.log.levels.ERROR)
  end
end, {
  nargs = 1,
  complete = function(_ArgLead, _CmdLine, _CursorPos)
    return { "status", "log", "logs" }
  end,
  desc = "Hermes commands: status, log"
})

-- Export internal functions for testing
M._is_ready = is_ready
M._is_loading = is_loading
M._is_failed = is_failed
M._set_loading_state = set_loading_state
M._set_loading_error = set_loading_error
M._handle_ready_state = handle_ready_state
M._handle_loading_state = handle_loading_state
M._handle_failed_state = handle_failed_state
M._handle_load_success = handle_load_success
M._handle_load_failure = handle_load_failure
M._should_auto_download = should_auto_download

return M
