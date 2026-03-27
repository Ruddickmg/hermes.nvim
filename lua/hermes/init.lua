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

-- Lazy-loaded native module
local _native = nil

-- Forward declaration
local get_native

-- Async loading state
local _loading_state = "NOT_LOADED"  -- NOT_LOADED, DOWNLOADING, LOADING, READY, FAILED
local _loading_error = nil
local _download_timeout = 60

-- Execute function asynchronously with loading state management
-- Minimal untestable wrapper (~25 lines)
local function execute_async(fn)
  if _loading_state == "READY" then
    -- Already loaded, execute immediately
    fn()
    return
  end
  
  if _loading_state == "DOWNLOADING" or _loading_state == "LOADING" then
    -- Still loading, notify user
    vim.notify("Hermes: Binary is still loading. Check :Hermes status for progress.", vim.log.levels.WARN)
    return
  end
  
  if _loading_state == "FAILED" then
    -- Previously failed
    vim.notify("Hermes: Failed to load. Run :Hermes status for details or :Hermes log for errors.", vim.log.levels.ERROR)
    return
  end
  
  -- NOT_LOADED: Start async loading
  local binary = require("hermes.binary")
  local config = require("hermes.config")
  
  -- Get download settings
  local download_cfg = config.get_download and config.get_download() or { auto_download_binary = true, timeout = 60 }
  local auto_download = download_cfg.auto_download_binary
  _download_timeout = download_cfg.timeout or 60
  
  if auto_download == false then
    -- User disabled auto-download, only load existing binary
    _loading_state = "LOADING"
    vim.schedule(function()
      local ok, loaded = pcall(function()
        local bin_path = binary.load_existing_binary()
        local lib, err = package.loadlib(bin_path, "luaopen_hermes")
        if not lib then
          error(string.format("Failed to load: %s", tostring(err)))
        end
        return lib()
      end)
      
      if ok then
        _native = loaded
        _loading_state = "READY"
        fn()
      else
        _loading_state = "FAILED"
        _loading_error = tostring(loaded)
        vim.notify("Hermes: Failed to load binary (auto-download disabled). Run :Hermes status for details.", vim.log.levels.ERROR)
      end
    end)
    return
  end
  
  -- Auto-download enabled
  _loading_state = "DOWNLOADING"
  vim.notify("Hermes: Downloading binary...", vim.log.levels.INFO)
  
  -- Use async download wrapper
  binary.ensure_binary_async(_download_timeout, function(success, result)
    vim.schedule(function()
      if not success then
        _loading_state = "FAILED"
        _loading_error = tostring(result)
        vim.notify("Hermes: Binary download failed. Run :Hermes status or :Hermes log for details.", vim.log.levels.ERROR)
        return
      end
      
      -- Download successful, now load the binary
      _loading_state = "LOADING"
      local ok, loaded = pcall(get_native)
      
      if not ok then
        _loading_state = "FAILED"
        _loading_error = tostring(loaded)
        vim.notify("Hermes: Failed to load binary. Run :Hermes status or :Hermes log for details.", vim.log.levels.ERROR)
        return
      end
      
      _native = loaded
      _loading_state = "READY"
      vim.notify("Hermes: Ready", vim.log.levels.INFO)
      
      -- Execute the queued function
      fn()
    end)
  end)
end

-- Get current loading state
function M.get_loading_state()
  return _loading_state
end

-- Get loading error if any
function M.get_loading_error()
  return _loading_error
end

-- Load native module on first use (synchronous, called by execute_async)
---@return table native_module The loaded native Hermes module
get_native = function()
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
    get_native().setup(opts)
  end)
end

-- ============================================================================
-- ============================================================================
-- API Exports (match Rust API exactly per README.md)
-- ============================================================================

---Connect to an ACP agent
---@param agent "opencode"|"copilot"|"gemini"|string Agent name (predefined or custom)
---@param opts? ConnectionOptions Connection options
function M.connect(agent, opts)
  execute_async(function()
    get_native().connect(agent, opts or {})
  end)
end

---Disconnect from agent(s)
---@param agents? string|string[] Agent name(s) to disconnect, nil for all
function M.disconnect(agents)
  execute_async(function()
    get_native().disconnect(agents)
  end)
end

---Authenticate with an agent
---@param auth_method_id string Authentication method ID from ConnectionInitialized
function M.authenticate(auth_method_id)
  execute_async(function()
    get_native().authenticate(auth_method_id)
  end)
end

---Create a new session
---@param opts? SessionOptions Session configuration options
function M.create_session(opts)
  execute_async(function()
    get_native().create_session(opts or {})
  end)
end

---Load an existing session
---@param session_id string Session ID to load
---@param opts? SessionOptions Session configuration options
function M.load_session(session_id, opts)
  execute_async(function()
    get_native().load_session(session_id, opts or {})
  end)
end

---List sessions with optional filtering
---@param opts? ListSessionsOptions Filter options
function M.list_sessions(opts)
  execute_async(function()
    get_native().list_sessions(opts)
  end)
end

---Send a prompt to the agent
---@param session_id string Session ID
---@param content PromptContent|PromptContent[] Content to send (single item or array)
function M.prompt(session_id, content)
  execute_async(function()
    get_native().prompt(session_id, content)
  end)
end

---Cancel current operation
---@param session_id string Session ID
function M.cancel(session_id)
  execute_async(function()
    get_native().cancel(session_id)
  end)
end

---Set session mode
---@param session_id string Session ID
---@param mode_id string Mode ID
function M.set_mode(session_id, mode_id)
  execute_async(function()
    get_native().set_mode(session_id, mode_id)
  end)
end

---Respond to a request
---@param request_id string Request ID from autocommand
---@param response? any Response data
function M.respond(request_id, response)
  execute_async(function()
    get_native().respond(request_id, response)
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
  complete = function(ArgLead, CmdLine, CursorPos)
    return { "status", "log", "logs" }
  end,
  desc = "Hermes commands: status, log"
})

-- Export get_native for testing purposes
-- This allows tests to verify the Rust FFI boundary
M._get_native = get_native

return M
