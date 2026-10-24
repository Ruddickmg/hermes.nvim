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
-- luacov: enable

-- Export enums and types from the root for convenience
local types_module = require("hermes.types")

local M = {}

M.Protocol = types_module.Protocol
M.Distribution = types_module.Distribution
M.McpServerType = types_module.McpServerType
M.PermissionKind = types_module.PermissionKind
M.ElicitationAction = types_module.ElicitationAction
M.ElicitationOutcome = types_module.ElicitationOutcome
M.PermissionOutcome = types_module.PermissionOutcome
M.ToolCallStatus = types_module.ToolCallStatus
M.PlanPriority = types_module.PlanPriority
M.PlanStatus = types_module.PlanStatus
M.ConfigOptionType = types_module.ConfigOptionType
M.ContentType = types_module.ContentType
M.LogLevel = types_module.LogLevel
M.LogFormat = types_module.LogFormat

-- Import logging module for notifications
local logging = require("hermes.logging")

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
---Checks actual binary existence for more accurate state reporting
---@private
-- luacov: enable
function M.get_loading_state()
	-- If we think we're READY but binary doesn't exist, we're actually NOT_LOADED
	if _loading_state == "READY" then
		local binary = require("hermes.binary")
		local bin_path = binary.get_active_binary_path()
		if vim.fn.filereadable(bin_path) ~= 1 then
			return "NOT_LOADED"
		end
	end
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
	return fn()
end

-- Handle loading states: queue the request and return
local function handle_loading_state(fn)
	if type(fn) == "function" then
		local queue = require("hermes.queue")
		queue.push(fn)
		logging.notify("Request queued, will execute when ready", vim.log.levels.DEBUG)
	else
		logging.notify("Invalid function provided to execute_async", vim.log.levels.WARN)
	end
end

-- Handle FAILED state: show error, clear queue, and return
local function handle_failed_state()
	-- Clear any queued calls since binary won't load
	local queue = require("hermes.queue")
	local cleared = queue.clear()
	if cleared > 0 then
		logging.notify(string.format("Cleared %d queued operations due to load failure", cleared), vim.log.levels.WARN)
	end

	logging.notify("Failed to load. Run :Hermes status for details or :Hermes log for errors.", vim.log.levels.ERROR)
end

-- Handle successful load completion (sync state update)
local function handle_load_success(loaded_module, fn)
	_native = loaded_module
	_loading_state = "READY"
	logging.notify("Successfully Loaded", vim.log.levels.DEBUG)
	fn()

	-- Execute any queued calls
	local queue = require("hermes.queue")
	if not queue.is_empty() then
		local _, err = queue.execute_all()
		if err then
			logging.notify("Queued operation failed: " .. err, vim.log.levels.ERROR)
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
		logging.notify(string.format("Cleared %d queued operations due to load failure", cleared), vim.log.levels.WARN)
	end

	logging.notify(context .. ". Run :Hermes status for details.", vim.log.levels.ERROR)
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
	logging.notify("Loading binary...", vim.log.levels.INFO)

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
		local ok, result = pcall(binary.load)

		if not ok then
			error("Failed to load or build native binary: " .. tostring(result))
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
		return handle_loading_state(fn)
	end

	if is_failed() then
		return handle_failed_state()
	end

	-- NOT_LOADED: Need to start loading
	local config = require("hermes.config")
	local download_cfg = config.get_download and config.get_download() or { auto = true, timeout = 60 }
	_download_timeout = download_cfg.timeout or 60

	if not should_auto_download() then
		return handle_auto_download_disabled(fn)
	end

	-- Start async download (NOW we show the notification)
	_loading_state = "DOWNLOADING"
	logging.notify("Downloading binary...", vim.log.levels.INFO)

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

	-- Store full configuration locally (used by health check and internal tools)
	require("hermes.config").setup(opts)

	-- 0.12+: suppress cmdline progress output by default (user can re-enable via config)
	if vim.fn.exists("+messagesopt") == 1 then
		local config = require("hermes.config")
		local progress_cfg = config.get_progress()
		if not progress_cfg or not progress_cfg.cmdline then
			-- Remove progress:c to suppress cmdline display of progress messages
			pcall(vim.cmd, "set messagesopt-=progress:c")
		else
			-- User wants cmdline output, ensure progress:c is set
			pcall(vim.cmd, "set messagesopt+=progress:c")
		end
	end

	-- Check if version changed and we need to re-download (only if auto-download is enabled)
	if should_auto_download() then
		local version = require("hermes.version")
		local configured_ver = version.get_wanted()
		local binary = require("hermes.binary")
		local ver_file = binary.get_version_file()
		local bin_path = binary.get_binary_path()
		local active_bin_path = binary.get_active_binary_path()

		if is_ready() and vim.fn.filereadable(ver_file) == 1 then
			local ok, installed_ver = pcall(function()
				return vim.fn.readfile(ver_file)[1]
			end)

			if ok and installed_ver ~= configured_ver then
				-- Only delete the data directory binary if that's where it lives
				-- (don't touch rock tree binaries managed by luarocks)
				if active_bin_path == bin_path then
					pcall(function()
						vim.fn.delete(bin_path)
					end)
				end

				-- Always reset state and delete version file
				pcall(function()
					vim.fn.delete(ver_file)
				end)

				_loading_state = "NOT_LOADED"
				_native = nil
				logging.notify(
					string.format("Version changed from %s to %s, downloading...", installed_ver, configured_ver),
					vim.log.levels.INFO
				)
			end
		end
	end

	-- Execute async with loading state management
	execute_async(function()
		M._load_native_sync().setup(opts)
	end)
end

---Connect to an ACP agent
---@param agent string Agent name, registry agent ID, or path to custom agent
---@param opts? ConnectionOptions Connection options
function M.connect(agent, opts)
	execute_async(function()
		M._load_native_sync().connect(agent, opts)
	end)
end

---Disconnect from agent(s)
---@param agents? string|string[] Agent name(s) to disconnect, nil for all
function M.disconnect(agents)
	execute_async(function()
		M._load_native_sync().disconnect(agents)
	end)
end

---Logout from agent(s)
---@param agents? string|string[] Agent name(s) to logout, nil for all
function M.logout(agents)
	execute_async(function()
		M._load_native_sync().logout(agents)
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
		M._load_native_sync().create_session(opts)
	end)
end

---Load an existing session
---@param session_id string Session ID to load
---@param opts? SessionOptions Session configuration options
function M.load_session(session_id, opts)
	execute_async(function()
		M._load_native_sync().load_session(session_id, opts)
	end)
end

---Resume an existing session
---@param session_id string Session ID to resume
---@param opts? SessionOptions Session configuration options
function M.resume_session(session_id, opts)
	execute_async(function()
		M._load_native_sync().resume_session(session_id, opts)
	end)
end

---List sessions with optional filtering
---@param opts? ListSessionsOptions Filter options
function M.list_sessions(opts)
	execute_async(function()
		M._load_native_sync().list_sessions(opts)
	end)
end

---List agents from the agent registry
---@param opts? AgentsOptions Options (update: boolean to fetch latest registry)
function M.agents(opts)
	execute_async(function()
		M._load_native_sync().agents(opts)
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

---Close active session and free all resources associated with it
---@param session_id string Session ID
function M.close_session(session_id)
	execute_async(function()
		M._load_native_sync().close_session(session_id)
	end)
end

---Delete session(s) from the session list
---@param session_id string|string[] Session ID(s) to delete
---@param opts? {cancel?: boolean} Options
function M.delete_session(session_id, opts)
	execute_async(function()
		M._load_native_sync().delete_session(session_id, opts)
	end)
end

---Cancel an active session's operation, or a pending incoming request by its request ID
---@param id string Session ID, or a Hermes request ID for a pending agent request
function M.cancel(id)
	execute_async(function()
		M._load_native_sync().cancel(id)
	end)
end

---Set session mode
---@param session_id string Session ID
---@param mode string Mode ID/Name
function M.set_mode(session_id, mode)
	execute_async(function()
		M._load_native_sync().set_mode(session_id, mode)
	end)
end

---Set session model
---@param session_id string Session ID
---@param model string Model ID/Name
function M.set_model(session_id, model)
	execute_async(function()
		M._load_native_sync().set_model(session_id, model)
	end)
end

---Set thought level for a session
---@param session_id string Session ID
---@param level string Thought level ID/Name
function M.set_thought_level(session_id, level)
	execute_async(function()
		M._load_native_sync().set_thought_level(session_id, level)
	end)
end

---Get available modes for a session
---@param session_id string Session ID
function M.modes(session_id)
	return execute_async(function()
		return M._load_native_sync().modes(session_id)
	end)
end

---Get available models for a session
---@param session_id string Session ID
function M.models(session_id)
	return execute_async(function()
		return M._load_native_sync().models(session_id)
	end)
end

---Get available thought levels for a session
---@param session_id string Session ID
function M.thought_levels(session_id)
	return execute_async(function()
		return M._load_native_sync().thought_levels(session_id)
	end)
end

---Get available model configuration options for a session
---@param session_id string Session ID
function M.model_configurations(session_id)
	return execute_async(function()
		return M._load_native_sync().model_configurations(session_id)
	end)
end

---Configure a model option for a session
---@param session_id string Session ID
---@param config ConfigureModelConfig Table with `id` and `value` keys
function M.configure_model(session_id, config)
	execute_async(function()
		M._load_native_sync().configure_model(session_id, config)
	end)
end

---Get all configuration options for a session
---@param session_id string Session ID
function M.config_options(session_id)
	return execute_async(function()
		return M._load_native_sync().config_options(session_id)
	end)
end

---Set any configuration option for a session
---@param session_id string Session ID
---@param config SetConfigOptionConfig Table with `id` and `value` keys
function M.set_config_option(session_id, config)
	execute_async(function()
		M._load_native_sync().set_config_option(session_id, config)
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

-- luacov: enable

-- Eagerly load binary at startup if it exists
-- No version check here - we just load whatever binary is present
-- Version validation happens in setup()
local function eager_load_binary()
	if _loading_state ~= "NOT_LOADED" then
		return
	end

	local binary = require("hermes.binary")
	local bin_path = binary.get_active_binary_path()
	local ver_file = binary.get_version_file()

	if vim.fn.filereadable(bin_path) == 1 then
		local ok, result = pcall(function()
			local lib, err = package.loadlib(bin_path, "luaopen_hermes")
			if not lib then
				error("Failed to load: " .. tostring(err))
			end
			return lib()
		end)

		if ok then
			_native = result
			_loading_state = "READY"
			logging.notify("Binary loaded successfully", vim.log.levels.DEBUG)
		else
			-- Failed to load - delete corrupted binary
			pcall(function()
				vim.fn.delete(bin_path)
				vim.fn.delete(ver_file)
			end)
			logging.notify("Removed corrupted binary", vim.log.levels.DEBUG)
		end
	end
end

-- Call eager load immediately
eager_load_binary()

return M

