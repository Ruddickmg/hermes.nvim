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

---Build state tracking for async operations
-- luacov: disable
---@type boolean
-- luacov: enable
local _build_in_progress = false

---@type vim.SystemObj|nil
-- luacov: enable
local _build_job = nil

---Download state tracking for async operations
-- luacov: disable
---@type boolean
-- luacov: enable
local _download_in_progress = false

---@type number|nil
-- luacov: enable
local _download_job = nil

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

---Compute SHA256 hex hash of a file using Neovim built-ins
---@param file_path string Path to file
---@return string|nil hash Hex-encoded SHA256 hash, or nil on error
---@private
local function _compute_file_hash(file_path)
	local uv = vim.uv or vim.loop

	local fd = uv.fs_open(file_path, "r", 438) -- 0666
	if not fd then
		return nil
	end

	local stat = uv.fs_fstat(fd)
	if not stat or type(stat.size) ~= "number" then
		uv.fs_close(fd)
		return nil
	end

	local data = uv.fs_read(fd, stat.size, 0)
	uv.fs_close(fd)
	if type(data) ~= "string" then
		return nil
	end

	local ok, hash = pcall(vim.fn.sha256, data)
	if not ok or type(hash) ~= "string" then
		return nil
	end

	return hash:lower()
end

M._compute_file_hash = _compute_file_hash

---Parse checksums.txt content and find hash for a filename
---@param content string Contents of checksums.txt
---@param filename string Binary filename to find
---@return string|nil hash Hex hash for the filename, or nil if not found
---@private
local function _parse_checksums(content, filename)
	for line in content:gmatch("[^\r\n]+") do
		local hash, name = line:match("^([%x]+)%s+(.+)$")
		if hash and name == filename then
			return hash
		end
	end
	return nil
end

M._parse_checksums = _parse_checksums

---Verify binary hash against checksums.txt from the same release
---@param bin_path string Path to downloaded binary
---@param ver string Version string (e.g. "v0.10.1")
---@return boolean success Whether verification passed
---@return string|nil error Error message if verification failed
---@private
function M._verify_binary_hash(bin_path, ver)
	local download_mod = get_download()
	local binary_name = require("hermes.platform").get_binary_name()
	local checksums_url = string.format(
		"https://github.com/Ruddickmg/hermes.nvim/releases/download/%s/checksums.txt",
		ver
	)

	-- Download checksums.txt to a temp file
	local tmp_checksums = bin_path .. ".checksums"
	local ok, err = download_mod.download(checksums_url, tmp_checksums)
	if not ok then
		pcall(os.remove, tmp_checksums)

		local err_msg
		if type(err) == "table" then
			err_msg = err.message or err.stderr
		else
			err_msg = err
		end
		err_msg = tostring(err_msg)

		-- If checksums.txt doesn't exist (old release), warn but don't fail
		if (type(err) == "table" and err.http_code == 404) or err_msg:find("404") then
			vim.notify(
				"[hermes] checksums.txt not found for " .. ver .. ", skipping verification",
				vim.log.levels.WARN
			)
			return true
		end
		return false, "Failed to download checksums: " .. err_msg
	end

	-- Read checksums.txt
	local f = io.open(tmp_checksums, "r")
	if not f then
		os.remove(tmp_checksums)
		return false, "Failed to read checksums file"
	end
	local content = f:read("*a")
	f:close()
	os.remove(tmp_checksums)

	-- Find expected hash
	local expected = _parse_checksums(content, binary_name)
	if not expected then
		vim.notify(
			"[hermes] No hash found for " .. binary_name .. " in checksums.txt, skipping verification",
			vim.log.levels.WARN
		)
		return true
	end

	-- Compute actual hash
	local actual = _compute_file_hash(bin_path)
	if not actual then
		pcall(os.remove, bin_path)
		return false, "Failed to compute hash of downloaded binary"
	end

	-- Compare
	if actual ~= expected then
		os.remove(bin_path)
		return false, string.format(
			"Hash mismatch for %s: expected %s, got %s",
			binary_name, expected, actual
		)
	end

	return true
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
---Get the rock tree root from the current Lua file location
---Extracts the LuaRocks tree root by navigating up from the module file path
---@return string|nil Path to the rock tree root, or nil if undetectable
---@private
-- luacov: enable
function M._get_rock_root()
  local info = debug.getinfo(1)
  if not info or type(info.source) ~= "string" then
    return nil
  end
  local source = info.source:sub(2)
  if source == "" then
    return nil
  end
  return vim.fn.fnamemodify(source, ":p:h:h:h")
end

-- luacov: disable
---Check if the plugin is installed via LuaRocks
---Detects LuaRocks install by checking if the module source path contains "/luarocks/"
---@return boolean true if installed via LuaRocks
-- luacov: enable
function M.is_luarocks_install()
  local info = debug.getinfo(1)
  if not info or type(info.source) ~= "string" then
    return false
  end
  local source = info.source:sub(2)
  if source == "" then
    return false
  end
  return source:find("[/\\]luarocks[/\\]") ~= nil or source:find("[/\\]%.luarocks[/\\]") ~= nil
end

-- luacov: disable
---Check if the plugin is running in a Nix environment
---Detects NixOS or a Nix install via the module source path
---@return boolean true if in a Nix environment
-- luacov: enable
function M.is_nix_install()
  local uv = vim.uv or vim.loop
  if uv.fs_stat("/etc/NIXOS") ~= nil then
    return true
  end
  local info = debug.getinfo(1)
  if not info or type(info.source) ~= "string" then
    return false
  end
  local source = info.source:sub(2)
  if source == "" then
    return false
  end
  return source:find("[/\\]nix[/\\]store[/\\]") ~= nil
end

-- luacov: disable
---Get candidate library paths within an installation root
---Includes both the platform-specific release name and the plain cargo
---output name used by from-source builds that do not rename the library
---@param root string Installation root directory containing a lib/ directory
---@return string[] paths Candidate absolute paths, most preferred first
-- luacov: enable
function M.get_lib_candidates(root)
	local platform = require("hermes.platform")
	local ext = platform.get_ext()
	return {
		vim.fs.joinpath(root, "lib", M.get_binary_name()),
		vim.fs.joinpath(root, "lib", "libhermes." .. ext),
	}
end

-- luacov: disable
---Get path to binary installed alongside Lua files in the rock tree
---Checks if the plugin was installed via luarocks and a pre-built binary
---is available in the rock's lib/ directory. Falls back to the plain cargo
---output name so builds packaged without renaming (e.g. Nix) are detected
---@return string|nil Path to rock tree binary, or nil if not found
---@private
-- luacov: enable
function M.get_rock_binary_path()
  local rock_root = M._get_rock_root()
  if not rock_root then
    return nil
  end
	for _, path in ipairs(M.get_lib_candidates(rock_root)) do
		if vim.fn.filereadable(path) == 1 then
			return path
		end
	end
	return nil
end

-- luacov: disable
---Get the version of a LuaRocks-installed plugin from the rockspec filename
---Navigates the LuaRocks tree to find the installed rockspec and extracts the version
---@return string|nil Version string (e.g. "v0.1.0") or nil if not found
---@private
-- luacov: enable
function M._get_rock_version()
  local rock_root = M._get_rock_root()
  if not rock_root then
    return nil
  end
  local rockspec_pattern = rock_root .. "/hermes.nvim-*.rockspec"
  local files = vim.fn.glob(rockspec_pattern, false, true)
  if #files == 0 then
    return nil
  end
  local filename = vim.fn.fnamemodify(files[#files], ":t")
  local version = filename:match("hermes%.nvim%-(.+)%.rockspec$")
  if not version then
    return nil
  end
  -- Strip LuaRocks revision suffix (e.g., "0.1.0-1" → "0.1.0")
  version = version:gsub("%-%d+$", "")
  if not version:match("^v") then
    version = "v" .. version
  end
  return version
end

-- luacov: disable
---Get the active path to the binary
---Returns the rock tree path if a rock-installed binary exists,
---otherwise falls back to the standard data directory path
---@return string binary_path Full path to the binary that will be loaded
---@private
-- luacov: enable
function M.get_active_binary_path()
  local rock_path = M.get_rock_binary_path()
  if rock_path then
    return rock_path
  end
  return M.get_binary_path()
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
---Get the installed binary version
---Returns the version from the version file if binary exists
---Returns nil if binary doesn't exist or version file can't be read
---@return string|nil version The installed version or nil
---@private
-- luacov: enable
function M.get_installed_version()
	local bin_path = M.get_active_binary_path()
	local ver_file = M.get_version_file()

	if vim.fn.filereadable(bin_path) == 1 and vim.fn.filereadable(ver_file) == 1 then
		-- Safely read the version file
		local ok, result = pcall(vim.fn.readfile, ver_file)
		if ok and result and result[1] then
			return result[1]
		end
	end
	return nil
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

	-- Verify hash
	local verify_ok, verify_err = M._verify_binary_hash(dest_path, ver)
	if not verify_ok then
		return false, {
			message = verify_err,
			url = nil,
			http_code = nil,
			tool = nil,
			exit_code = nil,
			stderr = nil,
		}
	end

	return true
end

-- luacov: disable
---Build from source
-- luacov: disable
---Get the source directory of the Hermes plugin
---Uses debug.getinfo to determine the path of the current Lua file
---@return string source_dir The detected source directory path
---@private
function M._get_source_dir()
	-- Auto-detect source directory from current Lua file location
	-- debug.getinfo(1).source returns "@/path/to/lua/hermes/binary.lua"
	local current_file = debug.getinfo(1).source:sub(2) -- Remove leading "@"
	-- Go up 3 levels: binary.lua → hermes/ → lua/ → project_root/
	return vim.fn.fnamemodify(current_file, ":h:h:h")
end
-- luacov: enable

---Builds from the local source directory where the plugin is installed
---@param dest_dir string Destination directory
---@return boolean success Whether build succeeded
---@private
-- luacov: enable
function M.build_from_source(dest_dir)
	local notification_options = { title = "Hermes - build" }
	local logging = require("hermes.logging")

	-- Ensure destination directory exists
	vim.fn.mkdir(dest_dir, "p")

	-- Check for required tools (cargo only, no git needed)
	if vim.fn.executable("cargo") ~= 1 then
		logging.notify("Rust/Cargo is required to build from source", vim.log.levels.ERROR, notification_options)
		return false
	end

	local source_dir = M._get_source_dir()

	-- Verify this looks like a Hermes source directory
	local cargo_toml = source_dir .. "/Cargo.toml"
	if vim.fn.filereadable(cargo_toml) ~= 1 then
		logging.notify(
			"Could not find Hermes source code at: "
				.. source_dir
				.. "\n"
				.. "Expected to find Cargo.toml in that directory",
			vim.log.levels.ERROR
		)
		return false
	end

	logging.notify("Building from source (this may take a few minutes)...", vim.log.levels.INFO, notification_options)

	-- Build with cargo from the detected source directory
 	local build_cmd = "cd " .. vim.fn.shellescape(source_dir) .. " && cargo build --release"
	local output = vim.fn.system(build_cmd)

	if vim.v.shell_error ~= 0 then
		logging.notify("Cargo build failed:\n" .. output, vim.log.levels.ERROR, notification_options)
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

	logging.notify("Build successful! Hermes has been built from source.", vim.log.levels.DEBUG, notification_options)
	return true
end

-- luacov: disable
---Build from source asynchronously
---Builds from the local source directory without blocking Neovim
---@param dest_dir string Destination directory
---@param features string[] Extra Cargo features to enable (e.g. {"with-icons"})
---@param on_complete function Callback function(success: boolean, err: string|nil)
---@return boolean started Whether build was started (false if already in progress)
---@private
-- luacov: enable
function M.build_from_source_async(dest_dir, features, on_complete)
	-- Support legacy callers that omit the features arg
	if type(features) == "function" then
		on_complete = features
		features = {}
	end
	local notification_options = { title = "Hermes - build" }
	local logging = require("hermes.logging")
	-- Check if build already in progress
	if _build_in_progress then
		-- Use logging.notify directly to ensure this always shows regardless of log level
		logging.notify(
			"Build already in progress. Use :Hermes cancel to stop.",
			vim.log.levels.WARN,
			notification_options
		)
		return false
	end

	-- Mark build as in progress immediately so subsequent calls are blocked
	_build_in_progress = true

	-- Show notification immediately - use logging.notify directly to ensure it always shows
	logging.notify(
		"Building Hermes from source... (this may take a few minutes)",
		vim.log.levels.INFO,
		notification_options
	)

	-- Use vim.schedule to make the actual work async and non-blocking
	vim.schedule(function()
		-- Ensure destination directory exists
		vim.fn.mkdir(dest_dir, "p")

		-- Check for required tools
		if vim.fn.executable("cargo") ~= 1 then
			_build_in_progress = false
			logging.notify("Rust/Cargo is required to build from source", vim.log.levels.ERROR, notification_options)
			on_complete(false, "cargo not available")
			return
		end

		-- Auto-detect source directory from current Lua file location
		local current_file = debug.getinfo(1).source:sub(2)
		local source_dir = vim.fn.fnamemodify(current_file, ":h:h:h")

		-- Verify this looks like a Hermes source directory
		local cargo_toml = source_dir .. "/Cargo.toml"
		if vim.fn.filereadable(cargo_toml) ~= 1 then
			_build_in_progress = false
			logging.notify(
				"Could not find Hermes source code at: "
					.. source_dir
					.. "\n"
					.. "Expected to find Cargo.toml in that directory",
				vim.log.levels.ERROR,
				{ title = "Hermes" }
			)
			on_complete(false, "Cargo.toml not found")
			return
		end

		-- Start async cargo build using jobstart
		local uv = vim.uv or vim.loop
		local start_time = uv.now()
		local progress_interval = 60000 -- Show progress every 60 seconds

		local cargo_args = { "cargo", "build", "--release" }
		if features and #features > 0 then
			table.insert(cargo_args, "--features")
			table.insert(cargo_args, table.concat(features, ","))
		end

		local job_id = vim.fn.jobstart(cargo_args, {
			cwd = source_dir,
			on_stdout = function(_, _) end,
			on_stderr = function(_, _) end,
			on_exit = vim.schedule_wrap(function(_, exit_code, _)
				_build_in_progress = false
				_build_job = nil

				if exit_code ~= 0 then
					logging.notify(
						"Build failed with exit code: " .. exit_code,
						vim.log.levels.ERROR,
						notification_options
					)
					on_complete(false, "Build failed")
					return
				end

				-- Build succeeded - copy binary and write version
				local platform = require("hermes.platform")
				local ext = platform.get_ext()
				local built_lib = source_dir .. "/target/release/libhermes." .. ext
				local dest_lib = dest_dir .. "/" .. M.get_binary_name()

				-- Check if built binary exists
				if vim.fn.filereadable(built_lib) ~= 1 then
					local err_msg = "Built binary not found at: " .. built_lib
					logging.notify(err_msg, vim.log.levels.ERROR, notification_options)
					on_complete(false, err_msg)
					return
				end

				local copy_ok = uv.fs_copyfile(built_lib, dest_lib)

				if not copy_ok then
					local err_msg = "Failed to copy built library from " .. built_lib .. " to " .. dest_lib
					logging.notify(err_msg, vim.log.levels.ERROR, notification_options)
					on_complete(false, err_msg)
					return
				end

				-- Write version file
				local ver_file = M.get_version_file()
				vim.fn.writefile({ "source" }, ver_file)

				local elapsed = math.floor((uv.now() - start_time) / 1000)
				logging.notify(
					"Hermes was been built from source in " .. elapsed .. " seconds.",
					vim.log.levels.INFO,
					{ title = "Hermes" }
				)
				on_complete(true, nil)
			end),
		})

		if job_id <= 0 then
			_build_in_progress = false
			logging.notify("Failed to start cargo build", vim.log.levels.ERROR, notification_options)
			on_complete(false, "Failed to start build")
			return
		end

		_build_job = {
			kill = function()
				vim.fn.jobstop(job_id)
			end,
		}

		-- Set up progress timer
		local progress_timer = uv.new_timer()
		progress_timer:start(progress_interval, progress_interval, function()
			if not _build_in_progress then
				progress_timer:stop()
				progress_timer:close()
				return
			end
			local elapsed = math.floor((uv.now() - start_time) / 1000 / 60)
			vim.schedule(function()
				logging.notify(
					"Still building... (" .. elapsed .. " minutes elapsed)",
					vim.log.levels.INFO,
					notification_options
				)
			end)
		end)
	end)

	return true
end

-- luacov: disable
---Cancel an in-progress build
---@return boolean cancelled Whether a build was cancelled
---@private
-- luacov: enable
function M.cancel_build()
	local notification_options = { title = "Hermes - build" }
	local logging = require("hermes.logging")

	if _build_job ~= nil then
		-- Kill the build job using jobstop
		_build_job.kill()
		_build_in_progress = false
		_build_job = nil
		logging.notify("Build cancelled", vim.log.levels.INFO, notification_options)
		return true
	else
		if not _build_in_progress then
			logging.notify("No build in progress to cancel", vim.log.levels.WARN, notification_options)
			return false
		end

		_build_in_progress = false
		_build_job = nil

		logging.notify("Build cancelled", vim.log.levels.INFO, notification_options)
		return true
	end
end

-- luacov: disable
---Cancel an in-progress download
---@return boolean cancelled Whether a download was cancelled
---@private
-- luacov: enable
function M.cancel_download()
	local notification_options = { title = "Hermes - download" }
	local logging = require("hermes.logging")

	if _download_job ~= nil then
		vim.fn.jobstop(_download_job)
		_download_in_progress = false
		_download_job = nil
		logging.notify("Download cancelled", vim.log.levels.INFO, notification_options)
		return true
	else
		if not _download_in_progress then
			logging.notify("No download in progress to cancel", vim.log.levels.WARN, notification_options)
			return false
		end

		-- luacov: disable
		_download_in_progress = false
		_download_job = nil

		logging.notify("Download cancelled", vim.log.levels.INFO, notification_options)
		return true
		-- luacov: enable
	end
end

-- luacov: disable
---Check if a build is currently in progress
---@return boolean in_progress Whether a build is in progress
---@private
-- luacov: enable
function M.is_build_in_progress()
	return _build_in_progress
end

-- luacov: disable
---Ensure binary is available (synchronous)
---Downloads binary only if it doesn't exist or version differs from config
---@return string path Path to binary
---@private
-- luacov: enable
function M.ensure_binary()
	local rock_bin_path = M.get_rock_binary_path()
	if rock_bin_path then
		local ver = M._get_rock_version()
		if ver then
			vim.fn.mkdir(M.get_data_dir(), "p")
			vim.fn.writefile({ ver }, M.get_version_file())
		end
		return rock_bin_path
	end

	local bin_path = M.get_binary_path()
	local ver_file = M.get_version_file()
	local version = require("hermes.version")
	local wanted_ver = version.get_wanted()
	local auto_download = require("hermes.config").get_auto_download()

	-- Check if binary already exists
	if vim.fn.filereadable(bin_path) == 1 then
		-- Binary exists - check if version matches config
		if vim.fn.filereadable(ver_file) == 1 then
			local current_ver = vim.fn.readfile(ver_file)[1]
			local use_source = current_ver == "source" and not auto_download
			-- If versions match, or it's a source build, use existing binary
			if current_ver == wanted_ver or use_source then
				return bin_path
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
	local bin_path = M.get_active_binary_path()

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

	local rock_bin_path = M.get_rock_binary_path()
	if rock_bin_path then
		local ver = M._get_rock_version()
		if ver then
			vim.fn.mkdir(M.get_data_dir(), "p")
			vim.fn.writefile({ ver }, M.get_version_file())
		end
		on_complete(true, rock_bin_path)
		return
	end

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

	-- Ensure data directory exists
	vim.fn.mkdir(M.get_data_dir(), "p")

	-- If version is "latest", fetch the actual latest version asynchronously
	if wanted_ver == "latest" then
		local version_mod = require("hermes.version")
		version_mod.fetch_latest_async(function(tag, _err)
			if not tag then
				on_complete(false, "Failed to fetch latest version")
				return
			end
			M._download_binary_async(tag, bin_path, ver_file, on_complete)
		end)
	else
		M._download_binary_async(wanted_ver, bin_path, ver_file, on_complete)
	end
end

-- luacov: disable
---Download a specific binary version asynchronously
---Performs platform/tool checks and downloads the specified version
---@param wanted_ver string Version to download (e.g., "v0.1.0")
---@param on_complete function Callback function(success: boolean, result: string)
-- luacov: enable
function M.download_async(wanted_ver, on_complete)
	local platform = require("hermes.platform")

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

	local download_mod = get_download()
	local download_tool = download_mod.get_available_tool()
	if not download_tool then
		on_complete(false, "No download tool available. Please install curl, wget, or PowerShell.")
		return
	end

	local bin_path = M.get_binary_path()
	local ver_file = M.get_version_file()

	vim.fn.mkdir(M.get_data_dir(), "p")
	M._download_binary_async(wanted_ver, bin_path, ver_file, on_complete)
end

-- luacov: disable
---Download binary asynchronously (internal helper)
---@param wanted_ver string Version to download
---@param bin_path string Binary path
---@param ver_file string Version file path
---@param on_complete function Callback function(success: boolean, result: string)
-- luacov: enable
function M._download_binary_async(wanted_ver, bin_path, ver_file, on_complete)
	local download_mod = get_download()

	-- Construct download URL
	local url =
		string.format("https://github.com/Ruddickmg/hermes.nvim/releases/download/%s/%s", wanted_ver, M.get_binary_name())

	-- Binary doesn't exist or version differs, need to download
	_download_in_progress = true
	local progress_id = "hermes-binary-download"

	local job_id = download_mod.download_async(url, bin_path, progress_id, function(download_ok, download_err)
		_download_in_progress = false
		_download_job = nil

		if download_ok then
			-- Make executable (Unix-like systems)
			if vim.fn.has("win32") ~= 1 then
				vim.fn.system({ "chmod", "+x", bin_path })
			end
			-- Verify hash
			local verify_ok, verify_err = M._verify_binary_hash(bin_path, wanted_ver)
			if not verify_ok then
				on_complete(false, verify_err or "Hash verification failed")
				return
			end
			-- Save version for reference
			vim.fn.writefile({ wanted_ver }, ver_file)
			on_complete(true, bin_path)
		else
			on_complete(false, download_err or "Download failed")
		end
	end)

	if job_id and job_id > 0 then
		_download_job = job_id
	else
		_download_in_progress = false
	end
end

return M
