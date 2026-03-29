-- Unit tests for lua/hermes/init.lua error handling
-- Tests error formatting and status display functions

local stub = require("luassert.stub")

describe("hermes.init error handling", function()
  local init
  
  before_each(function()
    -- Clear module cache and reload
    package.loaded["hermes.init"] = nil
    package.loaded["hermes.config"] = nil
    
    -- Setup config first to avoid conflicts
    local config = require("hermes.config")
    config.setup({
      download = { version = "latest", auto = false },
    })
    
    init = require("hermes.init")
  end)
  
  after_each(function()
    -- Reset loading state to prevent test pollution
    if init then
      init._set_loading_state("NOT_LOADED")
      init._set_loading_error(nil)
    end
    -- Clear module cache to ensure fresh state for next test
    package.loaded["hermes.init"] = nil
    package.loaded["hermes.config"] = nil
  end)
  
  describe("error formatting", function()
    it("formats plain string errors", function()
      local formatted = init._format_error_for_display("Simple error message")
      
      assert.equals("Simple error message", formatted)
    end)
    
    it("formats structured error with URL", function()
      local err = {
        message = "Download failed",
        url = "https://example.com/file",
      }
      
      local formatted = init._format_error_for_display(err)
      
      assert.truthy(formatted:match("Error: Download failed"))
      assert.truthy(formatted:match("URL: https://example.com/file"))
    end)
    
    it("formats structured error with HTTP code 404", function()
      local err = {
        message = "Not found",
        url = "https://github.com/Ruddickmg/hermes.nvim/releases/download/v1.0.0/file.so",
        http_code = 404,
        tool = "curl",
        exit_code = 0,
      }
      
      local formatted = init._format_error_for_display(err)
      
      assert.truthy(formatted:match("Error: Not found"))
      assert.truthy(formatted:match("URL:"))
      assert.truthy(formatted:match("HTTP Code: 404 %(Not Found%)"))
      assert.truthy(formatted:match("Download Tool: curl"))
      assert.truthy(formatted:match("Exit Code: 0"))
    end)
    
    it("formats structured error with HTTP code 403", function()
      local err = {
        message = "Forbidden",
        http_code = 403,
      }
      
      local formatted = init._format_error_for_display(err)
      
      assert.truthy(formatted:match("HTTP Code: 403 %(Forbidden%)"))
    end)
    
    it("truncates long stderr output", function()
      local long_stderr = string.rep("Error details ", 50)  -- 650 chars
      local err = {
        message = "Download failed",
        stderr = long_stderr,
      }
      
      local formatted = init._format_error_for_display(err)
      
      -- Should be truncated with ...
      assert.truthy(formatted:match("%.%.%."))
      -- Should not contain full stderr (match returns nil when not found)
      assert.is_nil(formatted:match(long_stderr))
    end)
  end)
  
  describe("error suggestions", function()
    it("suggests version check for 404 errors", function()
      local err = {
        message = "Not found",
        http_code = 404,
      }
      
      local suggestion = init._get_error_suggestion(err)
      
      assert.truthy(suggestion:match("Version not found"))
      assert.truthy(suggestion:match("releases"))
    end)
    
    it("suggests build from source for 403 errors", function()
      local err = {
        message = "Forbidden",
        http_code = 403,
      }
      
      local suggestion = init._get_error_suggestion(err)
      
      assert.truthy(suggestion:match("Download blocked"))
      assert.truthy(suggestion:match(":Hermes build"))
    end)
    
    it("suggests waiting for 500 errors", function()
      local err = {
        message = "Server Error",
        http_code = 500,
      }
      
      local suggestion = init._get_error_suggestion(err)
      
      assert.truthy(suggestion:match("GitHub server error"))
      assert.truthy(suggestion:match("Wait"))
    end)
    
    it("suggests installing curl for missing tool errors", function()
      local err = {
        message = "No download tool available (tried curl, wget, PowerShell)",
      }
      
      local suggestion = init._get_error_suggestion(err)
      
      assert.truthy(suggestion:match("Install curl"))
    end)
    
    it("suggests building from source for plain string errors", function()
      local err = "Connection timeout"
      
      local suggestion = init._get_error_suggestion(err)
      
      assert.truthy(suggestion:match(":Hermes build"))
    end)
    
    it("suggests building from source for empty file errors", function()
      local err = {
        message = "Downloaded file is too small or empty",
      }
      
      local suggestion = init._get_error_suggestion(err)
      
      assert.truthy(suggestion:match("Download incomplete"))
    end)
  end)
  
  describe("loading state management", function()
    it("tracks NOT_LOADED initial state", function()
      assert.equals("NOT_LOADED", init.get_loading_state())
    end)
    
    it("tracks loading state changes", function()
      -- Initial state
      assert.equals("NOT_LOADED", init.get_loading_state())
      
      -- Simulate state transitions
      init._set_loading_state("DOWNLOADING")
      assert.equals("DOWNLOADING", init.get_loading_state())
      
      init._set_loading_state("FAILED")
      assert.equals("FAILED", init.get_loading_state())
    end)
    
    it("tracks loading errors", function()
      local test_error = { message = "Test error", url = "http://test.com" }
      
      init._set_loading_error(test_error)
      
      local err = init.get_loading_error()
      assert.is_table(err)
      assert.equals("Test error", err.message)
    end)
    
    it("detects ready state", function()
      assert.is_false(init._is_ready())
      
      init._set_loading_state("READY")
      
      assert.is_true(init._is_ready())
    end)
    
    it("detects failed state", function()
      assert.is_false(init._is_failed())
      
      init._set_loading_state("FAILED")
      
      assert.is_true(init._is_failed())
    end)
    
    it("detects loading state", function()
      assert.is_false(init._is_loading())
      
      init._set_loading_state("DOWNLOADING")
      assert.is_true(init._is_loading())
      
      init._set_loading_state("LOADING")
      assert.is_true(init._is_loading())
    end)
  end)
  
  describe("auto-download check", function()
    it("returns true when auto-download is enabled", function()
      stub(require("hermes.config"), "get_download").returns({ auto = true })
      
      assert.is_true(init._should_auto_download())
    end)
    
    it("returns false when auto-download is disabled", function()
      stub(require("hermes.config"), "get_download").returns({ auto = false })
      
      assert.is_false(init._should_auto_download())
    end)
    
    it("defaults to true when config is nil", function()
      stub(require("hermes.config"), "get_download").returns(nil)
      
      assert.is_true(init._should_auto_download())
    end)
  end)
end)
