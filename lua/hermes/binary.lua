-- luacov: disable
---Binary management for Hermes
---@module hermes.binary
-- luacov: enable

local M = {}

-- Repository URL for manual builds
local REPO_URL = "https://github.com/Ruddickmg/hermes.nvim.git"

---Download module (lazy-loaded)
-- luacov: disable
---@type table|nil
-- luacov: enable
local download = nil

---Get download module (lazy-load)
-- luacov: disable
---@return table download_module The download module
-- luacov: enable
local function get_download()
	if not download then
		download = require("hermes.download")
	end
	return download
end

---Supported platforms for pre-built binaries
-- luacov: disable
---@type table<string, boolean>
-- luacov: enable
M.SUPPORTED_PLATFORMS = {
	["linux-x86_64"] = true,
	["linux-aarch64"] = true,
	["macos-x86_64"] = true,
	["macos-aarch64"] = true,
	["windows-x86_64"] = true,
}

-- luacov: disable
---Get the data directory for Hermes
---@return string data_dir Path to data directory
---@private
-- luacov: enable
function M.get_data_dir()
	return vim.fn.stdpath("data") .. "/hermes"
end

-- luacov: disable
---Get the binary name for current platform
---@return string binary_name Name of the binary file
---@private
-- luacov: enable
function M.get_binary_name()
	local platform = require("hermes.platform")
	local os = platform.get_os()
	local arch = platform.get_arch()
	local ext = platform.get_ext()
	return string.format("libhermes-%s-%s.%s", os, arch, ext)
end

-- luacov: disable
---Get the full path to the binary
---@return string binary_path Full path to binary
---@private
-- luacov: enable
function M.get_binary_path()
	return M.get_data_dir() .. "/" .. M.get_binary_name()
end

-- luacov: disable
---Get the version file path
---@return string version_file_path Path to version file
---@private
-- luacov: enable
function M.get_version_file()
	return M.get_data_dir() .. "/version.txt"
end

-- luacov: disable
---Download binary for platform
---@param dest_path string Destination path for binary
---@param ver string Version to download
---@return boolean success Whether download succeeded
---@return table|nil error Error info table if failed (structured error from download module)
---@private
-- luacov: enable
function M.download(dest_path, ver)
	local platform = require("hermes.platform")
	local download_mod = get_download()

	-- Ensure data directory exists
	vim.fn.mkdir(M.get_data_dir(), "p")

	-- Get platform info
	local platform_key = platform.get_platform_key()
	if not platform_key then
		return false,
			{
				message = "Unable to determine platform",
				url = nil,
				http_code = nil,
				tool = nil,
				exit_code = nil,
				stderr = nil,
			}
	end

	-- If version is "latest", fetch the actual latest version
	if ver == "latest" then
		local version = require("hermes.version")
		ver = version.fetch_latest()
	end

	-- Construct download URL
	local url =
		string.format("https://github.com/Ruddickmg/hermes.nvim/releases/download/%s/%s", ver, M.get_binary_name())

	-- Download the binary
	local ok, err = download_mod.download(url, dest_path)

	if not ok then
		-- err is now a structured error table from download module
		return false, err
	end

	-- Make executable (Unix-like systems)
	if vim.fn.has("win32") ~= 1 then
		vim.fn.system({ "chmod", "+x", dest_path })
	end

	return true
end

-- luacov: disable
---Build from source
---Builds from the local source directory where the plugin is installed
---@param dest_dir string Destination directory
---@return boolean success Whether build succeeded
---@private
-- luacov: enable
function M.build_from_source(dest_dir)
	local logging = require("hermes.logging")

	-- Ensure destination directory exists
	vim.fn.mkdir(dest_dir, "p")

	-- Check for required tools (cargo only, no git needed)
	if vim.fn.executable("cargo") ~= 1 then
		logging.notify("Rust/Cargo is required to build from source", vim.log.levels.ERROR)
		return false
	end

	-- Auto-detect source directory from current Lua file location
	-- debug.getinfo(1).source returns "@/path/to/lua/hermes/binary.lua"
	local current_file = debug.getinfo(1).source:sub(2)  -- Remove leading "@"
	-- Go up 3 levels: binary.lua → hermes/ → lua/ → project_root/
	local source_dir = vim.fn.fnamemodify(current_file, ":h:h:h")

	-- Verify this looks like a Hermes source directory
	local cargo_toml = source_dir .. "/Cargo.toml"
	if vim.fn.filereadable(cargo_toml) ~= 1 then
		logging.notify(
			"Could not find Hermes source code at: " .. source_dir .. "\n"
				.. "Expected to find Cargo.toml in that directory",
			vim.log.levels.ERROR
		)
		return false
	end

	logging.notify("Building Hermes from source (this may take a few minutes)...", vim.log.levels.INFO)

	-- Build with cargo from the detected source directory
	local build_cmd = "cd " .. vim.fn.shellescape(source_dir) .. " && cargo build --release"
	local output = vim.fn.system(build_cmd)

	if vim.v.shell_error ~= 0 then
		logging.notify("Cargo build failed:\n" .. output, vim.log.levels.ERROR)
		return false
	end

	-- Copy built binary to destination
	local platform = require("hermes.platform")
	local ext = platform.get_ext()
	local built_lib = source_dir .. "/target/release/libhermes." .. ext
	local dest_lib = dest_dir .. "/" .. M.get_binary_name()

	local uv = vim.uv or vim.loop
	local copy_ok = uv.fs_copyfile(built_lib, dest_lib)

	if not copy_ok then
		logging.notify("Failed to copy built library from " .. built_lib .. " to " .. dest_lib, vim.log.levels.ERROR)
		return false
	end

	-- Write version file to mark this as a source build
	local ver_file = M.get_version_file()
	vim.fn.writefile({ "source" }, ver_file)

	logging.notify("Build successful! Hermes has been built from source.", vim.log.levels.INFO)
	return true
end

-- luacov: disable
---Ensure binary is available (synchronous)
---Downloads binary only if it doesn't exist or version differs from config
---@return string path Path to binary
---@private
-- luacov: enable
function M.ensure_binary()
	local bin_path = M.get_binary_path()
	local ver_file = M.get_version_file()
	local version = require("hermes.version")
	local wanted_ver = version.get_wanted()

	-- Check if binary already exists
	if vim.fn.filereadable(bin_path) == 1 then
		-- Binary exists - check if version matches config
		if vim.fn.filereadable(ver_file) == 1 then
			local current_ver = vim.fn.readfile(ver_file)[1]
			-- If versions match, or it's a source build, use existing binary
			if current_ver == wanted_ver or current_ver == "source" then
				return bin_path
			end
			-- Versions differ - check if it's a source build
			if current_ver == "source" then
				error(
					"A source-built Hermes binary is installed, but the config requests version "
						.. wanted_ver
						.. ".\n\n"
						.. "You have two options:\n\n"
						.. "1. Rebuild from source: :Hermes build\n"
						.. "2. Change your config version to 'source' to use the built binary\n\n"
						.. "Current config:\n"
						.. "  download = { version = '"
						.. wanted_ver
						.. "' }\n\n"
						.. "To use source builds:\n"
						.. "  download = { version = 'source' }"
				)
			end
			-- Versions differ - need to download new version
		end
		-- No version file or version mismatch - will download new version
	end

	-- Binary doesn't exist or version differs - need to download
	local platform = require("hermes.platform")

	-- Check if platform is supported for pre-built binaries
	local platform_key = platform.get_platform_key()
	if not platform_key then
		error(
			"Unable to determine platform.\n\n"
				.. "Please check the installation instructions:\n"
				.. "https://github.com/Ruddickmg/hermes.nvim#installation"
		)
	end

	if not M.SUPPORTED_PLATFORMS[platform_key] then
		local supported_list = {}
		for plat, _ in pairs(M.SUPPORTED_PLATFORMS) do
			table.insert(supported_list, "  - " .. plat:gsub("-", " "):gsub("^%l", string.upper))
		end
		table.sort(supported_list)

		error(
			string.format(
				"Platform not supported for automatic binary download: %s\n\n"
					.. "Pre-built binaries are available for these platforms:\n%s\n\n"
					.. "To use Hermes on your platform, you have two options:\n\n"
					.. "Option 1 - Build manually (Recommended):\n"
					.. "  1. Install Rust: https://rustup.rs/\n"
					.. "  2. Run :Hermes build inside Neovim\n\n"
					.. "Option 2 - Build outside Neovim:\n"
					.. "  1. Clone: git clone %s\n"
					.. "  2. Build: cargo build --release\n"
					.. "  3. Copy target/release/libhermes.* to %s\n\n"
					.. "For detailed instructions, see:\n"
					.. "https://github.com/Ruddickmg/hermes.nvim#installation",
				platform.get_display_string(),
				table.concat(supported_list, "\n"),
				REPO_URL,
				M.get_data_dir()
			)
		)
	end

	-- Check if download tools are available
	local download_mod = get_download()
	local download_tool = download_mod.get_available_tool()
	if not download_tool then
		error(
			"Unable to download Hermes binary.\n\n"
				.. "No download tool found. Please install one of the following:\n"
				.. "  - curl (preferred)\n"
				.. "  - wget\n\n"
				.. "Alternatively, you can build from source:\n"
				.. "  1. Install Rust: https://rustup.rs/\n"
				.. "  2. Run :Hermes build inside Neovim\n\n"
				.. "For detailed instructions, see:\n"
				.. "https://github.com/Ruddickmg/hermes.nvim#installation"
		)
	end

	-- Download binary for supported platform
	local download_ok = M.download(bin_path, wanted_ver)

	if not download_ok then
		-- Download failed on a supposedly supported platform
		error(
			string.format(
				"Failed to download Hermes binary for %s.\n\n"
					.. "This is unexpected for a supported platform.\n\n"
					.. "Troubleshooting steps:\n"
					.. "  1. Check your internet connection\n"
					.. "  2. Check if GitHub is accessible\n"
					.. "  3. The release may not exist yet for version %s\n\n"
					.. "To build manually:\n"
					.. "  1. Install Rust: https://rustup.rs/\n"
					.. "  2. Run :Hermes build inside Neovim\n\n"
					.. "For detailed instructions, see:\n"
					.. "https://github.com/Ruddickmg/hermes.nvim#installation",
				platform.get_display_string(),
				wanted_ver
			)
		)
	end

	-- Save version for reference
	vim.fn.writefile({ wanted_ver }, ver_file)

	return bin_path
end

-- luacov: disable
---Load existing binary without downloading
---Checks if binary exists at expected path, errors if not found
---@return string path Path to existing binary
---@private
-- luacov: enable
function M.load_existing_binary()
	local bin_path = M.get_binary_path()

	-- Check if binary already exists
	if vim.fn.filereadable(bin_path) == 0 then
		local platform = require("hermes.platform")
		error(
			string.format(
				"Binary not found and download.auto is disabled.\n\n"
					.. "Current platform: %s\n\n"
					.. "To resolve this, choose one option:\n\n"
					.. "Option 1 - Enable auto-download in your config:\n"
					.. '  require("hermes").setup({\n'
					.. "    download = {\n"
					.. "      auto = true,\n"
					.. "    },\n"
					.. "  })\n\n"
					.. "Option 2 - Build manually:\n"
					.. "  1. Install Rust: https://rustup.rs/\n"
					.. "  2. Run :Hermes build inside Neovim\n\n"
					.. "For detailed instructions, see:\n"
					.. "https://github.com/Ruddickmg/hermes.nvim#installation",
				platform.get_display_string()
			)
		)
	end

	return bin_path
end

-- luacov: disable
---Load native module
---Ensures binary is available and loads it
---@return table native_module The loaded native module
---@private
-- luacov: enable
function M.load()
	local bin_path = M.ensure_binary()

	local lib, err = package.loadlib(bin_path, "luaopen_hermes")
	if not lib then
		error(string.format("Failed to load native module from: %s\nError: %s", bin_path, tostring(err)))
	end

	return lib()
end

-- luacov: disable
---Ensure binary is available asynchronously
---Downloads binary if needed, then calls on_complete with the binary path
---@param timeout number Timeout in seconds
---@param on_complete function Callback function(success: boolean, result: string)
---@private
-- luacov: enable
function M.ensure_binary_async(timeout, on_complete)
	timeout = timeout or 60

	local platform = require("hermes.platform")
	local version = require("hermes.version")

	-- Check if platform is supported
	local platform_key = platform.get_platform_key()
	if not platform_key then
		on_complete(false, "Unable to determine platform")
		return
	end

	if not M.SUPPORTED_PLATFORMS[platform_key] then
		on_complete(
			false,
			"Platform not supported for automatic binary download: "
				.. platform.get_display_string()
				.. ". Consider building from source."
		)
		return
	end

	-- Check if download tools are available
	local download_mod = get_download()
	local download_tool = download_mod.get_available_tool()
	if not download_tool then
		on_complete(false, "No download tool available. Please install curl, wget, or PowerShell.")
		return
	end

	-- Use vim.schedule to make the entire process async
	vim.schedule(function()
		local bin_path = M.get_binary_path()
		local ver_file = M.get_version_file()
		local wanted_ver = version.get_wanted()

		-- Check if binary already exists
		if vim.fn.filereadable(bin_path) == 1 then
			-- Binary exists - check if version matches config
			if vim.fn.filereadable(ver_file) == 1 then
				local current_ver = vim.fn.readfile(ver_file)[1]
				-- If versions match, use existing binary
				if current_ver == wanted_ver then
					on_complete(true, bin_path)
					return
				end
				-- Versions differ - will download new version
			end
			-- No version file or version mismatch - will download
		end

		-- Binary doesn't exist or version differs, need to download
		local download_ok, download_err = M.download(bin_path, wanted_ver)

		if download_ok then
			-- Save version for reference
			vim.fn.writefile({ wanted_ver }, ver_file)
			on_complete(true, bin_path)
		else
			on_complete(false, download_err or "Download failed")
		end
	end)
end

return M
