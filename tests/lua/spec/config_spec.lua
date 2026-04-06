-- Unit tests for lua/hermes/config.lua
-- Tests installation-only configuration (download table with version, auto, timeout)
-- NOTE: Per AGENTS.md, we do NOT test default values (hard-coded constants)

describe("hermes.config", function()
  local config

  before_each(function()
    package.loaded["hermes.config"] = nil
    config = require("hermes.config")
  end)

  describe("setup() stores configuration", function()
    it("stores download.version setting", function()
      config.setup({ download = { version = "v1.0.0" } })
      
      assert.equals("v1.0.0", config.get_version())
    end)

    it("stores download.auto setting", function()
      config.setup({ download = { auto = false } })
      
      assert.is_false(config.get_auto_download())
    end)

    it("stores download.timeout setting", function()
      config.setup({ download = { timeout = 120 } })
      
      assert.equals(120, config.get_download_timeout())
    end)
  end)

  describe("get() returns configuration table", function()
    it("returns table with stored values", function()
      config.setup({ 
        download = { 
          version = "test-version", 
          auto = false,
          timeout = 90
        }
      })
      local current = config.get()

      -- Per AGENTS.md, comparing multiple related values in one assertion is OK
      -- This verifies the entire config object as a logical unit including log config
      assert.same({ 
        download = {
          version = "test-version", 
          auto = false,
          timeout = 90
        },
        log = {
          notification = {
            level = "info"  -- Changed from "error" to "info" for better UX
          }
        }
      }, current)
    end)
  end)

  describe("get_version()", function()
    it("returns version string", function()
      config.setup({ download = { version = "v2.0.0" } })
      
      assert.equals("v2.0.0", config.get_version())
    end)
  end)

  describe("get_download()", function()
    it("returns download configuration table", function()
      config.setup({ 
        download = {
          version = "v3.0.0",
          auto = true,
          timeout = 45
        }
      })
      
      local download_cfg = config.get_download()
      assert.same({
        version = "v3.0.0",
        auto = true,
        timeout = 45
      }, download_cfg)
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
