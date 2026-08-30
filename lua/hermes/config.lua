-- luacov: disable
---Configuration management for Hermes
---@module hermes.config
---Stores the full user configuration with defaults matching the Rust binary.
-- luacov: enable

local M = {}

-- luacov: disable
---@class HermesDownloadConfig
---Download configuration for binary management
---@field version? string Version to use ("latest" or specific version like "v0.1.0")
---@field auto? boolean Whether to auto-download binary (default: true)
---@field timeout? number Download timeout in seconds (default: 60)

---@class HermesLogTargetConfig
---@field level? number|string Log level (default: "off")
---@field format? string Log format (default: "compact")
---@field show_ansi? boolean Show ANSI color codes in output (default: false)

---@class HermesLogFileConfig
---@field level? number|string Log level (default: "off")
---@field format? string Log format (default: "json")
---@field path? string Path to log directory (default: vim.fn.stdpath("state") .. "/hermes")
---@field name? string Log file name (default: "hermes.log")
---@field show_ansi? boolean Show ANSI color codes in output (default: false)
---@field max_size? number Maximum file size in bytes (default: 10485760 = 10MB)
---@field max_files? number Maximum number of log files to keep (default: 5)

---@class HermesLogConfig
---@field stdio? HermesLogTargetConfig Stdio logging settings
---@field notification? HermesLogTargetConfig Notification logging settings
---@field message? HermesLogTargetConfig Message logging settings
---@field file? HermesLogFileConfig File logging settings

---@class HermesPermissionsConfig
---@field fs_write_access? boolean Allow agent to write files (default: true)
---@field fs_read_access? boolean Allow agent to read files (default: true)
---@field terminal_access? boolean Allow agent to execute terminal commands (default: true)
---@field request_permissions? boolean Allow agent to send permission requests (default: true)
---@field send_notifications? boolean Allow agent to send notifications (default: true)
---@field elicitation_form? boolean Allow agent to send form elicitation requests (default: true)
---@field elicitation_url? boolean Allow agent to send URL elicitation requests (default: true)

---@class HermesTerminalConfig
---@field delete? boolean Auto-delete terminals on exit (default: false)
---@field enabled? boolean Enable terminal functionality (default: true)
---@field hidden? boolean Hide terminal windows (default: true)
---@field buffered? boolean Buffer terminal output (default: true)

---@class HermesBufferConfig
---@field auto_save? boolean Auto-save modified files after writing (default: false)

---@class HermesSessionConfig
---@field store_history? boolean Store session history locally (default: true)

---@class HermesDistributionsBinaryConfig
---@field enabled? boolean Enable binary distribution (default: true)
---@field path? string Custom cache directory (default: "")

---@class HermesDistributionsConfig
---@field binary? HermesDistributionsBinaryConfig Binary distribution settings
---@field uvx? boolean Enable uvx distribution (default: true)
---@field npx? boolean Enable npx distribution (default: true)

---@class HermesProgressConfig
---@field cmdline? boolean Show progress messages in cmdline (0.12+ only, default: false)
---@field update_frequency? integer How often to poll for progress in milliseconds (default: 150)

---@class HermesConfig
---Full Hermes configuration
---@field download? HermesDownloadConfig Download configuration
---@field log? HermesLogConfig Logging configuration
---@field permissions? HermesPermissionsConfig Permission settings
---@field terminal? HermesTerminalConfig Terminal configuration
---@field buffer? HermesBufferConfig Buffer configuration
---@field session? HermesSessionConfig Session configuration
---@field distributions? HermesDistributionsConfig Distribution settings
---@field progress? HermesProgressConfig Progress display configuration
---@field root_markers? string[] Files/directories to identify project root (default: {".git"})

-- luacov: disable
---Deep merge a user table into a base table (shallow for leaf values)
---@param base table The base configuration table
---@param user table The user-provided overrides
---@return table The merged configuration
-- luacov: enable
local function merge_config(base, user)
	local result = {}
	for k, v in pairs(base) do
		if type(v) == "table" then
			if user[k] ~= nil and type(user[k]) == "table" then
				result[k] = merge_config(v, user[k])
			else
				result[k] = merge_config(v, {})
			end
		else
			if user[k] ~= nil then
				result[k] = user[k]
			else
				result[k] = v
			end
		end
	end
	-- Add any user keys not in base
	for k, v in pairs(user) do
		if base[k] == nil then
			result[k] = v
		end
	end
	return result
end

-- luacov: disable
---Default configuration values matching Rust defaults
---@type HermesConfig
-- luacov: enable
local default_config = {
	download = {
		version = "latest",
		auto = true,
		timeout = 60,
	},
	log = {
		stdio = {
			level = "off",
			format = "compact",
			show_ansi = false,
		},
		notification = {
			level = "error",
			format = "compact",
			show_ansi = false,
		},
		message = {
			level = "off",
			format = "compact",
			show_ansi = false,
		},
		file = {
			level = "off",
			format = "json",
			path = vim.fn.stdpath("state") .. "/hermes",
			name = "hermes.log",
			show_ansi = false,
			max_size = 10485760,
			max_files = 5,
		},
	},
	permissions = {
		fs_write_access = true,
		fs_read_access = true,
		terminal_access = true,
		request_permissions = true,
		send_notifications = true,
		elicitation_form = true,
		elicitation_url = true,
	},
	terminal = {
		delete = false,
		enabled = true,
		hidden = true,
		buffered = true,
	},
	buffer = {
		auto_save = false,
	},
	session = {
		store_history = true,
	},
	distributions = {
		binary = {
			enabled = true,
			path = "",
		},
		uvx = true,
		npx = true,
	},
	progress = {
		cmdline = false,
		update_frequency = 150,
	},
	root_markers = { ".git" },
}

---@type HermesConfig
-- luacov: enable
local _config = default_config

-- luacov: disable
---Setup hermes configuration with user overrides
---@param opts? HermesConfig User configuration options
-- luacov: enable
function M.setup(opts)
	opts = opts or {}
	local defaults = vim.deepcopy(default_config)
	if opts.download == nil or opts.download.auto == nil then
		if require("hermes.binary").is_nix_install() then
			defaults.download.auto = false
		end
	end
	_config = merge_config(defaults, opts)
end

-- luacov: disable
---Get current full configuration
---@return HermesConfig Current configuration
-- luacov: enable
function M.get()
	return _config
end

-- luacov: disable
---Get download configuration
---@return HermesDownloadConfig Download configuration
---@private
-- luacov: enable
function M.get_download()
	return _config.download
end

-- luacov: disable
---Get binary version setting
---@return string Binary version to use
---@private
-- luacov: enable
function M.get_version()
	return _config.download.version
end

-- luacov: disable
---Get auto download setting
---@return boolean Whether to auto-download binary
---@private
-- luacov: enable
function M.get_auto_download()
	return _config.download.auto ~= false
end

-- luacov: disable
---Get download timeout
---@return number Download timeout in seconds
---@private
-- luacov: enable
function M.get_download_timeout()
	return _config.download.timeout
end

-- luacov: disable
---Get notification log level for vim.notify filtering
---@return number|string Log level
---@private
-- luacov: enable
function M.get_notification_level()
	return _config.log.notification.level
end

-- luacov: disable
---Get progress configuration
---@return HermesProgressConfig Progress configuration
---@private
-- luacov: enable
function M.get_progress()
	return _config.progress or { cmdline = false, update_frequency = 150 }
end

-- luacov: disable
---Get log file path
---@return string Full path to log file
---@private
-- luacov: enable
function M.get_log_file_path()
	return _config.log.file.path .. "/" .. _config.log.file.name
end

-- luacov: disable
---Get registry binary cache directory
---@return string Path to agent binary cache
---@private
-- luacov: enable
function M.get_registry_cache_dir()
	local dist_config = _config.distributions and _config.distributions.binary
	if dist_config and dist_config.path and dist_config.path ~= "" then
		return dist_config.path .. "/hermes/agents"
	end
	local data_home = os.getenv("XDG_DATA_HOME")
	if not data_home then
		local home = os.getenv("HOME") or "."
		data_home = home .. "/.local/share"
	end
	return data_home .. "/hermes/agents"
end

return M
