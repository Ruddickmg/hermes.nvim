local M = {}

local uv = vim.loop

local BASE_URL = "https://github.com/Ruddickmg/hermes.nvim/releases/download"
local VERSION = "v0.1.0"

-- where binaries are stored locally
local function data_dir()
	return vim.fn.stdpath("data") .. "/hermes"
end

local function ensure_dir(path)
	if not uv.fs_stat(path) then
		uv.fs_mkdir(path, 493) -- 0755
	end
end

local function get_platform()
	local uname = uv.os_uname()

	if uname.sysname == "Linux" then
		return "linux", "so"
	elseif uname.sysname == "Darwin" then
		return "macos", "dylib"
	elseif uname.sysname:match("Windows") then
		return "windows", "dll"
	else
		error("Unsupported OS: " .. uname.sysname)
	end
end

local function get_arch()
	local arch = uv.os_uname().machine

	if arch == "x86_64" or arch == "amd64" then
		return "x86_64"
	elseif arch == "aarch64" or arch == "arm64" then
		return "aarch64"
	else
		error("Unsupported architecture: " .. arch)
	end
end

local function binary_name()
	local platform, ext = get_platform()
	local arch = get_arch()
	return string.format("hermes-%s-%s.%s", platform, arch, ext)
end

local function local_path()
	return data_dir() .. "/" .. binary_name()
end

local function download_url()
	return string.format("%s/%s/%s", BASE_URL, VERSION, binary_name())
end

local function download_binary(path)
	ensure_dir(data_dir())

	local url = download_url()

	vim.notify("Downloading my_plugin binary...", vim.log.levels.INFO)

	local result = vim.fn.system({
		"curl",
		"-L",
		"-o",
		path,
		url,
	})

	if vim.v.shell_error ~= 0 then
		error("Failed to download binary:\n" .. result)
	end

	-- make executable (unix only)
	if not vim.loop.os_uname().sysname:match("Windows") then
		vim.fn.system({ "chmod", "+x", path })
	end
end

local function ensure_binary()
	local path = local_path()

	if not uv.fs_stat(path) then
		download_binary(path)
	end

	return path
end

local function load_native()
	local path = ensure_binary()

	local ok, lib = pcall(package.loadlib, path, "luaopen_my_plugin")

	if not ok or not lib then
		error("Failed to load native module: " .. tostring(lib))
	end

	return lib()
end

-- public API
function M.setup(opts)
	opts = opts or {}

	local native = load_native()

	if native.setup then
		native.setup(opts)
	end

	return native
end

return M
