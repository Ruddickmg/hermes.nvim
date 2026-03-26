-- Unit tests for lua/hermes/config.lua
-- Tests installation-only configuration (version and auto_download_binary)
-- NOTE: Per AGENTS.md, we do NOT test default values (hard-coded constants)

describe("hermes.config", function()
  local config

  before_each(function()
    package.loaded["hermes.config"] = nil
    config = require("hermes.config")
  end)

  describe("setup() stores configuration", function()
    it("stores version setting", function()
      config.setup({ version = "v1.0.0" })
      
      assert.equals("v1.0.0", config.get_version())
    end)

    it("stores auto_download_binary setting", function()
      config.setup({ auto_download_binary = false })
      
      assert.is_false(config.get_auto_download())
    end)
  end)

  describe("get() returns configuration table", function()
    it("returns table with stored values", function()
      config.setup({ version = "test-version", auto_download_binary = false })
      local current = config.get()

      -- Per AGENTS.md, comparing multiple related values in one assertion is OK
      -- This verifies the entire config object as a logical unit including log config
      assert.same({ 
        version = "test-version", 
        auto_download_binary = false,
        log = {
          notification = {
            level = "error"
          }
        }
      }, current)
    end)
  end)

  describe("get_version()", function()
    it("returns version string", function()
      config.setup({ version = "v2.0.0" })
      
      assert.equals("v2.0.0", config.get_version())
    end)
  end)

  describe("get_notification_level()", function()
    it("returns configured level from string", function()
      config.setup({
        log = {
          notification = {
            level = "warn"
          }
        }
      })
      
      assert.equals("warn", config.get_notification_level())
    end)
    
    it("returns configured level from vim.log.levels", function()
      config.setup({
        log = {
          notification = {
            level = vim.log.levels.DEBUG
          }
        }
      })
      
      assert.equals(vim.log.levels.DEBUG, config.get_notification_level())
    end)
  end)
end)
