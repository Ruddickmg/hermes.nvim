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

  describe("setup() with nix install detected", function()
    local stub = require("luassert.stub")

    it("defaults download.auto to false when installed via nix", function()
      local binary = require("hermes.binary")
      local nix_stub = stub(binary, "is_nix_install").returns(true)

      config.setup({})

      nix_stub:revert()
      assert.is_false(config.get_auto_download())
    end)

    it("keeps explicit auto=true override on nix installs", function()
      local binary = require("hermes.binary")
      local nix_stub = stub(binary, "is_nix_install").returns(true)

      config.setup({ download = { auto = true } })

      nix_stub:revert()
      assert.is_true(config.get_auto_download())
    end)

    it("keeps download.auto true when not installed via nix", function()
      local binary = require("hermes.binary")
      local nix_stub = stub(binary, "is_nix_install").returns(false)

      config.setup({})

      nix_stub:revert()
      assert.is_true(config.get_auto_download())
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

      assert.same({ 
        version = "test-version", 
        auto = false,
        timeout = 90
      }, current.download)
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

  describe("get_progress()", function()
    it("returns configured cmdline setting", function()
      config.setup({ progress = { cmdline = true } })

      local progress_cfg = config.get_progress()
      assert.is_true(progress_cfg.cmdline)
    end)
  end)

  describe("agents configuration", function()
    it("stores user-defined agent connections", function()
      config.setup({
        agents = {
          opencode = { command = "opencode", args = { "acp" } },
        },
      })

      assert.equals("opencode", config.get().agents.opencode.command)
    end)

    it("defaults agents to an empty table", function()
      config.setup({})

      assert.same({}, config.get().agents)
    end)
  end)
end)
