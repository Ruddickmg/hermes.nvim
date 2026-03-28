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
			local config_stub = stub(require("hermes.config"), "get").returns({ version = "latest" })

			local result = version.get_wanted()

			assert.equals("latest", result)

			config_stub:revert()
		end)

		it("adds 'v' prefix when version doesn't have it", function()
			local config_stub = stub(require("hermes.config"), "get").returns({ version = "1.2.3" })

			local result = version.get_wanted()

			assert.equals("v1.2.3", result)

			config_stub:revert()
		end)

		it("preserves 'v' prefix when version already has it", function()
			local config_stub = stub(require("hermes.config"), "get").returns({ version = "v1.2.3" })

			local result = version.get_wanted()

			assert.equals("v1.2.3", result)

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
	end)
end)