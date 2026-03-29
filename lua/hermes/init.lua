-- luacov: disable
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

---@class HermesDownloadConfig
---Download configuration for binary management
---@field version? string Version to use ("latest" or specific version like "v0.1.0")
---@field auto? boolean Whether to auto-download pre-built binary (default: true)
---@field timeout? number Download timeout in seconds (default: 60)

---@class HermesConfig
---Hermes plugin configuration options
---@field download? HermesDownloadConfig Download configuration for binary management
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
-- luacov: enable

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

-- luacov: disable
---Get current loading state
---@private
-- luacov: enable
function M.get_loading_state()
	return _loading_state
end

-- luacov: disable
---Get loading error if any
---@private
-- luacov: enable
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
	local download_cfg = config.get_download()
	return not download_cfg or download_cfg.auto ~= false
end

-- ============================================================================
-- State Transition Functions (pure sync, fully testable)
-- ============================================================================

-- Handle READY state: execute function immediately
local function handle_ready_state(fn)
	fn()
	return true
end

-- Handle loading states: queue the request and return
local function handle_loading_state(fn)
	local queue = require("hermes.queue")
	queue.push(fn)
	vim.notify("Hermes: Request queued, will execute when ready", vim.log.levels.DEBUG)
	return false
end

-- Handle FAILED state: show error, clear queue, and return
local function handle_failed_state()
	-- Clear any queued calls since binary won't load
	local queue = require("hermes.queue")
	local cleared = queue.clear()
	if cleared > 0 then
		vim.notify(
			string.format("Hermes: Cleared %d queued operations due to load failure", cleared),
			vim.log.levels.WARN
		)
	end

	vim.notify(
		"Hermes: Failed to load. Run :Hermes status for details or :Hermes log for errors.",
		vim.log.levels.ERROR
	)
	return false
end

-- Handle successful load completion (sync state update)
local function handle_load_success(loaded_module, fn)
	_native = loaded_module
	_loading_state = "READY"
	vim.notify("Hermes: Ready", vim.log.levels.INFO)
	fn()

	-- Execute any queued calls
	local queue = require("hermes.queue")
	if not queue.is_empty() then
		local executed, err = queue.execute_all()
		if err then
			vim.notify("Hermes: Queued operation failed: " .. err, vim.log.levels.ERROR)
		end
	end
end

-- Handle load failure (sync state update)
-- err_msg can be a string or a structured error table from download module
local function handle_load_failure(err_msg, context)
	_loading_state = "FAILED"
	_loading_error = err_msg

	-- Clear any queued calls since binary won't load
	local queue = require("hermes.queue")
	local cleared = queue.clear()
	if cleared > 0 then
		vim.notify(
			string.format("Hermes: Cleared %d queued operations due to load failure", cleared),
			vim.log.levels.WARN
		)
	end

	vim.notify("Hermes: " .. context .. ". Run :Hermes status for details.", vim.log.levels.ERROR)
end

-- Format structured error for display
-- @param err table|string Error info (structured table or plain string)
-- @return string formatted error message for display
local function format_error_for_display(err)
	if type(err) ~= "table" then
		return tostring(err)
	end

	local lines = {}

	-- Main error message
	if err.message then
		table.insert(lines, "Error: " .. err.message)
	end

	-- URL attempted
	if err.url then
		table.insert(lines, "URL: " .. err.url)
	end

	-- HTTP status code with description
	if err.http_code then
		local code_desc = {
			[404] = " (Not Found)",
			[403] = " (Forbidden)",
			[401] = " (Unauthorized)",
			[500] = " (Server Error)",
			[502] = " (Bad Gateway)",
			[503] = " (Service Unavailable)",
			[504] = " (Gateway Timeout)",
		}
		local desc = code_desc[err.http_code] or ""
		table.insert(lines, "HTTP Code: " .. err.http_code .. desc)
	end

	-- Tool used
	if err.tool then
		table.insert(lines, "Download Tool: " .. err.tool)
	end

	-- Exit code
	if err.exit_code then
		table.insert(lines, "Exit Code: " .. err.exit_code)
	end

	-- Additional error details (stderr)
	if err.stderr and err.stderr ~= "" and err.stderr ~= err.message then
		local stderr_preview = err.stderr:sub(1, 200)
		if #err.stderr > 200 then
			stderr_preview = stderr_preview .. "..."
		end
		table.insert(lines, "Details: " .. stderr_preview)
	end

	return table.concat(lines, "\n  ")
end

-- Get suggested fix based on error type
-- @param err table|string Error info
-- @return string suggestion
local function get_error_suggestion(err)
	if type(err) ~= "table" then
		return "Try building from source with :Hermes build"
	end

	-- Suggest based on HTTP code
	if err.http_code == 404 then
		return "Version not found. Check available versions at: https://github.com/Ruddickmg/hermes.nvim/releases"
	elseif err.http_code == 403 then
		return "Download blocked. This may be due to rate limiting or network restrictions. Try building from source with :Hermes build"
	elseif err.http_code == 401 then
		return "Authentication required. Check if this is a private repository or try building from source."
	elseif err.http_code and err.http_code >= 500 then
		return "GitHub server error. Wait a moment and try again, or build from source with :Hermes build"
	elseif err.message and err.message:match("too small") then
		return "Download incomplete. This may be due to network issues. Try again or build from source with :Hermes build"
	elseif err.message and err.message:match("No download tool available") then
		return "Install curl or wget to enable automatic downloads, or build from source with :Hermes build"
	end

	return "Try building from source with :Hermes build, or check your internet connection"
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

-- luacov: disable
---Load native module synchronously (can be tested with mocked deps)
---@private
-- luacov: enable
function M._load_native_sync()
	if not _native then
		local binary = require("hermes.binary")
		local ok, result = pcall(binary.load_or_build)

		if not ok then
			error("Hermes: Failed to load or build native binary: " .. tostring(result))
		end

		_native = result
	end
	return _native
end

-- Main async executor - minimal async code, delegates to sync functions
local function execute_async(fn)
	-- EARLY CHECK: If binary already exists and matches config, load silently
	local binary = require("hermes.binary")
	local bin_path = binary.get_binary_path()

	if vim.fn.filereadable(bin_path) == 1 then
		local ver_file = binary.get_version_file()
		local version = require("hermes.version")
		local configured_ver = version.get_wanted()

		-- Check if we can use existing binary
		local should_use_existing = false

		if vim.fn.filereadable(ver_file) == 1 then
			-- Use pcall in case file becomes unreadable between check and read
			local ok, installed_ver = pcall(function()
				return vim.fn.readfile(ver_file)[1]
			end)
			-- Use existing if configured version matches installed version
			if ok and configured_ver == installed_ver then
				should_use_existing = true
			end
		end

		if should_use_existing then
			-- Binary exists and version matches - try to load it
			-- Use pcall to handle load failures gracefully
			local ok, result = pcall(function()
				local lib, err = package.loadlib(bin_path, "luaopen_hermes")
				if not lib then
					error("Failed to load: " .. tostring(err))
				end
				return lib()
			end)

			if ok then
				-- Successfully loaded existing binary
				-- Use vim.schedule for consistency with other loading paths
				vim.schedule(function()
					_native = result
					_loading_state = "READY"
					vim.notify("Hermes: Ready", vim.log.levels.INFO)
					fn() -- Execute the callback
				end)
				return
			else
				-- Loading failed - binary might be corrupted
				-- Remove the invalid binary and version file to force re-download
				pcall(function()
					vim.fn.delete(bin_path)
					vim.fn.delete(ver_file)
				end)
				-- Fall through to download flow
			end
		end
	end

	-- Check states in priority order (all sync checks)
	if is_ready() then
		return handle_ready_state(fn)
	end

	if is_loading() then
		return handle_loading_state(fn)
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

	-- Start async download (NOW we show the notification)
	_loading_state = "DOWNLOADING"
	vim.notify("Hermes: Downloading binary...", vim.log.levels.INFO)

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
		download = opts.download,
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

-- Build status content (lines and highlights) for display
-- Pure function - can be tested without Neovim UI
-- @param state string Current loading state (e.g., "READY", "FAILED")
-- @param error table|nil Error information if state is "FAILED"
-- @return table lines Array of strings to display
-- @return table highlights Array of highlight specifications
local function build_status_content(state, error)
	local lines = {}
	local highlights = {}
	local line_count = 0

	local function add_line(text, hl)
		table.insert(lines, text)
		line_count = line_count + 1
		if hl then
			table.insert(highlights, { hl, line_count - 1, 0, -1, -1 })
		end
		return line_count
	end

	-- Header
	add_line("Hermes Status", "Title")
	add_line(string.rep("=", 60))
	add_line("")

	-- Current state with appropriate highlight
	local state_line = "State: " .. state
	add_line(state_line)
	if state == "READY" then
		table.insert(highlights, { "DiagnosticOk", line_count - 1, 7, -1, -1 })
	elseif state == "FAILED" then
		table.insert(highlights, { "DiagnosticError", line_count - 1, 7, -1, -1 })
	elseif state == "DOWNLOADING" or state == "LOADING" then
		table.insert(highlights, { "DiagnosticWarn", line_count - 1, 7, -1, -1 })
	end

	-- Binary information
	local binary = require("hermes.binary")
	add_line("Binary Path: " .. binary.get_binary_path())

	local version = require("hermes.version")
	add_line("Version: " .. version.get_wanted())

	-- Check if binary exists
	local bin_path = binary.get_binary_path()
	if vim.fn.filereadable(bin_path) == 1 then
		local size = vim.fn.getfsize(bin_path)
		add_line("Binary Size: " .. size .. " bytes")
	else
		add_line("Binary Size: Not found")
	end

	add_line("")

	-- Error details if failed
	if state == "FAILED" and error then
		add_line("Error Details:", "DiagnosticError")
		add_line(string.rep("-", 60))

		local error_text = format_error_for_display(error)
		-- Split error text into lines and add with indentation
		for _, err_line in ipairs(vim.split(error_text, "\n")) do
			add_line("  " .. err_line)
		end

		add_line("")
		add_line("Suggested Fix:", "DiagnosticWarn")
		add_line("  " .. get_error_suggestion(error))

		add_line("")
		add_line("Troubleshooting:")
		add_line("  1. Check your internet connection")
		add_line("  2. Verify the version exists at:")
		add_line("     https://github.com/Ruddickmg/hermes.nvim/releases")
		add_line("  3. Try building manually: :Hermes build")
		add_line("  4. Check logs: :Hermes log")
	end

	-- Platform info
	add_line("")
	add_line("Platform Information:")
	add_line(string.rep("-", 60))
	local platform = require("hermes.platform")
	add_line("  OS: " .. (platform.get_os() or "unknown"))
	add_line("  Architecture: " .. (platform.get_arch() or "unknown"))
	add_line("  Platform Key: " .. (platform.get_platform_key() or "unknown"))

	-- Download tool info
	add_line("")
	add_line("Download Tools:")
	add_line(string.rep("-", 60))
	local download = require("hermes.download")
	add_line("  curl: " .. (download.is_curl_available() and "available" or "not found"))
	add_line("  wget: " .. (download.is_wget_available() and "available" or "not found"))
	add_line("  PowerShell: " .. (download.is_powershell_available() and "available" or "not found"))

	return lines, highlights
end

-- Show detailed status information including any download errors
-- Creates a formatted buffer with status details
local function show_status()
	local lines, highlights = build_status_content(_loading_state, _loading_error)

	-- Create floating window
	local buf = vim.api.nvim_create_buf(false, true)
	vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)

	-- Apply highlights
	for _, hl in ipairs(highlights) do
		vim.api.nvim_buf_add_highlight(buf, -1, hl[1], hl[2], hl[3], hl[4])
	end

	-- Calculate window size
	local width = 70
	local height = math.min(#lines + 2, vim.o.lines - 4)

	-- Center window
	local col = math.floor((vim.o.columns - width) / 2)
	local row = math.floor((vim.o.lines - height) / 2)

	local win = vim.api.nvim_open_win(buf, true, {
		relative = "editor",
		width = width,
		height = height,
		col = col,
		row = row,
		style = "minimal",
		border = "rounded",
		title = " Hermes Status ",
		title_pos = "center",
	})

	-- Set buffer options
	vim.bo[buf].modifiable = false
	vim.bo[buf].buftype = "nofile"

	-- Add keymaps to close
	vim.keymap.set("n", "q", function()
		vim.api.nvim_win_close(win, true)
	end, { buffer = buf, silent = true })
	vim.keymap.set("n", "<Esc>", function()
		vim.api.nvim_win_close(win, true)
	end, { buffer = buf, silent = true })
end

-- Register :Hermes user command
vim.api.nvim_create_user_command("Hermes", function(opts)
	local args = opts.args:lower()

	if args == "status" then
		show_status()
	elseif args == "build" then
		-- Trigger build from source
		vim.notify("Hermes: Building from source...", vim.log.levels.INFO)
		vim.schedule(function()
			local binary = require("hermes.binary")
			local data_dir = binary.get_data_dir()
			local ok, err = binary.build_from_source(data_dir)
			if ok then
				vim.notify("Hermes: Build successful! Restart Neovim to load the new binary.", vim.log.levels.INFO)
			else
				vim.notify("Hermes: Build failed: " .. tostring(err), vim.log.levels.ERROR)
			end
		end)
	elseif args == "log" then
		-- Open log file
		local config = require("hermes.config")
		local log_config = config.get_log and config.get_log() or {}
		local log_path = log_config.path
		if log_path and vim.fn.filereadable(log_path) == 1 then
			vim.cmd("vsplit " .. vim.fn.fnameescape(log_path))
		else
			vim.notify("Hermes: No log file found", vim.log.levels.WARN)
		end
	else
		vim.notify("Hermes: Unknown command '" .. opts.args .. "'. Available: status, build, log", vim.log.levels.ERROR)
	end
end, {
	nargs = 1,
	complete = function()
		return { "status", "build", "log" }
	end,
	desc = "Hermes commands: status, build, log",
})

-- ============================================================================
-- Export internal functions for testing (marked private to hide from LSP)
-- ============================================================================

-- luacov: disable

---@private
M._is_ready = is_ready

---@private
M._is_loading = is_loading

---@private
M._is_failed = is_failed

---@private
M._set_loading_state = set_loading_state

---@private
M._set_loading_error = set_loading_error

---@private
M._handle_ready_state = handle_ready_state

---@private
M._handle_loading_state = handle_loading_state

---@private
M._handle_failed_state = handle_failed_state

---@private
M._handle_load_success = handle_load_success

---@private
M._handle_load_failure = handle_load_failure

---@private
M._should_auto_download = should_auto_download

---@private
M._format_error_for_display = format_error_for_display

---@private
M._get_error_suggestion = get_error_suggestion

---@private
M._show_status = show_status

---@private
M._build_status_content = build_status_content

-- luacov: enable

return M
