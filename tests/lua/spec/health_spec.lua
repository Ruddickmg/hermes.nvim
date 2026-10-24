-- Tests for lua/hermes/health.lua
-- Health check provides :checkhealth hermes diagnostics

local stub = require("luassert.stub")

describe("hermes.health", function()
	local health
	local calls
	local orig_health
	local nix_install_stub

	before_each(function()
		-- Track all vim.health calls
		calls = { ok = {}, error = {}, warn = {}, info = {}, start = {} }
		orig_health = {
			ok = vim.health.ok,
			error = vim.health.error,
			warn = vim.health.warn,
			info = vim.health.info,
			start = vim.health.start,
		}
		vim.health.ok = function(msg)
			table.insert(calls.ok, msg)
		end
		vim.health.error = function(msg)
			table.insert(calls.error, msg)
		end
		vim.health.warn = function(msg)
			table.insert(calls.warn, msg)
		end
		vim.health.info = function(msg)
			table.insert(calls.info, msg)
		end
		vim.health.start = function(msg)
			table.insert(calls.start, msg)
		end

		package.loaded["hermes.health"] = nil
		package.loaded["hermes.config"] = nil
		package.loaded["hermes.binary"] = nil
		package.loaded["hermes.version"] = nil
		package.loaded["hermes.platform"] = nil
		package.loaded["hermes.download"] = nil
		package.loaded["hermes"] = nil

		-- Keep the host environment from affecting default-dependent assertions
		local binary = require("hermes.binary")
		nix_install_stub = stub(binary, "is_nix_install").returns(false)

		-- Ensure config is initialized before requiring health
		local config = require("hermes.config")
		config.setup({})

		health = require("hermes.health")
	end)

	after_each(function()
		vim.health.ok = orig_health.ok
		vim.health.error = orig_health.error
		vim.health.warn = orig_health.warn
		vim.health.info = orig_health.info
		vim.health.start = orig_health.start
		if nix_install_stub then
			nix_install_stub:revert()
			nix_install_stub = nil
		end
	end)

	local function has_call(list, pattern)
		for _, msg in ipairs(list) do
			if msg:match(pattern) then
				return true
			end
		end
		return false
	end

	describe("check()", function()
		it("starts with Neovim section", function()
			health.check()
			assert.equals("Neovim", calls.start[1])
		end)

		it("reports ok for supported Neovim version", function()
			health.check()
			assert.is_true(has_call(calls.ok, "Neovim >= 0%.11"))
		end)

		it("reports error for unsupported Neovim version", function()
			local orig_version = vim.version
			vim.version = function()
				return { major = 0, minor = 10, patch = 0 }
			end
			health.check()
			assert.is_true(has_call(calls.error, "Neovim >= 0%.11 is required"))
			vim.version = orig_version
		end)

		it("includes Hermes Binary section", function()
			health.check()
			assert.is_true(has_call(calls.start, "Hermes Binary"))
		end)

		it("shows rock tree annotation when binary is from rock tree", function()
			local binary = require("hermes.binary")
			local active_stub = stub(binary, "get_active_binary_path").returns("/fake/rock/libhermes.so")
			local data_stub = stub(binary, "get_binary_path").returns("/fake/data/libhermes.so")
			health.check()
			active_stub:revert()
			data_stub:revert()
			assert.is_true(has_call(calls.info, "%(rock tree%)"))
		end)

		it("reports ok when binary is ready", function()
			local hermes = require("hermes")
			local get_state = hermes.get_loading_state
			hermes.get_loading_state = function() return "READY" end
			health.check()
			hermes.get_loading_state = get_state
			assert.is_true(has_call(calls.ok, "Binary loaded and ready"))
		end)

		it("reports error when binary loading failed", function()
			local hermes = require("hermes")
			local get_state = hermes.get_loading_state
			hermes.get_loading_state = function() return "FAILED" end
			hermes._set_loading_error("test failure")
			health.check()
			hermes.get_loading_state = get_state
			assert.is_true(has_call(calls.error, "Binary loading failed"))
		end)

		it("reports warn when binary is downloading", function()
			local hermes = require("hermes")
			local get_state = hermes.get_loading_state
			hermes.get_loading_state = function() return "DOWNLOADING" end
			health.check()
			hermes.get_loading_state = get_state
			assert.is_true(has_call(calls.warn, "being downloaded"))
		end)

		it("reports info when binary not yet loaded", function()
			local hermes = require("hermes")
			local get_state = hermes.get_loading_state
			hermes.get_loading_state = function() return "NOT_LOADED" end
			health.check()
			hermes.get_loading_state = get_state
			assert.is_true(has_call(calls.info, "not loaded yet"))
		end)

		it("reports ok when binary file exists", function()
			local fr = stub(vim.fn, "filereadable").returns(1)
			local gfs = stub(vim.fn, "getfsize").returns(1234)
			health.check()
			fr:revert()
			gfs:revert()
			assert.is_true(has_call(calls.ok, "Binary exists"))
		end)

		it("reports error when binary file missing with auto-download enabled", function()
			local fr = stub(vim.fn, "filereadable").returns(0)
			health.check()
			fr:revert()
			assert.is_true(has_call(calls.error, "will download on first use"))
		end)

		it("reports error when binary file missing with auto-download disabled", function()
			local config = require("hermes.config")
			config.setup({ download = { auto = false } })
			local fr = stub(vim.fn, "filereadable").returns(0)
			health.check()
			fr:revert()
			assert.is_true(has_call(calls.error, "needs to be built from source"))
		end)

		it("reports ok for cargo when binary missing and auto-download disabled", function()
			local config = require("hermes.config")
			config.setup({ download = { auto = false } })
			local fr = stub(vim.fn, "filereadable").returns(0)
			local exe = stub(vim.fn, "executable").returns(1)
			health.check()
			fr:revert()
			exe:revert()
			assert.is_true(has_call(calls.ok, "Rust/Cargo is installed"))
		end)

		it("reports error for cargo when binary missing and auto-download disabled", function()
			local config = require("hermes.config")
			config.setup({ download = { auto = false } })
			local fr = stub(vim.fn, "filereadable").returns(0)
			local exe = stub(vim.fn, "executable").returns(0)
			health.check()
			fr:revert()
			exe:revert()
			assert.is_true(has_call(calls.error, "Rust/Cargo is not installed"))
		end)

		it("reports searched plugin root when binary missing on nix install", function()
			local binary = require("hermes.binary")
			local nix_stub = stub(binary, "is_nix_install").returns(true)
			local root_stub = stub(binary, "_get_rock_root").returns("/nix/store/fake-hermes-nvim")
			local fr = stub(vim.fn, "filereadable").returns(0)
			health.check()
			nix_stub:revert()
			root_stub:revert()
			fr:revert()
			assert.is_true(has_call(calls.info, "Plugin root searched: /nix/store/fake%-hermes%-nvim"))
		end)

		it("lists checked library candidates when binary missing on nix install", function()
			local binary = require("hermes.binary")
			local nix_stub = stub(binary, "is_nix_install").returns(true)
			local root_stub = stub(binary, "_get_rock_root").returns("/nix/store/fake-hermes-nvim")
			local fr = stub(vim.fn, "filereadable").returns(0)
			health.check()
			nix_stub:revert()
			root_stub:revert()
			fr:revert()
			assert.is_true(has_call(calls.info, "Checked: /nix/store/fake%-hermes%-nvim/lib/libhermes%."))
		end)

		it("reports undetectable plugin root when nix rock root is nil", function()
			local binary = require("hermes.binary")
			local nix_stub = stub(binary, "is_nix_install").returns(true)
			local root_stub = stub(binary, "_get_rock_root").returns(nil)
			local fr = stub(vim.fn, "filereadable").returns(0)
			health.check()
			nix_stub:revert()
			root_stub:revert()
			fr:revert()
			assert.is_true(has_call(calls.info, "Could not determine plugin root"))
		end)

		it("includes Version section", function()
			health.check()
			assert.is_true(has_call(calls.start, "Version"))
		end)

		it("reports info for wanted version", function()
			health.check()
			assert.is_true(has_call(calls.info, "Wanted:"))
		end)

		it("reports source build when auto-download disabled", function()
			local config = require("hermes.config")
			config.setup({ download = { auto = false } })
			health.check()
			assert.is_true(has_call(calls.info, "Built from source"))
		end)

		it("reports Installed via LuaRocks when luarocks install detected", function()
			local binary = require("hermes.binary")
			local rock_stub = stub(binary, "is_luarocks_install").returns(true)
			health.check()
			rock_stub:revert()
			assert.is_true(has_call(calls.info, "Installed via LuaRocks"))
		end)

		it("omits Installed via LuaRocks when not a luarocks install", function()
			health.check()
			assert.is_false(has_call(calls.info, "Installed via LuaRocks"))
		end)

		it("reports Installed via Nix when nix install detected", function()
			local binary = require("hermes.binary")
			local nix_stub = stub(binary, "is_nix_install").returns(true)
			health.check()
			nix_stub:revert()
			assert.is_true(has_call(calls.info, "Installed via Nix"))
		end)

		it("omits Installed via Nix when not a nix install", function()
			health.check()
			assert.is_false(has_call(calls.info, "Installed via Nix"))
		end)

		it("includes Platform section", function()
			health.check()
			assert.is_true(has_call(calls.start, "Platform"))
		end)

		it("reports ok for supported platform", function()
			health.check()
			assert.is_true(has_call(calls.ok, "Platform is supported"))
		end)

		it("reports error for unsupported platform and notes source build", function()
			local platform = require("hermes.platform")
			local orig = platform.is_supported
			platform.is_supported = function() return false end
			health.check()
			platform.is_supported = orig
			assert.is_true(has_call(calls.error, "build from source required"))
		end)

		it("reports error for missing cargo on unsupported platform", function()
			local platform = require("hermes.platform")
			local orig = platform.is_supported
			platform.is_supported = function() return false end
			local exe = stub(vim.fn, "executable").returns(0)
			health.check()
			platform.is_supported = orig
			exe:revert()
			assert.is_true(has_call(calls.error, "Rust/Cargo is not installed"))
		end)

		it("reports ok for cargo on unsupported platform", function()
			local platform = require("hermes.platform")
			local orig = platform.is_supported
			platform.is_supported = function() return false end
			local exe = stub(vim.fn, "executable").returns(1)
			health.check()
			platform.is_supported = orig
			exe:revert()
			assert.is_true(has_call(calls.ok, "Rust/Cargo is installed"))
		end)

		it("includes Download Tools section", function()
			health.check()
			assert.is_true(has_call(calls.start, "Download Tools"))
		end)

		it("reports info or ok for each download tool", function()
			health.check()
			local found_curl = has_call(calls.ok, "curl") or has_call(calls.warn, "curl")
			local found_wget = has_call(calls.ok, "wget") or has_call(calls.info, "wget")
			local found_ps = has_call(calls.ok, "PowerShell") or has_call(calls.info, "PowerShell")
			assert.is_true(found_curl, "Should report curl status")
			assert.is_true(found_wget, "Should report wget status")
			assert.is_true(found_ps, "Should report PowerShell status")
		end)

		it("includes Log Files section", function()
			health.check()
			assert.is_true(has_call(calls.start, "Log Files"))
		end)

		it("reports info for log directory", function()
			health.check()
			assert.is_true(has_call(calls.info, "Directory:"))
		end)

		it("reports info when no log files found", function()
			local isdir = stub(vim.fn, "isdirectory").returns(0)
			health.check()
			isdir:revert()
			assert.is_true(has_call(calls.info, "No log files found"))
		end)

		it("reports ok when log directory is writable", function()
			-- Default config has file logging disabled, so writable check still reports ok
			local access = stub(vim.uv, "fs_access").returns(true)
			health.check()
			access:revert()
			assert.is_true(has_call(calls.ok, "Log directory is writable"))
		end)

		it("reports error when log directory not writable and logging enabled", function()
			local config = require("hermes.config")
			config.setup({ log = { file = { level = "debug" } } })
			local access = stub(vim.uv, "fs_access").returns(false)
			health.check()
			access:revert()
			assert.is_true(has_call(calls.error, "Log directory is not writable"))
		end)

		it("reports warn when log directory not writable and logging disabled", function()
			local access = stub(vim.uv, "fs_access").returns(false)
			health.check()
			access:revert()
			assert.is_true(has_call(calls.warn, "Log directory is not writable"))
		end)

		it("includes Registry Binaries section", function()
			health.check()
			assert.is_true(has_call(calls.start, "Registry Binaries"))
		end)

		it("reports info when no registry binaries found", function()
			local isdir = stub(vim.fn, "isdirectory").returns(0)
			health.check()
			isdir:revert()
			assert.is_true(has_call(calls.info, "No registry binaries installed"))
		end)

		it("reports ok when registry cache directory is writable", function()
			local access = stub(vim.uv, "fs_access").returns(true)
			health.check()
			access:revert()
			assert.is_true(has_call(calls.ok, "Registry cache directory is writable"))
		end)

		it("reports error when registry cache not writable and binary dist enabled", function()
			local access = stub(vim.uv, "fs_access").returns(false)
			health.check()
			access:revert()
			assert.is_true(has_call(calls.error, "Registry cache directory is not writable"))
		end)

		it("reports warn when registry cache not writable and binary dist disabled", function()
			local config = require("hermes.config")
			config.setup({ distributions = { binary = { enabled = false } } })
			local access = stub(vim.uv, "fs_access").returns(false)
			health.check()
			access:revert()
			assert.is_true(has_call(calls.warn, "Registry cache directory is not writable"))
		end)

		it("reports found registry binaries with ok/warn", function()
			local config = require("hermes.config")
			local cache_dir = config.get_registry_cache_dir()
			vim.fn.mkdir(cache_dir .. "/test-agent/v1.0.0", "p")
			local f = io.open(cache_dir .. "/test-agent/v1.0.0/test-bin", "w")
			f:write("mock")
			f:close()
			-- Make executable on Unix
			if vim.fn.has("win32") ~= 1 and vim.fn.has("win64") ~= 1 then
				vim.fn.system({ "chmod", "+x", cache_dir .. "/test-agent/v1.0.0/test-bin" })
			end
			health.check()
			assert.is_true(has_call(calls.ok, "test%-agent@v1%.0%.0"))
			vim.fn.delete(cache_dir, "rf")
		end)

		it("includes Configuration section", function()
			health.check()
			assert.is_true(has_call(calls.start, "Configuration"))
		end)

		it("reports pretty-printed config via info", function()
			health.check()
			assert.is_true(has_call(calls.info, "Permissions:"))
			assert.is_true(has_call(calls.info, "Terminal:"))
			assert.is_true(has_call(calls.info, "Log %(File%):"))
		end)
	end)
end)
