-- Test helper utilities for Lua tests
-- Provides temporary directory management and HTTP mocking

local stub = require("luassert.stub")
local M = {}

---Create a temporary directory for testing
---@return string temp_path Path to temp directory
function M.create_temp_dir()
	-- Use vim.fn.tempname() to get a unique temp path
	local base = vim.fn.tempname()
	local temp_path = base .. "_hermes_test"

	-- Create directory structure
	vim.fn.mkdir(temp_path, "p")
	vim.fn.mkdir(temp_path .. "/hermes", "p")

	return temp_path .. "/hermes"
end

---Clean up temporary directory
---@param temp_path string Path to clean up
function M.cleanup_temp_dir(temp_path)
	if temp_path and vim.fn.isdirectory(temp_path) == 1 then
		-- Remove directory and all contents recursively
		vim.fn.delete(temp_path, "rf")
	end
end

---Mock successful download by stubbing the download module
---Returns a stub object that can be reverted with stub_obj:revert()
---@return table stub_obj Stub object with :revert() method
function M.mock_download_success()
	local download = require("hermes.download")
	return stub(download, "download").returns(true, nil)
end

---Mock failed download by stubbing the download module
---Returns a stub object that can be reverted with stub_obj:revert()
---@return table stub_obj Stub object with :revert() method
function M.mock_download_failure()
	local download = require("hermes.download")
	return stub(download, "download").returns(false, "Download failed")
end

---Create a minimal init.lua for isolated Neovim instances
---@return string init_content
function M.get_minimal_init()
	return [[
-- Minimal init.lua for testing
vim.opt.runtimepath:append(vim.fn.getcwd())

-- Add lua directory to package path
package.path = package.path .. ";" .. vim.fn.getcwd() .. "/lua/?.lua"
package.path = package.path .. ";" .. vim.fn.getcwd() .. "/lua/?/init.lua"

-- Disable some plugins/ui for faster tests
vim.opt.swapfile = false
vim.opt.backup = false
vim.opt.writebackup = false
vim.opt.updatecount = 0

-- Ensure temp directory cleanup on exit
vim.api.nvim_create_autocmd("VimLeavePre", {
  callback = function()
    -- Cleanup will be handled by test framework
  end
})
]]
end

return M

