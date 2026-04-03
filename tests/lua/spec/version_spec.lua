-- Unit tests for lua/hermes/version.lua
-- Tests version management (simplified: no cache)

local stub = require("luassert.stub")

describe("hermes.version", function()
	local version

	before_each(function()
		package.loaded["hermes.version"] = nil
		version = require("hermes.version")
	end)

	describe("get_wanted()", function()
		it("returns latest when version is 'latest'", function()
			-- Stub config to return latest
			local config_stub = stub(require("hermes.config"), "get_version").returns("latest")

			local result = version.get_wanted()

			assert.equals("latest", result)

			config_stub:revert()
		end)

		it("adds 'v' prefix when version doesn't have it", function()
			local config_stub = stub(require("hermes.config"), "get_version").returns("1.2.3")

			local result = version.get_wanted()

			assert.equals("v1.2.3", result)

			config_stub:revert()
		end)

		it("preserves 'v' prefix when version already has it", function()
			local config_stub = stub(require("hermes.config"), "get_version").returns("v1.2.3")

			local result = version.get_wanted()

			assert.equals("v1.2.3", result)

			config_stub:revert()
		end)

		it("handles version with just v prefix", function()
			local config_stub = stub(require("hermes.config"), "get_version").returns("v")

			local result = version.get_wanted()

			assert.equals("v", result)

			config_stub:revert()
		end)

		it("handles empty version string", function()
			local config_stub = stub(require("hermes.config"), "get_version").returns("")

			local result = version.get_wanted()

			assert.equals("v", result)

			config_stub:revert()
		end)

		it("handles version with complex semver", function()
			local config_stub = stub(require("hermes.config"), "get_version").returns("1.0.0-alpha.1")

			local result = version.get_wanted()

			assert.equals("v1.0.0-alpha.1", result)

			config_stub:revert()
		end)

		it("returns 'source' when version is 'source'", function()
			local config_stub = stub(require("hermes.config"), "get_version").returns("source")

			local result = version.get_wanted()

			assert.equals("source", result)

			config_stub:revert()
		end)

		it("does not add 'v' prefix to 'source'", function()
			-- This is the critical test - ensure "source" doesn't become "vsource"
			local config_stub = stub(require("hermes.config"), "get_version").returns("source")

			local result = version.get_wanted()

			-- Must be exactly "source", not "vsource"
			assert.equals("source", result)
			assert.is_not.equals("vsource", result)

			config_stub:revert()
		end)
	end)

	describe("fetch_latest()", function()
		it("returns fallback version on download failure", function()
			local download_stub = stub(require("hermes.download"), "download").returns(false, "Network error")
			local notify_stub = stub(vim, "notify")

			local result = version.fetch_latest()

			-- Should return fallback version
			assert.equals("v0.1.0", result)

			download_stub:revert()
			notify_stub:revert()
		end)

		it("parses version from successful GitHub response", function()
			-- Create a mock temp file with valid JSON response
			local mock_file = os.tmpname()
			local f = io.open(mock_file, "w")
			f:write('{"tag_name": "v2.0.0", "name": "Release v2.0.0"}')
			f:close()

			-- Stub download to succeed and capture the temp file path
			local captured_path
			local download_stub = stub(require("hermes.download"), "download").invokes(function(_url, path)
				captured_path = path
				local uv = vim.uv or vim.loop
				uv.fs_copyfile(mock_file, path)
				return true, nil
			end)
			local notify_stub = stub(vim, "notify")

			-- Call fetch_latest directly to test the parsing logic
			local result = version.fetch_latest()

			-- Cleanup
			os.remove(mock_file)
			if captured_path then
				os.remove(captured_path)
			end

			-- Should have parsed v2.0.0 from the JSON
			assert.equals("v2.0.0", result)

			download_stub:revert()
			notify_stub:revert()
		end)

		it("returns fallback on invalid JSON response", function()
			-- Create a mock temp file with invalid JSON
			local mock_file = os.tmpname()
			local f = io.open(mock_file, "w")
			f:write("invalid json without tag_name")
			f:close()

			local captured_path
			local download_stub = stub(require("hermes.download"), "download").invokes(function(_url, path)
				captured_path = path
				local uv = vim.uv or vim.loop
				uv.fs_copyfile(mock_file, path)
				return true, nil
			end)
			local notify_stub = stub(vim, "notify")

			local result = version.fetch_latest()

			-- Cleanup
			os.remove(mock_file)
			if captured_path then
				os.remove(captured_path)
			end

			-- Should return fallback version (v0.1.0)
			assert.equals("v0.1.0", result)

			download_stub:revert()
			notify_stub:revert()
		end)

		it("returns fallback when file cannot be read", function()
			-- Stub download to succeed but file doesn't exist (can't be opened)
			local download_stub = stub(require("hermes.download"), "download").returns(true, nil)
			local notify_stub = stub(vim, "notify")
			
			-- Don't create the file, so io.open returns nil
			-- The temp file path returned by os.tmpname() won't exist

			local result = version.fetch_latest()

			-- Should return fallback version (v0.1.0)
			assert.equals("v0.1.0", result)

			download_stub:revert()
			notify_stub:revert()
		end)

		it("handles download error with structured error table", function()
			local download_stub = stub(require("hermes.download"), "download").returns(false, {
				message = "Connection timeout",
				http_code = 504,
				tool = "curl"
			})
			local notify_stub = stub(vim, "notify")

			local result = version.fetch_latest()

			-- Should return fallback version
			assert.equals("v0.1.0", result)

			download_stub:revert()
			notify_stub:revert()
		end)

		it("handles download error with nil error message", function()
			local download_stub = stub(require("hermes.download"), "download").returns(false, nil)
			local notify_stub = stub(vim, "notify")

			local result = version.fetch_latest()

			-- Should return fallback version
			assert.equals("v0.1.0", result)

			download_stub:revert()
			notify_stub:revert()
		end)

		it("parses version from GitHub API with additional fields", function()
			-- Create a mock temp file with complete GitHub API response
			local mock_file = os.tmpname()
			local f = io.open(mock_file, "w")
			f:write('{"tag_name": "v1.5.0", "name": "Release v1.5.0", "body": "Test release", "created_at": "2024-01-01"}')
			f:close()

			local captured_path
			local download_stub = stub(require("hermes.download"), "download").invokes(function(_url, path)
				captured_path = path
				local uv = vim.uv or vim.loop
				uv.fs_copyfile(mock_file, path)
				return true, nil
			end)
			local notify_stub = stub(vim, "notify")

			local result = version.fetch_latest()

			-- Cleanup
			os.remove(mock_file)
			if captured_path then
				os.remove(captured_path)
			end

			-- Should parse v1.5.0
			assert.equals("v1.5.0", result)

			download_stub:revert()
			notify_stub:revert()
		end)
	end)
end)