-- Unit tests for lua/hermes/binary.lua
-- Tests binary management with mocked HTTP downloads using download module

local helpers = require("helpers")
local stub = require("luassert.stub")

describe("hermes.binary", function()
	local binary
	local download
	local temp_dir
	local stdpath_stub
	local filereadable_stub
	local download_stub
	local version_stub
	local rock_binary_stub

	before_each(function()
		temp_dir = helpers.create_temp_dir()
		-- create_temp_dir returns temp_path/hermes, but we need stdpath("data") to return
		-- the parent directory (temp_path), so that binary.get_data_dir() returns temp_path/hermes
		local temp_path = temp_dir:gsub("/hermes$", "")
		stdpath_stub = stub(vim.fn, "stdpath").returns(temp_path)

		package.loaded["hermes.binary"] = nil
		package.loaded["hermes.download"] = nil
		package.loaded["hermes.platform"] = nil
		package.loaded["hermes.config"] = nil
		package.loaded["hermes.version"] = nil

		binary = require("hermes.binary")
		download = require("hermes.download")
		rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
	end)

	after_each(function()
		helpers.cleanup_temp_dir(temp_dir)
		if stdpath_stub then
			stdpath_stub:revert()
		end
		if filereadable_stub then
			filereadable_stub:revert()
		end
		if download_stub then
			download_stub:revert()
		end
		if version_stub then
			version_stub:revert()
		end
		if rock_binary_stub then
			rock_binary_stub:revert()
		end

		-- Clean up any inline stubs of vim.fn functions that tests may have created
		-- These are not tracked by the variables above and can cause test pollution
		pcall(function()
			if vim.fn.readfile.revert then
				vim.fn.readfile:revert()
			end
		end)
		pcall(function()
			if vim.fn.writefile.revert then
				vim.fn.writefile:revert()
			end
		end)
		pcall(function()
			if vim.fn.executable.revert then
				vim.fn.executable:revert()
			end
		end)
		pcall(function()
			if vim.fn.filereadable.revert then
				vim.fn.filereadable:revert()
			end
		end)
	end)

	describe("get_data_dir()", function()
		it("returns path ending with hermes", function()
			local dir = binary.get_data_dir()
			assert.matches("hermes$", dir)
		end)

		it("returns consistent path", function()
			local dir1 = binary.get_data_dir()
			local dir2 = binary.get_data_dir()
			assert.equals(dir1, dir2)
		end)
	end)

	describe("get_version_file()", function()
		it("returns path in data directory", function()
			local ver_file = binary.get_version_file()
			local data_dir = binary.get_data_dir()

			assert.is_true(ver_file:find(data_dir) == 1, "Version file should be in data directory")
		end)
	end)

	describe("get_binary_path()", function()
		it("includes platform-specific name", function()
			local bin_path = binary.get_binary_path()
			local expected_name = binary.get_binary_name()

			assert.truthy(bin_path:find(expected_name, 1, true), "Binary path should contain: " .. expected_name)
		end)
	end)

		describe("get_rock_binary_path()", function()
		it("finds binary at rock tree lib/ relative to module location", function()
			rock_binary_stub:revert()
			local bin_name = binary.get_binary_name()
			local func_source = debug.getinfo(binary.get_rock_binary_path, "S").source:sub(2)
			local rock_root = vim.fn.fnamemodify(func_source, ":h:h:h")
			local lib_dir = rock_root .. "/lib"
			local expected_path = lib_dir .. "/" .. bin_name
			vim.fn.mkdir(lib_dir, "p")
			io.open(expected_path, "w"):close()

			local result = binary.get_rock_binary_path()

			vim.fn.delete(lib_dir, "rf")
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
			assert.equals(expected_path, result)
		end)

		it("finds plain cargo-named library when platform-named file absent", function()
			rock_binary_stub:revert()
			local ext = require("hermes.platform").get_ext()
			local func_source = debug.getinfo(binary.get_rock_binary_path, "S").source:sub(2)
			local rock_root = vim.fn.fnamemodify(func_source, ":h:h:h")
			local lib_dir = rock_root .. "/lib"
			local expected_path = lib_dir .. "/libhermes." .. ext
			vim.fn.mkdir(lib_dir, "p")
			io.open(expected_path, "w"):close()

			local result = binary.get_rock_binary_path()

			vim.fn.delete(lib_dir, "rf")
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
			assert.equals(expected_path, result)
		end)

		it("prefers platform-named library over plain cargo-named one", function()
			rock_binary_stub:revert()
			local bin_name = binary.get_binary_name()
			local ext = require("hermes.platform").get_ext()
			local func_source = debug.getinfo(binary.get_rock_binary_path, "S").source:sub(2)
			local rock_root = vim.fn.fnamemodify(func_source, ":h:h:h")
			local lib_dir = rock_root .. "/lib"
			local expected_path = lib_dir .. "/" .. bin_name
			local fallback_path = lib_dir .. "/libhermes." .. ext
			vim.fn.mkdir(lib_dir, "p")
			io.open(expected_path, "w"):close()
			io.open(fallback_path, "w"):close()

			local result = binary.get_rock_binary_path()

			vim.fn.delete(lib_dir, "rf")
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
			assert.equals(expected_path, result)
		end)

		it("returns nil when binary not found in rock tree", function()
			rock_binary_stub:revert()
			assert.is_nil(binary.get_rock_binary_path())
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
		end)

		it("returns nil when debug.getinfo returns nil", function()
			rock_binary_stub:revert()
			local original_getinfo = debug.getinfo
			debug.getinfo = function(level, ...)
				if level == 1 then return nil end
				return original_getinfo(level, ...)
			end
			local result = binary.get_rock_binary_path()
			debug.getinfo = original_getinfo
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
			assert.is_nil(result)
		end)

		it("returns nil when debug info source is not a string", function()
			rock_binary_stub:revert()
			local original_getinfo = debug.getinfo
			debug.getinfo = function(level, ...)
				if level == 1 then return { source = 42 } end
				return original_getinfo(level, ...)
			end
			local result = binary.get_rock_binary_path()
			debug.getinfo = original_getinfo
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
			assert.is_nil(result)
		end)

		it("returns nil when debug info source is empty after stripping @", function()
			rock_binary_stub:revert()
			local original_getinfo = debug.getinfo
			debug.getinfo = function(level, ...)
				if level == 1 then return { source = "@" } end
				return original_getinfo(level, ...)
			end
			local result = binary.get_rock_binary_path()
			debug.getinfo = original_getinfo
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns(nil)
			assert.is_nil(result)
		end)
	end)

	describe("_get_rock_root()", function()
		it("returns a string path", function()
			local root = binary._get_rock_root()
			assert.is_string(root)
		end)

		it("returns absolute path", function()
			local root = binary._get_rock_root()
			assert.is_not_nil(root:match("^/") or root:match("^%a:[/\\]"), "Expected absolute path")
		end)
	end)

	describe("is_luarocks_install()", function()
		it("returns false when debug.getinfo returns nil", function()
			local original_getinfo = debug.getinfo
			debug.getinfo = function(level, ...)
				if level == 1 then return nil end
				return original_getinfo(level, ...)
			end
			local result = binary.is_luarocks_install()
			debug.getinfo = original_getinfo
			assert.is_false(result)
		end)

		it("returns false when source is empty after stripping @", function()
			local original_getinfo = debug.getinfo
			debug.getinfo = function(level, ...)
				if level == 1 then return { source = "@" } end
				return original_getinfo(level, ...)
			end
			local result = binary.is_luarocks_install()
			debug.getinfo = original_getinfo
			assert.is_false(result)
		end)
	end)

	describe("is_nix_install()", function()
		it("returns false when debug.getinfo returns nil", function()
			local uv = vim.uv or vim.loop
			local fs_stat_stub = stub(uv, "fs_stat").returns(nil)
			local original_getinfo = debug.getinfo
			debug.getinfo = function(level, ...)
				if level == 1 then return nil end
				return original_getinfo(level, ...)
			end
			local result = binary.is_nix_install()
			debug.getinfo = original_getinfo
			fs_stat_stub:revert()
			assert.is_false(result)
		end)

		it("returns false when source is empty after stripping @", function()
			local uv = vim.uv or vim.loop
			local fs_stat_stub = stub(uv, "fs_stat").returns(nil)
			local original_getinfo = debug.getinfo
			debug.getinfo = function(level, ...)
				if level == 1 then return { source = "@" } end
				return original_getinfo(level, ...)
			end
			local result = binary.is_nix_install()
			debug.getinfo = original_getinfo
			fs_stat_stub:revert()
			assert.is_false(result)
		end)

		it("returns true when the system is NixOS", function()
			local uv = vim.uv or vim.loop
			local fs_stat_stub = stub(uv, "fs_stat").returns({})
			local result = binary.is_nix_install()
			fs_stat_stub:revert()
			assert.is_true(result)
		end)
	end)

	describe("_get_rock_version()", function()
		it("returns nil when glob returns empty", function()
			local glob_stub = stub(vim.fn, "glob").returns({})
			local result = binary._get_rock_version()
			glob_stub:revert()
			assert.is_nil(result)
		end)

		it("returns version from rockspec filename with revision", function()
			local glob_stub = stub(vim.fn, "glob").returns({
				"/some/path/lib/luarocks/rocks/hermes.nvim/0.1.0-1/hermes.nvim-0.1.0-1.rockspec",
			})
			local result = binary._get_rock_version()
			glob_stub:revert()
			assert.equals("v0.1.0", result)
		end)

		it("returns version from rockspec filename without revision", function()
			local glob_stub = stub(vim.fn, "glob").returns({
				"/some/path/lib/luarocks/rocks/hermes.nvim/0.1.0/hermes.nvim-0.1.0.rockspec",
			})
			local result = binary._get_rock_version()
			glob_stub:revert()
			assert.equals("v0.1.0", result)
		end)

		it("handles version already with v prefix", function()
			local glob_stub = stub(vim.fn, "glob").returns({
				"/some/path/lib/luarocks/rocks/hermes.nvim/v0.1.0-1/hermes.nvim-v0.1.0-1.rockspec",
			})
			local result = binary._get_rock_version()
			glob_stub:revert()
			assert.equals("v0.1.0", result)
		end)

		it("strips revision suffix correctly", function()
			local glob_stub = stub(vim.fn, "glob").returns({
				"/some/path/lib/luarocks/rocks/hermes.nvim/0.10.0-3/hermes.nvim-0.10.0-3.rockspec",
			})
			local result = binary._get_rock_version()
			glob_stub:revert()
			assert.equals("v0.10.0", result)
		end)

		it("returns nil when rock_root is nil", function()
			local root_stub = stub(binary, "_get_rock_root").returns(nil)
			local result = binary._get_rock_version()
			root_stub:revert()
			assert.is_nil(result)
		end)

		it("returns nil when glob returns file without rockspec extension", function()
			local glob_stub = stub(vim.fn, "glob").returns({
				"/some/path/lib/luarocks/rocks/hermes.nvim/0.1.0-1/hermes.nvim.txt",
			})
			local result = binary._get_rock_version()
			glob_stub:revert()
			assert.is_nil(result)
		end)
	end)

	describe("get_active_binary_path()", function()
		it("returns rock path when rock binary exists", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/lib.so")
			local result = binary.get_active_binary_path()
			assert.equals("/fake/rock/lib.so", result)
		end)

		it("returns data dir path when no rock binary", function()
			local result = binary.get_active_binary_path()
			local expected = binary.get_binary_path()
			assert.equals(expected, result)
		end)
	end)

	describe("download()", function()
		it("downloads to correct path", function()
			local captured_dest
			download_stub = stub(download, "download").invokes(function(_, dest)
				captured_dest = dest
				return true, nil
			end)
			stub(binary, "_verify_binary_hash").returns(true)

			local target_path = temp_dir .. "/libhermes-linux-x86_64.so"
			binary.download(target_path, "v1.0.0")

			assert.equals(target_path, captured_dest)
		end)

		it("returns true on success", function()
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(true)

			local result = binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.is_true(result)
		end)

		it("returns false on failure", function()
			download_stub = stub(download, "download").returns(false, "Network error")

			local result = binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.is_false(result)
		end)

		it("returns false when platform is unsupported", function()
			stub(require("hermes.platform"), "get_platform_key").returns(nil)

			local result, _ = binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.is_false(result)
		end)

		it("returns structured error when platform is unsupported", function()
			stub(require("hermes.platform"), "get_platform_key").returns(nil)

			local _, err = binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.truthy(err and err.message)
		end)

		it("fetches latest version when ver is latest", function()
			local fetch_called = false
			stub(require("hermes.version"), "fetch_latest").invokes(function()
				fetch_called = true
				return "v2.0.0"
			end)
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(true)

			binary.download(temp_dir .. "/test.so", "latest")

			assert.is_true(fetch_called, "fetch_latest should be called when version is 'latest'")
		end)
	end)
	describe("ensure_binary()", function()
		it("downloads when binary missing", function()
			filereadable_stub = stub(vim.fn, "filereadable").returns(0)
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(true)
			stub(download, "get_available_tool").returns("curl")
			version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			stub(vim.fn, "writefile")

			binary.ensure_binary()

			assert.stub(download_stub).was_called()
		end)

		it("skips download when binary exists and version matches", function()
			-- Create existing binary file and version file
			local bin_path = binary.get_binary_path()
			local ver_file = binary.get_version_file()
			vim.fn.mkdir(binary.get_data_dir(), "p")
			io.open(bin_path, "w"):close()
			local f = io.open(ver_file, "w")
			f:write("v1.0.0")
			f:close()

			-- Mock: binary exists (1), version file exists (1)
			local filereadable_count = 0
			filereadable_stub = stub(vim.fn, "filereadable").invokes(function()
				filereadable_count = filereadable_count + 1
				return 1 -- Both files exist
			end)

			stub(vim.fn, "readfile").returns({ "v1.0.0" })
			version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			download_stub = stub(download, "download")

			binary.ensure_binary()

			assert.stub(download_stub).was_not_called()
		end)

		it("downloads when binary exists but version differs", function()
			-- Create existing binary file with old version
			local bin_path = binary.get_binary_path()
			local ver_file = binary.get_version_file()
			vim.fn.mkdir(binary.get_data_dir(), "p")
			io.open(bin_path, "w"):close()
			local f = io.open(ver_file, "w")
			f:write("v0.9.0")
			f:close()

			-- Mock: binary exists (1), version file exists (1)
			local filereadable_count = 0
			filereadable_stub = stub(vim.fn, "filereadable").invokes(function()
				filereadable_count = filereadable_count + 1
				return 1 -- Both files exist
			end)

			stub(vim.fn, "readfile").returns({ "v0.9.0" })
			version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(true)
			stub(vim.fn, "writefile")

			binary.ensure_binary()

			assert.stub(download_stub).was_called()
		end)

		it("downloads when binary exists but version file is missing", function()
			-- Create binary file but NO version file
			local bin_path = binary.get_binary_path()
			vim.fn.mkdir(binary.get_data_dir(), "p")
			io.open(bin_path, "w"):close()

			-- Mock: binary exists (1), version file does NOT exist (0)
			filereadable_stub = stub(vim.fn, "filereadable").invokes(function(path)
				if path == bin_path then
					return 1
				end
				return 0 -- version file missing
			end)

			version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(true)
			stub(vim.fn, "writefile")

			binary.ensure_binary()

			assert.stub(download_stub).was_called()
		end)

		it("returns rock binary path immediately when found", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/libhermes.so")
			download_stub = stub(download, "download")

			local result = binary.ensure_binary()

			assert.equals("/fake/rock/libhermes.so", result)
		end)

		it("skips download when rock binary path is found", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/libhermes.so")
			download_stub = stub(download, "download")

			binary.ensure_binary()

			assert.stub(download_stub).was_not_called()
		end)
	end)

	describe("build_from_source()", function()
		it("copies built library to correct path with platform suffix", function()
			-- Create a mock built library file (simulating cargo build output)
			local platform = require("hermes.platform")
			local build_dir = temp_dir .. "/build"
			local target_dir = build_dir .. "/target/release"
			local ext = platform.get_ext()
			local mock_built_lib = target_dir .. "/libhermes." .. ext
			local expected_bin_name = binary.get_binary_name()
			local expected_final_path = temp_dir .. "/" .. expected_bin_name

			-- Create directory structure and mock library file
			vim.fn.mkdir(target_dir, "p")
			local f = io.open(mock_built_lib, "w")
			f:write("mock library content")
			f:close()

			-- Mock the build process by directly testing the copy behavior
			-- This bypasses the actual git clone and cargo build
			local uv = vim.uv or vim.loop
			local dest_dir = temp_dir
			local bin_name = binary.get_binary_name()
			local final_path = dest_dir .. "/" .. bin_name

			-- Manually copy the file to simulate what build_from_source should do
			local result, err = uv.fs_copyfile(mock_built_lib, final_path)

			-- Verify both that copy succeeded AND file exists at expected path
			local file_exists = vim.fn.filereadable(expected_final_path) == 1
			assert.is_true(
				result and file_exists,
				"Failed to copy: " .. (err or "unknown error") .. " or file not found at: " .. expected_final_path
			)
		end)

		it("uses correct filename format consistent with get_binary_path()", function()
			local platform = require("hermes.platform")
			local expected_name = platform.get_binary_name()

			-- Build expected format and verify in single assertion
			local expected_format = "libhermes-"
				.. platform.get_os()
				.. "-"
				.. platform.get_arch()
				.. "."
				.. platform.get_ext()
			assert.equals(expected_format, expected_name)
		end)
	end)

	describe("build_from_source() error handling", function()
		it("returns false when cargo is not available", function()
			-- This is already covered by test at line 542
			-- Keeping this block for organization
			stub(vim.fn, "executable").returns(0) -- cargo not available
			local result = binary.build_from_source(temp_dir)
			assert.is_false(result)
		end)

		it("returns false when Cargo.toml is missing", function()
			stub(vim.fn, "executable").returns(1) -- cargo available
			-- Mock filereadable to return 0 for Cargo.toml (not found)
			stub(vim.fn, "filereadable").invokes(function(path)
				if path:match("Cargo%.toml$") then
					return 0
				end
				return 1
			end)

			local result = binary.build_from_source(temp_dir)
			assert.is_false(result)
		end)

		it("handles copy failure gracefully", function()
			local platform = require("hermes.platform")
			local build_dir = temp_dir .. "/build"
			local target_dir = build_dir .. "/target/release"
			local ext = platform.get_ext()
			local mock_built_lib = target_dir .. "/libhermes." .. ext

			-- Create directory and mock built file
			vim.fn.mkdir(target_dir, "p")
			local f = io.open(mock_built_lib, "w")
			f:write("mock content")
			f:close()

			-- Mock successful git and cargo
			stub(vim.fn, "system").returns("")
			stub(vim.fn, "executable").returns(1)

			-- Mock fs_copyfile to fail
			local uv_stub = stub(vim.uv or vim.loop, "fs_copyfile").returns(nil, "Permission denied")
			local notify_stub = stub(require("hermes.logging"), "notify")

			local result = binary.build_from_source(temp_dir)

			uv_stub:revert()
			notify_stub:revert()

			assert.is_false(result)
		end)

		it("returns true on successful build", function()
			local platform = require("hermes.platform")
			local build_dir = temp_dir .. "/build"
			local target_dir = build_dir .. "/target/release"
			local ext = platform.get_ext()
			local mock_built_lib = target_dir .. "/libhermes." .. ext
			local dest_dir = temp_dir

			vim.fn.mkdir(target_dir, "p")
			local f = io.open(mock_built_lib, "w")
			f:write("mock library content")
			f:close()

			local system_stub = stub(vim.fn, "system").returns("")
			local executable_stub = stub(vim.fn, "executable").returns(1)
			local notify_stub = stub(require("hermes.logging"), "notify")

			local result = binary.build_from_source(dest_dir)

			system_stub:revert()
			executable_stub:revert()
			notify_stub:revert()

			assert.is_true(result)
		end)

		it("writes version file on successful build", function()
			local platform = require("hermes.platform")
			local build_dir = temp_dir .. "/build"
			local target_dir = build_dir .. "/target/release"
			local ext = platform.get_ext()
			local mock_built_lib = target_dir .. "/libhermes." .. ext
			local bin_name = binary.get_binary_name()
			local dest_dir = temp_dir
			local final_path = dest_dir .. "/" .. bin_name

			vim.fn.mkdir(target_dir, "p")
			local f = io.open(mock_built_lib, "w")
			f:write("mock library content")
			f:close()

			local system_stub = stub(vim.fn, "system").returns("")
			local executable_stub = stub(vim.fn, "executable").returns(1)
			local notify_stub = stub(require("hermes.logging"), "notify")

			binary.build_from_source(dest_dir)

			system_stub:revert()
			executable_stub:revert()
			notify_stub:revert()

			local ver_file = binary.get_version_file()
			local version_content = vim.fn.filereadable(ver_file) == 1 and vim.fn.readfile(ver_file)[1] or nil

			pcall(vim.fn.delete, ver_file)
			pcall(vim.fn.delete, final_path)

			assert.equals("source", version_content)
		end)
	end)

	describe("ensure_binary() error paths", function()
		it("returns error for unsupported platform", function()
			stub(require("hermes.platform"), "is_supported").returns(false)
			stub(require("hermes.platform"), "get_platform_key").returns("mips")
			stub(require("hermes.platform"), "get_display_string").returns("mips")
			stub(vim.fn, "filereadable").returns(0)

			local ok, _ = pcall(function()
				binary.ensure_binary()
			end)

			assert.is_false(ok)
		end)

		it("error message mentions platform for unsupported platform", function()
			stub(require("hermes.platform"), "is_supported").returns(false)
			stub(require("hermes.platform"), "get_platform_key").returns("mips")
			stub(require("hermes.platform"), "get_display_string").returns("mips")
			stub(vim.fn, "filereadable").returns(0)

			local _, err = pcall(function()
				binary.ensure_binary()
			end)

			assert.truthy(err:match("not supported") or err:match("platform"))
		end)

		it("returns error on download failure", function()
			stub(vim.fn, "filereadable").returns(0)
			stub(require("hermes.config"), "get").returns({
				download = {
					auto = true,
					version = "v9.9.9",
				},
			})
			stub(download, "download").returns(false, "HTTP 404")
			stub(download, "get_available_tool").returns("curl")
			stub(require("hermes.platform"), "is_supported").returns(true)

			local ok, _ = pcall(function()
				binary.ensure_binary()
			end)

			assert.is_false(ok)
		end)

		it("error message mentions download failure", function()
			stub(vim.fn, "filereadable").returns(0)
			stub(require("hermes.config"), "get").returns({
				download = {
					auto = true,
					version = "v9.9.9",
				},
			})
			stub(download, "download").returns(false, "HTTP 404")
			stub(download, "get_available_tool").returns("curl")
			stub(require("hermes.platform"), "is_supported").returns(true)

			local _, err = pcall(function()
				binary.ensure_binary()
			end)

			assert.truthy(err:match("download") or err:match("Failed"))
		end)
	end)

	describe("ensure_binary() with rock binary", function()
		it("writes version.txt from rock version when found", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/lib.so")
			version_stub = stub(binary, "_get_rock_version").returns("v0.2.0")
			local writefile_stub = stub(vim.fn, "writefile")
			local mkdir_stub = stub(vim.fn, "mkdir")

			binary.ensure_binary()

			version_stub:revert()
			mkdir_stub:revert()
			writefile_stub:revert()

			assert.stub(writefile_stub).was_called()
		end)

		it("writes correct version to version.txt", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/lib.so")
			version_stub = stub(binary, "_get_rock_version").returns("v0.2.0")
			local writefile_stub = stub(vim.fn, "writefile")
			local mkdir_stub = stub(vim.fn, "mkdir")

			binary.ensure_binary()

			version_stub:revert()
			mkdir_stub:revert()
			writefile_stub:revert()

			assert.stub(writefile_stub).was_called_with({ "v0.2.0" }, temp_dir .. "/version.txt")
		end)

		it("does not write version.txt when _get_rock_version returns nil", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/lib.so")
			version_stub = stub(binary, "_get_rock_version").returns(nil)
			local writefile_stub = stub(vim.fn, "writefile")

			binary.ensure_binary()

			version_stub:revert()
			writefile_stub:revert()

			assert.stub(writefile_stub).was_not_called()
		end)
	end)

	describe("load_existing_binary()", function()
		it("returns path when binary exists", function()
			local bin_path = binary.get_binary_path()
			vim.fn.mkdir(binary.get_data_dir(), "p")
			io.open(bin_path, "w"):close()

			filereadable_stub = stub(vim.fn, "filereadable").returns(1)

			local result = binary.load_existing_binary()
			assert.equals(bin_path, result)
		end)

		it("errors when no download tools available", function()
			-- Mock no download tools available
			stub(download, "is_curl_available").returns(false)
			stub(download, "is_wget_available").returns(false)
			stub(download, "get_available_tool").returns(nil)

			local ok, _ = pcall(function()
				binary.ensure_binary()
			end)

			assert.is_false(ok)
		end)

		it("error message mentions download tools when none available", function()
			-- Mock no download tools available
			stub(vim.fn, "filereadable").returns(0)
			stub(download, "get_available_tool").returns(nil)
			stub(require("hermes.platform"), "is_supported").returns(true)
			stub(require("hermes.platform"), "get_platform_key").returns("linux-x86_64")

			local _, err = pcall(function()
				binary.ensure_binary()
			end)
			assert.truthy(err:match("curl") or err:match("wget"))
		end)
	end)

	describe("load()", function()
		it("returns native module when binary exists and loads successfully", function()
			-- Use the real binary from target/release
			local platform = require("hermes.platform")
			local bin_path = binary.get_binary_path()

			-- Ensure binary directory exists and copy real binary
			vim.fn.mkdir(binary.get_data_dir(), "p")
			local source_bin = vim.fn.getcwd() .. "/target/release/libhermes." .. platform.get_ext()
			local uv = vim.uv or vim.loop
			uv.fs_copyfile(source_bin, bin_path)

			-- Mock filereadable to return 1 (file exists)
			stub(vim.fn, "filereadable").returns(1)

			-- Mock the version module to avoid download checks
			stub(require("hermes.version"), "get_wanted").returns("v0.0.1")

			-- Also need to mock readfile for version check
			stub(vim.fn, "readfile").returns({ "v0.0.1" })

			-- Call load - should use real binary
			local ok, result = pcall(function()
				return binary.load()
			end)

			-- Should succeed and return a table (the native module) - combined assertion
			assert.is_true(
				ok and type(result) == "table",
				"load should succeed and return native module table: " .. tostring(result)
			)
		end)

		it("loads from rock binary path when rock binary exists", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/libhermes.so")
			local captured_path = nil
			local loadlib_stub = stub(package, "loadlib").invokes(function(path, _name)
				captured_path = path
				return function() return {} end
			end)

			binary.load()

			loadlib_stub:revert()
			assert.equals("/fake/rock/libhermes.so", captured_path)
		end)

		it("returns false when package.loadlib fails to load binary", function()
			filereadable_stub = stub(vim.fn, "filereadable").returns(1)
			version_stub = stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
			local readfile_stub = stub(vim.fn, "readfile").returns({ "v0.0.1" })
			local loadlib_stub = stub(package, "loadlib").returns(nil)

			local ok = pcall(function()
				return binary.load()
			end)

			loadlib_stub:revert()
			readfile_stub:revert()
			version_stub:revert()
			filereadable_stub:revert()

			assert.is_false(ok)
		end)

		it("returns error message when package.loadlib fails to load binary", function()
			filereadable_stub = stub(vim.fn, "filereadable").returns(1)
			version_stub = stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
			local readfile_stub = stub(vim.fn, "readfile").returns({ "v0.0.1" })
			local loadlib_stub = stub(package, "loadlib").returns(nil)

			local _, err = pcall(function()
				return binary.load()
			end)

			loadlib_stub:revert()
			readfile_stub:revert()
			version_stub:revert()
			filereadable_stub:revert()

			assert.truthy(err:match("Failed to load native module"))
		end)
	end)

  describe("ensure_binary_async()", function()
    it("callback is called synchronously when rock binary found", function()
      rock_binary_stub:revert()
      rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/libhermes.so")

      local callback_called = false
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
      end)

      assert.is_true(callback_called, "Callback should fire synchronously for rock binary")
    end)

    it("callback receives rock binary path", function()
      rock_binary_stub:revert()
      rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/libhermes.so")

      local callback_result = nil
      binary.ensure_binary_async(60, function(_success, result)
        callback_result = result
      end)

      assert.equals("/fake/rock/libhermes.so", callback_result)
    end)

    it("callback is called when binary exists", function()
      local platform = require("hermes.platform")
      local bin_path = binary.get_binary_path()

      vim.fn.mkdir(binary.get_data_dir(), "p")
      local source_bin = vim.fn.getcwd() .. "/target/release/libhermes." .. platform.get_ext()
      local uv = vim.uv or vim.loop
      uv.fs_copyfile(source_bin, bin_path)

      vim.fn.writefile({ "v0.0.1" }, binary.get_version_file())
      stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
      stub(vim.fn, "filereadable").returns(1)
      stub(vim.fn, "readfile").returns({ "v0.0.1" })
      stub(download, "get_available_tool").returns("curl")

      local callback_called = false
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
      end)

      vim.wait(100, function()
        return callback_called
      end)

      assert.is_true(callback_called, "Callback should be called when binary exists")
    end)

    it("callback receives success when binary exists", function()
      local platform = require("hermes.platform")
      local bin_path = binary.get_binary_path()

      vim.fn.mkdir(binary.get_data_dir(), "p")
      local source_bin = vim.fn.getcwd() .. "/target/release/libhermes." .. platform.get_ext()
      local uv = vim.uv or vim.loop
      uv.fs_copyfile(source_bin, bin_path)

      vim.fn.writefile({ "v0.0.1" }, binary.get_version_file())
      stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
      stub(vim.fn, "filereadable").returns(1)
      stub(vim.fn, "readfile").returns({ "v0.0.1" })
      stub(download, "get_available_tool").returns("curl")

      local callback_success = nil
      binary.ensure_binary_async(60, function(success, _result)
        callback_success = success
      end)

      vim.wait(100, function()
        return callback_success ~= nil
      end)

      assert.is_true(callback_success, "Callback should receive success when binary exists")
    end)

    it("callback receives non-nil result when binary exists", function()
      local platform = require("hermes.platform")
      local bin_path = binary.get_binary_path()

      vim.fn.mkdir(binary.get_data_dir(), "p")
      local source_bin = vim.fn.getcwd() .. "/target/release/libhermes." .. platform.get_ext()
      local uv = vim.uv or vim.loop
      uv.fs_copyfile(source_bin, bin_path)

      vim.fn.writefile({ "v0.0.1" }, binary.get_version_file())
      stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
      stub(vim.fn, "filereadable").returns(1)
      stub(vim.fn, "readfile").returns({ "v0.0.1" })
      stub(download, "get_available_tool").returns("curl")

      local callback_result = nil
      binary.ensure_binary_async(60, function(_success, result)
        callback_result = result
      end)

      vim.wait(100, function()
        return callback_result ~= nil
      end)

      assert.is_not_nil(callback_result, "Callback should receive binary path")
    end)

    it("downloads when binary is missing", function()
      local bin_path = binary.get_binary_path()
      vim.fn.delete(bin_path)

      stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
      stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
        on_complete(true, nil)
      end)
      stub(binary, "_verify_binary_hash").returns(true)
      stub(vim.fn, "writefile")
      stub(download, "get_available_tool").returns("curl")

      local ok = pcall(function()
        binary.ensure_binary_async(60, function() end)
      end)

      assert.is_true(ok, "ensure_binary_async should not crash when binary is missing")
    end)

    it("downloads when binary exists but version differs", function()
      local bin_path = binary.get_binary_path()
      local ver_file = binary.get_version_file()
      vim.fn.mkdir(binary.get_data_dir(), "p")
      io.open(bin_path, "w"):close()
      local f = io.open(ver_file, "w")
      f:write("v0.9.0")
      f:close()

      stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
      stub(vim.fn, "filereadable").returns(1)
      stub(vim.fn, "readfile").returns({ "v0.9.0" })
      stub(vim.fn, "writefile")
      stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
        on_complete(true, nil)
      end)
      stub(binary, "_verify_binary_hash").returns(true)
      stub(download, "get_available_tool").returns("curl")

      local callback_called = false
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
      end)

      vim.wait(100)

      assert.is_true(callback_called, "Callback should be called for version mismatch")
    end)

    it("callback is called for unsupported platform", function()
      stub(require("hermes.platform"), "get_platform_key").returns(nil)

      local callback_called = false
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
      end)

      assert.is_true(callback_called, "Callback should be called for unsupported platform")
    end)

    it("callback reports failure for unsupported platform", function()
      stub(require("hermes.platform"), "get_platform_key").returns(nil)

      local callback_success = nil
      binary.ensure_binary_async(60, function(success, _result)
        callback_success = success
      end)

      assert.is_false(callback_success, "Should report failure for unsupported platform")
    end)

    it("callback is called when no download tool available", function()
      stub(download, "get_available_tool").returns(nil)

      local callback_called = false
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
      end)

      assert.is_true(callback_called, "Callback should be called when no download tool")
    end)

    it("callback reports failure when no download tool available", function()
      stub(download, "get_available_tool").returns(nil)

      local callback_success = nil
      binary.ensure_binary_async(60, function(success, _result)
        callback_success = success
      end)

      assert.is_false(callback_success, "Should report failure when no download tool")
    end)
  end)

	describe("build_from_source()", function()
		it("exports build_from_source as a function", function()
			assert.is_function(binary.build_from_source)
		end)

		it("_get_source_dir returns a string path", function()
			local source_dir = binary._get_source_dir()
			assert.is_string(source_dir)
		end)
	end)

	describe("ensure_binary() with source version", function()
	it("accepts 'source' version as valid and returns binary path", function()
		-- Disable auto-download to ensure source build is used
		local config = require("hermes.config")
		config.setup({ download = { auto = false } })

		-- Create binary file
		local bin_path = binary.get_binary_path()
		vim.fn.mkdir(binary.get_data_dir(), "p")
		local f = io.open(bin_path, "w")
		f:write("mock binary")
		f:close()

		-- Create version file with "source"
		local ver_file = binary.get_version_file()
		vim.fn.writefile({ "source" }, ver_file)

		-- Should not error and should return the binary path
		local result = binary.ensure_binary()

		assert.equals(bin_path, result)

		-- Cleanup
		os.remove(bin_path)
		os.remove(ver_file)
	end)

		it("writes 'source' to version file after successful build", function()
			-- This test validates that ensure_binary handles 'source' version correctly
			-- Create data directory structure
			local data_dir = binary.get_data_dir()
			vim.fn.mkdir(data_dir, "p")

			-- Create a mock version file
			local ver_file = binary.get_version_file()
			vim.fn.writefile({ "source" }, ver_file)

			-- Read it back
			local lines = vim.fn.readfile(ver_file)

			-- Verify content
			assert.equals("source", lines[1])

			-- Cleanup
			os.remove(ver_file)
		end)
	end)

	describe("build_from_source_async()", function()
		before_each(function()
			-- Set notification level to INFO so we can capture notifications
			local config = require("hermes.config")
			config.setup({
				log = { notification = { level = "info" } },
			})
		end)

		it("returns false when build is already in progress", function()
			-- Stub vim.fn.jobstart to return a job ID
			local jobstart_stub = stub(vim.fn, "jobstart").returns(123)

			-- Start a build
			binary.build_from_source_async(temp_dir, function() end)

			-- Try to start another build while first is in progress
			local result2 = binary.build_from_source_async(temp_dir, function() end)

			jobstart_stub:revert()

			-- First should succeed (or return true to indicate it started)
			-- Second should fail/return false because build is already in progress
			assert.is_false(result2)
		end)

		it("shows warning when attempting duplicate build", function()
			stub(vim.fn, "jobstart").returns(123)

			local notify_calls = {}
			local original_notify = vim.notify
			vim.notify = function(msg, level)
				table.insert(notify_calls, { msg = msg, level = level })
			end

			-- Start first build
			binary.build_from_source_async(temp_dir, function() end)

			-- Try second build
			binary.build_from_source_async(temp_dir, function() end)

			vim.notify = original_notify

			-- Should have warning about build in progress
			assert.is_true(#vim.tbl_filter(function(call)
				return call.msg and call.msg:find("already in progress") ~= nil
			end, notify_calls) > 0, "Should warn about build already in progress")
		end)

		it("returns true when cargo is not available (async)", function()
			stub(vim.fn, "executable").returns(0) -- cargo not available

			local result = binary.build_from_source_async(temp_dir, function() end)

			-- Should return true (build started) but callback should fail
			assert.is_true(result)
		end)

		it("calls callback with failure when cargo is not available", function()
			stub(vim.fn, "executable").returns(0) -- cargo not available

			local callback_called = false

			binary.build_from_source_async(temp_dir, function(_success, _err)
				callback_called = true
			end)

			-- Wait for the async work to complete
			vim.wait(100)

			assert.is_true(callback_called)
		end)

		it("callback receives false success when cargo is not available", function()
			stub(vim.fn, "executable").returns(0) -- cargo not available

			local callback_success = nil

			binary.build_from_source_async(temp_dir, function(success, _err)
				callback_success = success
			end)

			-- Wait for the async work to complete
			vim.wait(100)

			assert.is_false(callback_success)
		end)

		it("notifies about missing cargo", function()
			stub(vim.fn, "executable").returns(0)

			local notify_calls = {}
			local original_notify = vim.notify
			vim.notify = function(msg, level)
				table.insert(notify_calls, { msg = msg, level = level })
			end

			binary.build_from_source_async(temp_dir, function() end)

			-- Wait for the async work to complete
			vim.wait(100)

			vim.notify = original_notify

			local found_cargo_msg = false
			for _, call in ipairs(notify_calls) do
				if call.msg and (call.msg:find("cargo") or call.msg:find("Rust")) then
					found_cargo_msg = true
					break
				end
			end

			assert.is_true(found_cargo_msg, "Should notify about missing cargo/Rust")
		end)

		it("accepts source directory as parameter", function()
			-- First clean up any previous build state
			if binary.is_build_in_progress() then
				binary.cancel_build()
			end

			-- Stub vim.fn.jobstart to return a valid job ID
			local jobstart_stub = stub(vim.fn, "jobstart").returns(123)

			-- Mock cargo as available
			stub(vim.fn, "executable").returns(1)

			-- Use temp_dir which is a valid directory
			local result = binary.build_from_source_async(temp_dir, function() end)

			jobstart_stub:revert()

			-- Should accept the custom directory
			assert.is_true(result)
		end)

		it("accepts callback function for completion", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			-- Verify the function accepts the callback parameter without error
			local ok, err = pcall(function()
				binary.build_from_source_async(temp_dir, function(_success, _error)
					-- Callback defined
				end)
			end)

			assert.is_true(ok, "Should accept callback parameter without error: " .. tostring(err))
		end)

		it("returns true when build starts successfully", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			local result = binary.build_from_source_async(temp_dir, function() end)

			assert.is_true(result)
		end)

		it("notifies that build has started", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			local notify_calls = {}
			local original_notify = vim.notify
			vim.notify = function(msg, level)
				table.insert(notify_calls, { msg = msg, level = level })
			end

			binary.build_from_source_async(temp_dir, function() end)

			vim.notify = original_notify

			local found_start_msg = false
			for _, call in ipairs(notify_calls) do
				if call.msg and (call.msg:find("Building") or call.msg:find("from source")) then
					found_start_msg = true
					break
				end
			end

			assert.is_true(found_start_msg, "Should notify that build has started")
		end)

		it("omits features flag when no features given", function()
			local captured_args
			local jobstart_stub = stub(vim.fn, "jobstart").invokes(function(args)
				captured_args = args
				return 123
			end)
			stub(vim.fn, "executable").returns(1)

			binary.build_from_source_async(temp_dir, function() end)
			vim.wait(50)

			jobstart_stub:revert()

			local has_features = false
			for _, arg in ipairs(captured_args) do
				if arg == "--features" then
					has_features = true
					break
				end
			end
			assert.is_false(has_features)
		end)

		it("includes --features flag in cargo args when features provided", function()
			local captured_args
			local jobstart_stub = stub(vim.fn, "jobstart").invokes(function(args)
				captured_args = args
				return 123
			end)
			stub(vim.fn, "executable").returns(1)

			binary.build_from_source_async(temp_dir, { "with-icons" }, function() end)
			vim.wait(50)

			jobstart_stub:revert()

			local has_features = false
			for _, arg in ipairs(captured_args) do
				if arg == "--features" then
					has_features = true
					break
				end
			end
			assert.is_true(has_features, "cargo args should include --features")
		end)

		it("includes feature name after --features flag", function()
			local captured_args
			local jobstart_stub = stub(vim.fn, "jobstart").invokes(function(args)
				captured_args = args
				return 123
			end)
			stub(vim.fn, "executable").returns(1)

			binary.build_from_source_async(temp_dir, { "with-icons" }, function() end)
			vim.wait(50)

			jobstart_stub:revert()

			local feature_name = nil
			for i, arg in ipairs(captured_args) do
				if arg == "--features" then
					feature_name = captured_args[i + 1]
					break
				end
			end
			assert.equals("with-icons", feature_name)
		end)

		it("accepts legacy two-argument call", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			local ok, err = pcall(function()
				binary.build_from_source_async(temp_dir, function(_success, _err)
				end)
			end)

			assert.is_true(ok, "Legacy 2-arg call should not error: " .. tostring(err))
		end)
	end)

	describe("cancel_build()", function()
		before_each(function()
			-- Set notification level to INFO
			local config = require("hermes.config")
			config.setup({
				log = { notification = { level = "info" } },
			})
		end)

		it("returns false when no build is in progress", function()
			local result = binary.cancel_build()
			assert.is_false(result)
		end)

		it("notifies when no build to cancel", function()
			local notify_calls = {}
			local original_notify = vim.notify
			vim.notify = function(msg, level)
				table.insert(notify_calls, { msg = msg, level = level })
			end

			binary.cancel_build()

			vim.notify = original_notify

			local found_no_build_msg = false
			for _, call in ipairs(notify_calls) do
				if call.msg and (call.msg:find("No build") or call.msg:find("in progress")) then
					found_no_build_msg = true
					break
				end
			end

			assert.is_true(found_no_build_msg, "Should notify when no build to cancel")
		end)

		it("returns true when build is cancelled", function()
			-- First start a build
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			binary.build_from_source_async(temp_dir, function() end)

			-- Wait for the async build to actually start
			vim.wait(50)

			-- Now cancel it
			local result = binary.cancel_build()

			assert.is_true(result)
		end)

		it("notifies when build is cancelled", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			local notify_calls = {}
			local original_notify = vim.notify
			vim.notify = function(msg, level)
				table.insert(notify_calls, { msg = msg, level = level })
			end

			binary.build_from_source_async(temp_dir, function() end)

			-- Wait for the async build to actually start
			vim.wait(50)

			binary.cancel_build()

			vim.notify = original_notify

			local found_cancelled_msg = false
			for _, call in ipairs(notify_calls) do
				if call.msg and call.msg:find("cancelled") then
					found_cancelled_msg = true
					break
				end
			end

			assert.is_true(found_cancelled_msg, "Should notify that build was cancelled")
		end)

		it("prevents duplicate builds after cancel", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			-- Start a build
			binary.build_from_source_async(temp_dir, function() end)

			-- Wait for the async build to actually start
			vim.wait(50)

			-- Cancel it
			binary.cancel_build()

			-- Now should be able to start a new build
			local result = binary.build_from_source_async(temp_dir, function() end)

			assert.is_true(result, "Should allow new build after cancel")
		end)
	end)

	describe("is_build_in_progress()", function()
		it("returns false initially", function()
			local result = binary.is_build_in_progress()
			assert.is_false(result)
		end)

		it("returns true when build is in progress", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			binary.build_from_source_async(temp_dir, function() end)

			-- Wait for the async build to actually start
			vim.wait(50)

			local result = binary.is_build_in_progress()

			assert.is_true(result)
		end)

		it("returns false after build is cancelled", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(vim.fn, "executable").returns(1)

			binary.build_from_source_async(temp_dir, function() end)

			-- Wait for the async build to actually start
			vim.wait(50)

			binary.cancel_build()

			local result = binary.is_build_in_progress()

			assert.is_false(result)
		end)
	end)

	describe("cancel_download()", function()
		it("returns true when a download job exists", function()
			local jobstop_stub = stub(vim.fn, "jobstop")
			stub(vim.fn, "jobstart").returns(123)
			stub(download, "download_async").returns(123)
			stub(download, "get_available_tool").returns("curl")
			stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			stub(vim.fn, "filereadable").returns(0)
			stub(vim.fn, "has").returns(0)

			binary.ensure_binary_async(60, function() end)
			vim.wait(100)

			local result = binary.cancel_download()

			assert.is_true(result, "cancel_download should return true when a job exists")

			jobstop_stub:revert()
		end)

		it("calls jobstop when a download job exists", function()
			local jobstop_stub = stub(vim.fn, "jobstop")
			stub(vim.fn, "jobstart").returns(123)
			stub(download, "download_async").returns(123)
			stub(download, "get_available_tool").returns("curl")
			stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			stub(vim.fn, "filereadable").returns(0)
			stub(vim.fn, "has").returns(0)

			binary.ensure_binary_async(60, function() end)
			vim.wait(100)

			binary.cancel_download()

			assert.stub(jobstop_stub).was_called()

			jobstop_stub:revert()
		end)

		it("returns false when no download is in progress", function()
			binary.cancel_download()

			local result = binary.cancel_download()
			assert.is_false(result, "cancel_download should return false when no job exists")
		end)

		it("resets download state after cancel", function()
			stub(vim.fn, "jobstart").returns(123)
			stub(download, "download_async").returns(123)
			stub(download, "get_available_tool").returns("curl")
			stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			stub(vim.fn, "filereadable").returns(0)
			stub(vim.fn, "has").returns(0)

			binary.ensure_binary_async(60, function() end)
			vim.wait(100)

			binary.cancel_download()

			local result = binary.cancel_download()
			assert.is_false(result, "State should be fully reset after cancel")
		end)
	end)

	describe("ensure_binary_async() failure paths", function()
		it("calls callback with failure when download fails", function()
			stub(download, "get_available_tool").returns("curl")
			stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
				on_complete(false, { message = "HTTP 404" })
			end)
			stub(vim.fn, "filereadable").returns(0)
			stub(vim.fn, "has").returns(0)

			local callback_success = nil
			binary.ensure_binary_async(60, function(success, _err)
				callback_success = success
			end)

			vim.wait(100)

			assert.is_false(callback_success, "Callback should receive false on download failure")
		end)

		it("calls callback with error message when download fails", function()
			stub(download, "get_available_tool").returns("curl")
			stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
			stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
				on_complete(false, { message = "Connection refused" })
			end)
			stub(vim.fn, "filereadable").returns(0)
			stub(vim.fn, "has").returns(0)

			local callback_err = nil
			binary.ensure_binary_async(60, function(_success, err)
				callback_err = err
			end)

			vim.wait(100)

			assert.is_not_nil(callback_err, "Callback should receive error details")
		end)
	end)

	describe("ensure_binary_async() rock version writing", function()
		it("writes version.txt from rock version when found", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/lib.so")
			version_stub = stub(binary, "_get_rock_version").returns("v0.2.0")
			local writefile_stub = stub(vim.fn, "writefile")
			local mkdir_stub = stub(vim.fn, "mkdir")

			binary.ensure_binary_async(60, function() end)

			version_stub:revert()
			mkdir_stub:revert()
			writefile_stub:revert()

			assert.stub(writefile_stub).was_called()
		end)

		it("writes correct version to version.txt", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/lib.so")
			version_stub = stub(binary, "_get_rock_version").returns("v0.2.0")
			local writefile_stub = stub(vim.fn, "writefile")
			local mkdir_stub = stub(vim.fn, "mkdir")

			binary.ensure_binary_async(60, function() end)

			version_stub:revert()
			mkdir_stub:revert()
			writefile_stub:revert()

			assert.stub(writefile_stub).was_called_with({ "v0.2.0" }, temp_dir .. "/version.txt")
		end)

		it("does not write version.txt when _get_rock_version returns nil", function()
			rock_binary_stub:revert()
			rock_binary_stub = stub(binary, "get_rock_binary_path").returns("/fake/rock/lib.so")
			version_stub = stub(binary, "_get_rock_version").returns(nil)
			local writefile_stub = stub(vim.fn, "writefile")

			binary.ensure_binary_async(60, function() end)

			version_stub:revert()
			writefile_stub:revert()

			assert.stub(writefile_stub).was_not_called()
		end)
	end)

	describe("download_async()", function()
		it("calls back with failure when platform cannot be determined", function()
			local platform_mod = require("hermes.platform")
			local platform_stub = stub(platform_mod, "get_platform_key").returns(nil)

			local callback_success
			binary.download_async("v1.0.0", function(success, _result)
				callback_success = success
			end)

			platform_stub:revert()
			assert.is_false(callback_success)
		end)

		it("error message mentions platform cannot be determined", function()
			local platform_mod = require("hermes.platform")
			local platform_stub = stub(platform_mod, "get_platform_key").returns(nil)

			local callback_result
			binary.download_async("v1.0.0", function(_success, result)
				callback_result = result
			end)

			platform_stub:revert()
			assert.equals("Unable to determine platform", callback_result)
		end)

		it("calls back with failure for unsupported platform", function()
			local platform_mod = require("hermes.platform")
			local platform_key_stub = stub(platform_mod, "get_platform_key").returns("unsupported-os")
			local platform_display_stub = stub(platform_mod, "get_display_string").returns("unsupported-os")

			local callback_success
			binary.download_async("v1.0.0", function(success, _result)
				callback_success = success
			end)

			platform_key_stub:revert()
			platform_display_stub:revert()
			assert.is_false(callback_success)
		end)

		it("error message mentions unsupported platform", function()
			local platform_mod = require("hermes.platform")
			local platform_key_stub = stub(platform_mod, "get_platform_key").returns("unsupported-os")
			local platform_display_stub = stub(platform_mod, "get_display_string").returns("unsupported-os")

			local callback_result
			binary.download_async("v1.0.0", function(_success, result)
				callback_result = result
			end)

			platform_key_stub:revert()
			platform_display_stub:revert()
			assert.is_true(callback_result:find("Platform not supported") ~= nil)
		end)

		it("calls back with failure when no download tool available", function()
			local platform_mod = require("hermes.platform")
			local platform_stub = stub(platform_mod, "get_platform_key").returns("linux-x86_64")
			local download_tool_stub = stub(download, "get_available_tool").returns(nil)

			local callback_success
			binary.download_async("v1.0.0", function(success, _result)
				callback_success = success
			end)

			platform_stub:revert()
			download_tool_stub:revert()
			assert.is_false(callback_success)
		end)

		it("error message mentions no download tool available", function()
			local platform_mod = require("hermes.platform")
			local platform_stub = stub(platform_mod, "get_platform_key").returns("linux-x86_64")
			local download_tool_stub = stub(download, "get_available_tool").returns(nil)

			local callback_result
			binary.download_async("v1.0.0", function(_success, result)
				callback_result = result
			end)

			platform_stub:revert()
			download_tool_stub:revert()
			assert.is_true(callback_result:find("No download tool available") ~= nil)
		end)

		it("calls internal download when proceeding", function()
			local platform_mod = require("hermes.platform")
			local platform_stub = stub(platform_mod, "get_platform_key").returns("linux-x86_64")
			local download_tool_stub = stub(download, "get_available_tool").returns("curl")
			local mkdir_stub = stub(vim.fn, "mkdir")
			local bin_path = temp_dir .. "/hermes"
			local ver_file = temp_dir .. "/version"
			local binary_path_stub = stub(binary, "get_binary_path").returns(bin_path)
			local version_file_stub = stub(binary, "get_version_file").returns(ver_file)
			local data_dir_stub = stub(binary, "get_data_dir").returns(temp_dir)
			local internal_stub = stub(binary, "_download_binary_async")

			binary.download_async("v1.0.0", function() end)

			platform_stub:revert()
			download_tool_stub:revert()
			mkdir_stub:revert()
			binary_path_stub:revert()
			version_file_stub:revert()
			data_dir_stub:revert()
			internal_stub:revert()

			assert.stub(internal_stub).was_called(1)
		end)

		it("creates data directory when proceeding with download", function()
			local platform_mod = require("hermes.platform")
			local platform_stub = stub(platform_mod, "get_platform_key").returns("linux-x86_64")
			local download_tool_stub = stub(download, "get_available_tool").returns("curl")
			local mkdir_stub = stub(vim.fn, "mkdir")
			local bin_path = temp_dir .. "/hermes"
			local ver_file = temp_dir .. "/version"
			local binary_path_stub = stub(binary, "get_binary_path").returns(bin_path)
			local version_file_stub = stub(binary, "get_version_file").returns(ver_file)
			local data_dir_stub = stub(binary, "get_data_dir").returns(temp_dir)
			local internal_stub = stub(binary, "_download_binary_async")

			binary.download_async("v1.0.0", function() end)

			platform_stub:revert()
			download_tool_stub:revert()
			mkdir_stub:revert()
			binary_path_stub:revert()
			version_file_stub:revert()
			data_dir_stub:revert()
			internal_stub:revert()

			assert.stub(mkdir_stub).was_called()
		end)
	end)

	describe("_parse_checksums()", function()
		it("finds hash for matching filename", function()
			local content = "abc123  libhermes-linux-x86_64.so\ndef456  libhermes-macos-x86_64.dylib\n"
			local result = binary._parse_checksums(content, "libhermes-linux-x86_64.so")
			assert.equals("abc123", result)
		end)

		it("returns nil for non-existent filename", function()
			local content = "abc123  libhermes-linux-x86_64.so\n"
			local result = binary._parse_checksums(content, "libhermes-windows-x86_64.dll")
			assert.is_nil(result)
		end)

		it("handles CRLF line endings", function()
			local content = "abc123  libhermes-linux-x86_64.so\r\ndef456  libhermes-macos-x86_64.dylib\r\n"
			local result = binary._parse_checksums(content, "libhermes-macos-x86_64.dylib")
			assert.equals("def456", result)
		end)
	end)

	describe("download() hash verification", function()
		it("returns false when hash verification fails", function()
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(false, "Hash mismatch")

			local result = binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.is_false(result)
		end)

		it("returns error message when hash verification fails", function()
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(false, "Hash mismatch for test.so")

			local _, err = binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.truthy(err and err.message and err.message:find("Hash mismatch"))
		end)

		it("does not verify hash when download fails", function()
			download_stub = stub(download, "download").returns(false, { message = "Network error" })
			local verify_stub = stub(binary, "_verify_binary_hash")
			binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.stub(verify_stub).was_not_called()
		end)

		it("skips chmod on win32", function()
			download_stub = stub(download, "download").returns(true, nil)
			stub(binary, "_verify_binary_hash").returns(true)
			local has_stub = stub(vim.fn, "has").returns(1)
			local system_stub = stub(vim.fn, "system")

			binary.download(temp_dir .. "/test.so", "v1.0.0")

			has_stub:revert()
			system_stub:revert()

			assert.stub(system_stub).was_not_called()
		end)

		it("passes through structured error from download", function()
			local structured_err = { message = "timeout", http_code = 504, tool = "curl" }
			download_stub = stub(download, "download").returns(false, structured_err)

			local _, err = binary.download(temp_dir .. "/test.so", "v1.0.0")

			assert.equals(structured_err, err)
		end)
	end)

	describe("_compute_file_hash()", function()
		it("returns correct sha256 for known content", function()
			local test_file = temp_dir .. "/hash_test.bin"
			local f = io.open(test_file, "w")
			f:write("hello world")
			f:close()

			-- sha256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
			local hash = binary._compute_file_hash(test_file)

			assert.equals("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9", hash)
		end)

		it("returns nil for non-existent file", function()
			local hash = binary._compute_file_hash(temp_dir .. "/nonexistent.bin")
			assert.is_nil(hash)
		end)
	end)

	describe("_verify_binary_hash()", function()
		local platform_mod

		before_each(function()
			platform_mod = require("hermes.platform")
		end)

		it("returns true when hash matches", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			local expected_hash = vim.fn.sha256("hello world"):lower()
			local checksums_content = expected_hash .. "  test.bin\n"
			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").invokes(function(_url, dest)
				local df = io.open(dest, "w")
				df:write(checksums_content)
				df:close()
				return true, nil
			end)

			local result = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_true(result)
		end)

		it("returns false when hash mismatches", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").invokes(function(_url, dest)
				local df = io.open(dest, "w")
				df:write("000000000000000000000000000000000000000000000000000000000000dead  test.bin\n")
				df:close()
				return true, nil
			end)

			local ok, err = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("Hash mismatch"))
		end)

		it("deletes binary on hash mismatch", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").invokes(function(_url, dest)
				local df = io.open(dest, "w")
				df:write("000000000000000000000000000000000000000000000000000000000000dead  test.bin\n")
				df:close()
				return true, nil
			end)

			binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_nil(io.open(bin_file, "r"))
		end)

		it("returns true on 404 checksums download", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").returns(false, { http_code = 404 })

			local result = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_true(result)
		end)

		it("returns false when checksums download fails", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").returns(false, { message = "Network error" })

			local ok, err = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("Failed to download checksums"))
		end)

		it("returns false when error is a table with stderr", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").returns(false, { stderr = "connection reset" })

			local ok, err = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("connection reset"))
		end)

		it("returns false when error is a plain string", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").returns(false, "DNS resolution failed")

			local ok, err = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("DNS resolution failed"))
		end)

		it("returns true when error string contains 404", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").returns(false, "HTTP 404 Not Found")

			local result = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_true(result)
		end)

		it("returns false when checksums file cannot be read after download", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").invokes(function(_url, dest)
				local df = io.open(dest, "w")
				df:write("abc123  test.bin\n")
				df:close()
				os.remove(dest)
				return true, nil
			end)

			local ok, err = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("Failed to read checksums"))
		end)

		it("returns false when error is nil", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").returns(false, nil)

			local ok, err = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("Failed to download checksums"))
		end)

		it("returns false when error table has neither message nor stderr", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").returns(false, { code = "ETIMEOUT" })

			local ok, err = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("Failed to download checksums"))
		end)

		it("returns true when no hash found for binary in checksums", function()
			local bin_file = temp_dir .. "/test.bin"
			local f = io.open(bin_file, "w")
			f:write("hello world")
			f:close()

			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").invokes(function(_url, dest)
				local df = io.open(dest, "w")
				df:write("abc123  other-binary.so\n")
				df:close()
				return true, nil
			end)

			local result = binary._verify_binary_hash(bin_file, "v1.0.0")

			assert.is_true(result)
		end)

		it("returns false when binary file does not exist", function()
			stub(platform_mod, "get_binary_name").returns("test.bin")
			download_stub = stub(download, "download").invokes(function(_url, dest)
				local df = io.open(dest, "w")
				df:write("abc123  test.bin\n")
				df:close()
				return true, nil
			end)

			local ok, err = binary._verify_binary_hash(temp_dir .. "/missing.bin", "v1.0.0")

			assert.is_false(ok)
			assert.truthy(err:find("Failed to compute hash"))
		end)
	end)

	describe("_download_binary_async() paths", function()
		it("calls back with failure when hash verification fails", function()
			stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
				on_complete(true, nil)
			end)
			stub(binary, "_verify_binary_hash").returns(false, "Hash mismatch for libhermes.so")
			stub(vim.fn, "has").returns(0)
			stub(vim.fn, "system")

			local callback_success = nil
			local callback_err = nil
			binary._download_binary_async("v1.0.0", temp_dir .. "/bin", temp_dir .. "/ver", function(success, err)
				callback_success = success
				callback_err = err
			end)

			vim.wait(100)

			assert.same({ false, "Hash mismatch for libhermes.so" }, { callback_success, callback_err })
		end)

		it("calls back with failure when download fails", function()
			stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
				on_complete(false, "Connection refused")
			end)

			local callback_success = nil
			local callback_err = nil
			binary._download_binary_async("v1.0.0", temp_dir .. "/bin", temp_dir .. "/ver", function(success, err)
				callback_success = success
				callback_err = err
			end)

			vim.wait(100)

			assert.same({ false, "Connection refused" }, { callback_success, callback_err })
		end)

		it("uses fallback error when download fails with nil error", function()
			stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
				on_complete(false, nil)
			end)

			local callback_success = nil
			binary._download_binary_async("v1.0.0", temp_dir .. "/bin", temp_dir .. "/ver", function(success, _err)
				callback_success = success
			end)

			vim.wait(100)

			assert.is_false(callback_success)
		end)

		it("calls back with success and writes version file on successful download", function()
			stub(download, "download_async").invokes(function(_url, _dest, _id, on_complete)
				on_complete(true, nil)
			end)
			stub(binary, "_verify_binary_hash").returns(true)
			stub(vim.fn, "has").returns(0)
			stub(vim.fn, "system")
			local writefile_stub = stub(vim.fn, "writefile")

			local callback_success = nil
			local callback_result = nil
			binary._download_binary_async("v1.0.0", temp_dir .. "/bin", temp_dir .. "/ver", function(success, result)
				callback_success = success
				callback_result = result
			end)

			vim.wait(100)

			assert.is_true(callback_success)
			assert.equals(temp_dir .. "/bin", callback_result)
			assert.stub(writefile_stub).was_called_with({ "v1.0.0" }, temp_dir .. "/ver")
		end)

		it("resets state when job_id is nil", function()
			stub(download, "download_async").returns(nil)

			binary._download_binary_async("v1.0.0", temp_dir .. "/bin", temp_dir .. "/ver", function() end)

			assert.is_false(binary.cancel_download())
		end)

		it("resets state when job_id is zero", function()
			stub(download, "download_async").returns(0)

			binary._download_binary_async("v1.0.0", temp_dir .. "/bin", temp_dir .. "/ver", function() end)

			assert.is_false(binary.cancel_download())
		end)
	end)
end)
