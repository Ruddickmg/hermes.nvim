-- Unit tests for lua/hermes/config.lua
-- Tests installation-only configuration (version and auto_download_binary)

describe("hermes.config", function()
  local config

  before_each(function()
    package.loaded["hermes.config"] = nil
    config = require("hermes.config")
  end)

  describe("setup()", function()
    it("stores version setting", function()
      config.setup({ version = "v1.0.0" })
      
      assert.equals("v1.0.0", config.get_version())
    end)

    it("stores auto_download_binary setting", function()
      config.setup({ auto_download_binary = false })
      
      assert.is_false(config.get_auto_download())
    end)

    it("defaults version to latest", function()
      config.setup({})
      
      assert.equals("latest", config.get_version())
    end)

    it("defaults auto_download_binary to true", function()
      config.setup({})
      
      assert.is_true(config.get_auto_download())
    end)

    it("accepts empty config", function()
      local ok = pcall(function()
        config.setup({})
      end)
      
      assert.is_true(ok)
    end)

    it("accepts no arguments", function()
      local ok = pcall(function()
        config.setup()
      end)
      
      assert.is_true(ok)
    end)
  end)

  describe("get()", function()
    it("returns current installation config", function()
      config.setup({ version = "test-version", auto_download_binary = false })
      local current = config.get()

      assert.equals("test-version", current.version)
      assert.is_false(current.auto_download_binary)
    end)
  end)

  describe("get_version()", function()
    it("returns version string", function()
      config.setup({ version = "v2.0.0" })
      
      assert.equals("v2.0.0", config.get_version())
    end)
  end)

  describe("get_notification_level()", function()
    it("returns default error level", function()
      config.setup({})
      
      assert.equals("error", config.get_notification_level())
    end)
    
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
