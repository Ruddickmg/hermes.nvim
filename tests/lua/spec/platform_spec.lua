-- Unit tests for lua/hermes/platform.lua
-- Tests platform detection and binary naming

describe("hermes.platform", function()
  local platform

  before_each(function()
    package.loaded["hermes.platform"] = nil
    platform = require("hermes.platform")
  end)

  describe("get_os()", function()
    it("returns valid OS string", function()
      local os = platform.get_os()
      -- Single assertion comparing to expected set of values
      assert.is_true(
        os == "linux" or os == "macos" or os == "windows",
        "Expected linux, macos, or windows but got: " .. tostring(os)
      )
    end)

    it("returns lowercase", function()
      local os = platform.get_os()
      assert.equals(os, os:lower())
    end)
  end)

  describe("get_arch()", function()
    it("returns valid architecture", function()
      local arch = platform.get_arch()
      assert.is_true(
        arch == "x86_64" or arch == "aarch64",
        "Expected x86_64 or aarch64 but got: " .. tostring(arch)
      )
    end)

    it("returns consistent value", function()
      local arch1 = platform.get_arch()
      local arch2 = platform.get_arch()
      assert.equals(arch1, arch2)
    end)
  end)

  describe("get_ext()", function()
    it("returns correct extension", function()
      local ext = platform.get_ext()
      local os = platform.get_os()

      if os == "linux" then
        assert.equals("so", ext)
      elseif os == "macos" then
        assert.equals("dylib", ext)
      elseif os == "windows" then
        assert.equals("dll", ext)
      end
    end)
  end)

  describe("get_binary_name()", function()
    it("generates correct name format", function()
      local name = platform.get_binary_name()
      local os = platform.get_os()
      local arch = platform.get_arch()
      local ext = platform.get_ext()

      assert.equals("libhermes-" .. os .. "-" .. arch .. "." .. ext, name)
    end)
  end)

  describe("get_display_string()", function()
    it("returns human-readable format", function()
      local display = platform.get_display_string()
      local os = platform.get_os()
      local arch = platform.get_arch()

      -- Verify contains both OS (capitalized) and arch
      assert.matches(os:gsub("^%l", string.upper), display)
      assert.matches(arch, display)
    end)
  end)

  describe("get_platform_key()", function()
    it("returns os-arch format", function()
      local key = platform.get_platform_key()
      local os = platform.get_os()
      local arch = platform.get_arch()

      assert.equals(os .. "-" .. arch, key)
    end)
  end)

  describe("SUPPORTED_PLATFORMS", function()
    it("contains expected platforms via binary module", function()
      local binary = require("hermes.binary")
      -- Check each platform individually
      assert.is_true(binary.SUPPORTED_PLATFORMS["linux-x86_64"])
      assert.is_true(binary.SUPPORTED_PLATFORMS["linux-aarch64"])
      assert.is_true(binary.SUPPORTED_PLATFORMS["macos-x86_64"])
      assert.is_true(binary.SUPPORTED_PLATFORMS["macos-aarch64"])
      assert.is_true(binary.SUPPORTED_PLATFORMS["windows-x86_64"])
    end)

    it("supports current platform", function()
      local binary = require("hermes.binary")
      local key = platform.get_platform_key()
      assert.is_true(
        binary.SUPPORTED_PLATFORMS[key],
        "Current platform " .. tostring(key) .. " should be supported"
      )
    end)
  end)
end)
