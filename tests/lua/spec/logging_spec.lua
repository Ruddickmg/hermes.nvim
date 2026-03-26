-- Unit tests for lua/hermes/logging.lua
-- Tests vim.notify wrapper with log level filtering

local stub = require("luassert.stub")

describe("hermes.logging", function()
  local logging
  local config
  local notify_stub
  
  before_each(function()
    package.loaded["hermes.logging"] = nil
    package.loaded["hermes.config"] = nil
    
    config = require("hermes.config")
    logging = require("hermes.logging")
    
    -- Stub vim.notify to capture calls
    notify_stub = stub(vim, "notify")
  end)
  
  after_each(function()
    if notify_stub then notify_stub:revert() end
  end)
  
  describe("notify() filtering", function()
    it("shows error messages with default error level", function()
      config.setup({})  -- Default level is "error"
      
      logging.notify("Test error", vim.log.levels.ERROR)
      
      assert.stub(notify_stub).was_called()
    end)
    
    it("filters out info messages with default error level", function()
      config.setup({})  -- Default level is "error"
      
      logging.notify("Test info", vim.log.levels.INFO)
      
      assert.stub(notify_stub).was_not_called()
    end)
    
    it("shows warn messages when level is set to warn", function()
      config.setup({
        log = {
          notification = {
            level = "warn"
          }
        }
      })
      
      logging.notify("Test warn", vim.log.levels.WARN)
      
      assert.stub(notify_stub).was_called()
    end)
    
    it("filters out debug messages when level is set to info", function()
      config.setup({
        log = {
          notification = {
            level = "info"
          }
        }
      })
      
      logging.notify("Test debug", vim.log.levels.DEBUG)
      
      assert.stub(notify_stub).was_not_called()
    end)
    
    it("handles string level names", function()
      config.setup({
        log = {
          notification = {
            level = "debug"
          }
        }
      })
      
      logging.notify("Test debug with string", "debug")
      
      assert.stub(notify_stub).was_called()
    end)
    
    it("defaults to error level when called with nil", function()
      config.setup({})  -- Default level is "error"
      
      logging.notify("Test nil level", nil)
      
      -- Should show because default is ERROR and nil defaults to ERROR
      assert.stub(notify_stub).was_called()
    end)
    
    it("passes through opts to vim.notify", function()
      config.setup({})
      
      logging.notify("Test", vim.log.levels.ERROR, { title = "Test Title" })
      
      assert.stub(notify_stub).was_called_with("Test", vim.log.levels.ERROR, { title = "Test Title" })
    end)
  end)
  
  describe("edge cases", function()
    it("handles case-insensitive level strings", function()
      config.setup({
        log = {
          notification = {
            level = "ERROR"
          }
        }
      })
      
      logging.notify("Test", "error")
      
      assert.stub(notify_stub).was_called()
    end)
    
    it("treats unknown level strings as error", function()
      config.setup({})
      
      logging.notify("Test", "unknown_level")
      
      -- Unknown level defaults to ERROR level (4), which passes "error" filter
      assert.stub(notify_stub).was_called()
    end)
    
    it("shows all messages when level is trace", function()
      config.setup({
        log = {
          notification = {
            level = "trace"
          }
        }
      })
      
      logging.notify("Test trace", vim.log.levels.TRACE)
      
      assert.stub(notify_stub).was_called()
    end)
    
    it("hides all non-off messages when level is off", function()
      config.setup({
        log = {
          notification = {
            level = "off"
          }
        }
      })
      
      logging.notify("Test error", vim.log.levels.ERROR)
      
      assert.stub(notify_stub).was_not_called()
    end)
  end)
end)
