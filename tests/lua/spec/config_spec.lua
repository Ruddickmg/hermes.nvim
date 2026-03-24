-- Unit tests for lua/hermes/config.lua
-- Tests configuration handling and validation

describe("hermes.config", function()
  local config

  before_each(function()
    package.loaded["hermes.config"] = nil
    config = require("hermes.config")
  end)

  describe("validate()", function()
    it("accepts valid config with all fields", function()
      local valid = {
        version = "latest",
        auto_download_binary = true,
        permissions = { fs_write_access = true }
      }

      local ok, err = config.validate(valid)
      assert.is_true(ok, "Validation should pass: " .. tostring(err))
    end)

    it("rejects non-string version", function()
      local invalid = { version = 123 }

      local ok, err = config.validate(invalid)
      assert.is_false(ok)
      assert.matches("version must be a string", err)
    end)

    it("rejects non-boolean auto_download_binary", function()
      local invalid = { auto_download_binary = "yes" }

      local ok, err = config.validate(invalid)
      assert.is_false(ok)
      assert.matches("auto_download_binary must be a boolean", err)
    end)

    it("accepts empty config", function()
      local ok, err = config.validate({})
      assert.is_true(ok)
      assert.is_nil(err)
    end)
  end)

  describe("setup()", function()
    it("merges user config with defaults", function()
      config.setup({ auto_download_binary = false })
      local current = config.get()

      assert.is_false(current.auto_download_binary)
    end)

    it("preserves defaults for unset fields", function()
      config.setup({ auto_download_binary = false })
      local current = config.get()

      assert.equals("latest", current.version)
    end)

    it("allows partial nested config", function()
      config.setup({
        permissions = { fs_write_access = false }
      })
      local current = config.get()

      assert.is_false(current.permissions.fs_write_access)
      assert.is_true(current.permissions.fs_read_access)
    end)

    it("merges across multiple calls", function()
      config.setup({ version = "v1.0.0" })
      config.setup({ auto_download_binary = false })

      local current = config.get()
      assert.equals("v1.0.0", current.version)
    end)
  end)

  describe("get()", function()
    it("returns copy of current config", function()
      config.setup({ version = "test-version" })
      local current = config.get()

      assert.equals("test-version", current.version)
    end)

    it("returns new instance after setup", function()
      local before = config.get()
      config.setup({ version = "changed" })
      local after = config.get()

			assert.are_not.same(before, after)
			assert.equals("changed", after.version)
		end)
	end)
end)
