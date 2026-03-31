---Plugin startup script - auto-sourced by Neovim
---Commands and initialization

-- Version check
if vim.fn.has("nvim-0.11") ~= 1 then
	vim.api.nvim_err_writeln("Hermes requires Neovim >= 0.11")
	return
end

-- Create user command (single source of truth - defined here in plugin script)
vim.api.nvim_create_user_command("Hermes", function(args)
	local subcmd = args.fargs[1]

	if subcmd == "status" then
		-- Show Hermes loading status
		local hermes = require("hermes")
		local state = hermes.get_loading_state()
		local error_msg = hermes.get_loading_error()
		local config = require("hermes.config")
		local download_cfg = config.get_download()

		local status_lines = {
			"Hermes Status",
			"=============",
			"",
			"State: " .. state,
		}

		if state == "NOT_LOADED" then
			table.insert(
				status_lines,
				"The binary has not been loaded yet. Run any Hermes API method to start loading."
			)
		elseif state == "DOWNLOADING" then
			table.insert(status_lines, "The binary is being downloaded...")
			table.insert(status_lines, "Timeout: " .. tostring(download_cfg.timeout or 60) .. " seconds")
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
	elseif subcmd == "log" or subcmd == "logs" then
		-- Show recent log messages
		local hermes = require("hermes")
		local state = hermes.get_loading_state()
		local error_msg = hermes.get_loading_error()

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
	elseif subcmd == "install" or subcmd == "download" then
		-- Force download/install
		vim.notify("Installing Hermes binary...", vim.log.levels.INFO)
		local ok, err = pcall(function()
			local binary = require("hermes.binary")
			local version = require("hermes.version")

			local ver = version.get_wanted()

			local path = binary.get_binary_path()
			-- Remove existing binary if present
			if vim.fn.filereadable(path) == 1 then
				vim.fn.delete(path)
			end

			-- Download fresh
			local success = binary.download(path, ver)
			if not success then
				error("Download failed")
			end

			-- Save version
			vim.fn.writefile({ ver }, binary.get_version_file())
		end)

		if ok then
			vim.notify("Hermes binary installed successfully!", vim.log.levels.INFO)
		else
			vim.notify("Installation failed: " .. tostring(err), vim.log.levels.ERROR)
		end
	elseif subcmd == "update" then
		-- Update to latest version (fetches from GitHub and downloads)
		vim.notify("Updating Hermes binary...", vim.log.levels.INFO)
		local ok, err = pcall(function()
			local binary = require("hermes.binary")
			local version = require("hermes.version")

			-- Fetch latest version from GitHub
			local latest_ver = version.fetch_latest()
			vim.notify("Latest version: " .. latest_ver, vim.log.levels.INFO)

			local path = binary.get_binary_path()
			-- Remove existing binary
			if vim.fn.filereadable(path) == 1 then
				vim.fn.delete(path)
			end

			-- Download latest version
			local success = binary.download(path, latest_ver)
			if not success then
				error("Download failed")
			end

			-- Save version
			vim.fn.writefile({ latest_ver }, binary.get_version_file())

			return latest_ver
		end)

		if ok then
			vim.notify("Hermes updated to version " .. err .. " successfully!", vim.log.levels.INFO)
		else
			vim.notify("Update failed: " .. tostring(err), vim.log.levels.ERROR)
		end
	elseif subcmd == "build" then
		-- Force build from source
		vim.notify("Building Hermes from source...", vim.log.levels.INFO)
		local ok, err = pcall(function()
			local binary = require("hermes.binary")
			local data_dir = binary.get_data_dir()

			-- Remove existing binary
			local path = binary.get_binary_path()
			if vim.fn.filereadable(path) == 1 then
				vim.fn.delete(path)
			end

			local success = binary.build_from_source(data_dir)
			if not success then
				error("Build failed")
			end

			-- Save version as "built"
			vim.fn.writefile({ "built" }, binary.get_version_file())
		end)

		if ok then
			vim.notify("Hermes built successfully!", vim.log.levels.INFO)
		else
			vim.notify("Build failed: " .. tostring(err), vim.log.levels.ERROR)
		end
	elseif subcmd == "version" or subcmd == "info" then
		-- Show version info
		local platform = require("hermes.platform")
		local version = require("hermes.version")
		require("hermes.config")

		local wanted = version.get_wanted()
		local platform_str = platform.get_display_string()

		print("Hermes Version Information:")
		print("  Wanted version: " .. wanted)
		print("  Platform: " .. platform_str)

		-- Check if binary exists
		local binary = require("hermes.binary")
		local bin_path = binary.get_binary_path()
		local ver_file = binary.get_version_file()

		if vim.fn.filereadable(bin_path) == 1 then
			print("  Binary: installed")
			if vim.fn.filereadable(ver_file) == 1 then
				local current = vim.fn.readfile(ver_file)[1]
				print("  Current version: " .. current)
			else
				print("  Current version: unknown")
			end
		else
			print("  Binary: not installed (will download on first use)")
		end
	elseif subcmd == "clean" then
		-- Clear binary
		vim.notify("Cleaning Hermes installation...", vim.log.levels.INFO)
		local binary = require("hermes.binary")
		local data_dir = binary.get_data_dir()

		-- Remove data directory
		if vim.fn.isdirectory(data_dir) == 1 then
			vim.fn.delete(data_dir, "rf")
		end

		vim.notify("Hermes cleaned successfully!", vim.log.levels.INFO)
	elseif subcmd == "setup" or subcmd == "config" then
		-- Show current configuration
		local config = require("hermes.config")
		local current = config.get()

		print("Hermes Configuration:")
		print(vim.inspect(current))
	else
		vim.notify(
			"Usage: :Hermes {status|log|install|update|build|version|clean|setup}\n\n"
				.. "Commands:\n"
				.. "  status   - Show loading status and configuration\n"
				.. "  log      - Show recent log messages\n"
				.. "  install  - Download and install the binary\n"
				.. "  update   - Update to the latest version from GitHub\n"
				.. "  build    - Build binary from source\n"
				.. "  version  - Show version information\n"
				.. "  clean    - Remove binary\n"
				.. "  setup    - Show current configuration",
			vim.log.levels.INFO
		)
	end
end, {
	nargs = "?",
	complete = function()
		return { "status", "log", "install", "update", "build", "version", "clean", "setup" }
	end,
	desc = "Hermes binary management and info",
})

-- Create highlight group for hermes notifications (optional)
vim.api.nvim_set_hl(0, "HermesInfo", { link = "DiagnosticInfo" })
vim.api.nvim_set_hl(0, "HermesWarning", { link = "DiagnosticWarn" })
vim.api.nvim_set_hl(0, "HermesError", { link = "DiagnosticError" })

-- Lazy-load on first API call - no eager initialization
-- The binary is only downloaded/built when user calls require("hermes").api_method()
