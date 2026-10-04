---Plugin startup script - auto-sourced by Neovim
---Commands and initialization
local logger = require("hermes.logging")

-- Version check
local nvim_ver = vim.version()
if nvim_ver.major == 0 and nvim_ver.minor < 11 then
	vim.api.nvim_err_writeln("Hermes requires Neovim >= 0.11")
	return
end

-- Create user command (single source of truth - defined here in plugin script)
vim.api.nvim_create_user_command("Hermes", function(args)
	local subcmd = args.fargs[1]

	if subcmd == "log" or subcmd == "logs" then
		-- Open the most recent log file in a new tab
		local config = require("hermes.config")
		local file_config = config.get().log.file
		local log_path = config.get_log_file_path()

		-- The base path is the active log file; rotated backups are named
		-- "<name>.1", "<name>.2", ... Fall back to the most recent backup if
		-- the base file does not exist.
		local path
		if vim.fn.filereadable(log_path) == 1 then
			path = log_path
		else
			for rotation = 1, file_config.max_files or 5 do
				local rotated = log_path .. "." .. rotation
				if vim.fn.filereadable(rotated) == 1 then
					path = rotated
					break
				end
			end
		end

		if not path then
			local hermes = require("hermes")
			local lines = {
				"No log file found at " .. log_path,
				"",
				"File logging is configured with level '" .. tostring(file_config.level) .. "'.",
				"Enable it with: require('hermes').setup({ log = { file = { level = 'info' } } })",
				"",
				"Current State: " .. hermes.get_loading_state(),
			}
			local error_msg = hermes.get_loading_error()
			if error_msg then
				table.insert(lines, "Last Error: " .. error_msg)
			end
			-- Explicitly invoked command: notify directly, bypassing the
			-- notification level filter so there is always a response.
			vim.notify(table.concat(lines, "\n"), vim.log.levels.INFO, { title = "Hermes" })
			return
		end

		vim.cmd("tabnew " .. vim.fn.fnameescape(path))
		vim.bo.readonly = true
		vim.bo.modifiable = false

		-- JSON logs are newline-delimited (one object per line)
		if file_config.format == "json" then
			vim.bo.filetype = "jsonl"
		end

		-- Tail behavior: land on the last line
		vim.cmd("normal! G")
	elseif subcmd == "install" or subcmd == "download" then
		-- Force download/install (async, non-blocking)
		logger.notify("Installing Hermes binary...", vim.log.levels.INFO)
		local binary = require("hermes.binary")
		local version = require("hermes.version")

		local ver = version.get_wanted()
		local path = binary.get_binary_path()

		-- Remove existing binary if present
		if vim.fn.filereadable(path) == 1 then
			vim.fn.delete(path)
		end

		local function on_download_done(success, result)
			if success then
				logger.notify("Hermes binary installed successfully!", vim.log.levels.INFO)
			else
				logger.notify("Installation failed: " .. ((type(result) == "table" and result.message) or tostring(result)), vim.log.levels.ERROR)
			end
		end

		if ver == "latest" then
			version.fetch_latest_async(function(tag, _err)
				if not tag then
					logger.notify("Failed to fetch latest version", vim.log.levels.ERROR)
					return
				end
				logger.notify("Latest version: " .. tag, vim.log.levels.INFO)
				binary.download_async(tag, on_download_done)
			end)
		else
			binary.download_async(ver, on_download_done)
		end
	elseif subcmd == "update" then
		-- Update to latest version (async, non-blocking)
		logger.notify("Updating Hermes binary...", vim.log.levels.INFO)
		local binary = require("hermes.binary")
		local version = require("hermes.version")

		local path = binary.get_binary_path()
		-- Remove existing binary
		if vim.fn.filereadable(path) == 1 then
			vim.fn.delete(path)
		end

		version.fetch_latest_async(function(tag, _err)
			if not tag then
				logger.notify("Failed to fetch latest version", vim.log.levels.ERROR)
				return
			end
			logger.notify("Latest version: " .. tag, vim.log.levels.INFO)
			binary.download_async(tag, function(success, result)
				if success then
					logger.notify("Hermes updated to version " .. tag .. " successfully!", vim.log.levels.INFO)
				else
					logger.notify("Update failed: " .. ((type(result) == "table" and result.message) or tostring(result)), vim.log.levels.ERROR)
				end
			end)
		end)
	elseif subcmd == "build" then
		-- Build from source asynchronously (non-blocking)
		-- Extra args are passed through as Cargo features, e.g.:
		--   :Hermes build with-icons   ->  cargo build --release --features with-icons
		local binary = require("hermes.binary")
		local data_dir = binary.get_data_dir()
		local features = vim.list_slice(args.fargs, 2)
		binary.build_from_source_async(data_dir, features, function(success, err)
			if success then
				logger.notify("Hermes built from source successfully!", vim.log.levels.DEBUG)

				-- Reset state and load immediately so no restart is needed
				local hermes = require("hermes")
				local logging = require("hermes.logging")

				-- Reset state to force fresh load
				hermes._set_loading_state("LOADING")
				hermes._set_loading_error(nil)

				-- Load the binary in next event loop tick
				vim.schedule(function()
					local ok, loaded = pcall(hermes._load_native_sync)

					if not ok then
						-- Load failed
						logging.notify(
							"Build succeeded but failed to load binary: " .. tostring(loaded),
							vim.log.levels.ERROR
						)
						return
					end

					-- Success! Use the handle_load_success function to properly set _native and state
					hermes._handle_load_success(loaded, function()
						logging.notify("Hermes is ready to use!", vim.log.levels.DEBUG)
					end)
				end)
			else
				logger.notify("Build failed: " .. tostring(err), vim.log.levels.ERROR)
			end
		end)
	elseif subcmd == "cancel" then
		-- Cancel an in-progress build or download
		local binary = require("hermes.binary")
		local cancelled = binary.cancel_build()
		if not cancelled then
			binary.cancel_download()
		end
	elseif subcmd == "clean" then
		-- Clear binary
		logger.notify("Cleaning Hermes installation...", vim.log.levels.DEBUG)
		local binary = require("hermes.binary")
		local data_dir = binary.get_data_dir()

		-- Remove data directory
		if vim.fn.isdirectory(data_dir) == 1 then
			vim.fn.delete(data_dir, "rf")
		end

		-- Reset internal state so Hermes knows the binary is gone
		local hermes = require("hermes")
		hermes._set_loading_state("NOT_LOADED")
		hermes._set_loading_error(nil)

		logger.notify("Hermes cleaned successfully!", vim.log.levels.INFO)
	else
		logger.notify(
			"Usage: :Hermes {log|install|update|build|cancel|clean}\n\n"
				.. "Commands:\n"
				.. "  log      - Show recent log messages\n"
				.. "  install  - Download and install the binary\n"
				.. "  update   - Update to the latest version from GitHub\n"
				.. "  build    - Build binary from source\n"
				.. "  cancel   - Cancel an in-progress build\n"
				.. "  clean    - Remove binary\n\n"
				.. "Use :checkhealth hermes for detailed status and diagnostics",
			vim.log.levels.INFO
		)
	end
end, {
	nargs = "*",
	complete = function(_, cmdline, _)
		local parts = vim.split(cmdline, "%s+", { trimempty = true })
		if #parts >= 3 or cmdline:match("%s$") then
			if parts[2] == "build" then
				return { "with-icons" }
			end
			return {}
		end
		return { "log", "install", "update", "build", "cancel", "clean" }
	end,
	desc = "Hermes binary management and info",
})

-- Create highlight group for hermes notifications (optional)
vim.api.nvim_set_hl(0, "HermesInfo", { link = "DiagnosticInfo" })
vim.api.nvim_set_hl(0, "HermesWarning", { link = "DiagnosticWarn" })
vim.api.nvim_set_hl(0, "HermesError", { link = "DiagnosticError" })

-- Lazy-load on first API call - no eager initialization
-- The binary is only downloaded/built when user calls require("hermes").api_method()
