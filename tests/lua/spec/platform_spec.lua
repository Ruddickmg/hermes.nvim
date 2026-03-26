-- Unit tests for lua/hermes/platform.lua
-- Tests platform detection and binary naming

local stub = require("luassert.stub")

describe("hermes.platform", function()
  local platform
  local os_uname_stub

  before_each(function()
    package.loaded["hermes.platform"] = nil
    platform = require("hermes.platform")
  end)

  after_each(function()
    if os_uname_stub then os_uname_stub:revert() end
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
    
    it("detects Windows via sysname pattern", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Windows_NT",
        machine = "x86_64"
      })
      
      local os = platform.get_os()
      
      assert.equals("windows", os)
    end)
    
    it("falls back to vim.fn.has for unknown sysname", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "UnknownOS",
        machine = "x86_64"
      })
      local has_stub = stub(vim.fn, "has")
      has_stub.on_call_with("win32").returns(0)
      has_stub.on_call_with("win64").returns(0)
      has_stub.on_call_with("mac").returns(1)
      has_stub.on_call_with("osx").returns(0)
      has_stub.on_call_with("linux").returns(0)
      
      local os = platform.get_os()
      
      assert.equals("macos", os)
      has_stub:revert()
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
    
    it("normalizes arm64 to aarch64", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Linux",
        machine = "arm64"
      })
      
      package.loaded["hermes.platform"] = nil
      platform = require("hermes.platform")
      
      assert.equals("aarch64", platform.get_arch())
    end)
    
    it("normalizes amd64 to x86_64", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Linux",
        machine = "amd64"
      })
      
      package.loaded["hermes.platform"] = nil
      platform = require("hermes.platform")
      
      assert.equals("x86_64", platform.get_arch())
    end)
    
    it("errors on 32-bit x86 architecture", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Linux",
        machine = "i386"
      })
      
      package.loaded["hermes.platform"] = nil
      platform = require("hermes.platform")
      
      local ok, err = pcall(function() return platform.get_arch() end)
      assert.is_false(ok)
      assert.truthy(err:match("32%-bit"))
    end)
    
    it("errors on unsupported architecture", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Linux",
        machine = "mips"
      })
      
      package.loaded["hermes.platform"] = nil
      platform = require("hermes.platform")
      
      local ok, err = pcall(function() return platform.get_arch() end)
      assert.is_false(ok)
      assert.truthy(err:match("Unsupported architecture"))
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
    
    it("returns unknown platform when detection fails", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Unknown",
        machine = "unknown"
      })
      
      package.loaded["hermes.platform"] = nil
      platform = require("hermes.platform")
      
      local display = platform.get_display_string()
      assert.equals("Unknown Platform", display)
    end)
  end)

  describe("get_platform_key()", function()
    it("returns os-arch format", function()
      local key = platform.get_platform_key()
      local os = platform.get_os()
      local arch = platform.get_arch()

      assert.equals(os .. "-" .. arch, key)
    end)
    
    it("returns nil when platform detection fails", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Unknown",
        machine = "unknown"
      })
      
      package.loaded["hermes.platform"] = nil
      platform = require("hermes.platform")
      
      local key = platform.get_platform_key()
      assert.is_nil(key)
    end)
  end)

  describe("is_supported()", function()
    it("returns true for supported platforms", function()
      local supported, err = platform.is_supported()
      assert.is_true(supported)
      assert.is_nil(err)
    end)
    
    it("returns false and error for unsupported platforms", function()
      os_uname_stub = stub(vim.loop, "os_uname").returns({
        sysname = "Unknown",
        machine = "unknown"
      })
      
      package.loaded["hermes.platform"] = nil
      platform = require("hermes.platform")
      
      local supported, err = platform.is_supported()
      assert.is_false(supported)
      assert.is_not_nil(err)
    end)
  end)

  describe("SUPPORTED_PLATFORMS", function()
    it("contains linux-x86_64 via binary module", function()
      local binary = require("hermes.binary")
      assert.is_true(binary.SUPPORTED_PLATFORMS["linux-x86_64"])
    end)

    it("contains linux-aarch64 via binary module", function()
      local binary = require("hermes.binary")
      assert.is_true(binary.SUPPORTED_PLATFORMS["linux-aarch64"])
    end)

    it("contains macos-x86_64 via binary module", function()
      local binary = require("hermes.binary")
      assert.is_true(binary.SUPPORTED_PLATFORMS["macos-x86_64"])
    end)

    it("contains macos-aarch64 via binary module", function()
      local binary = require("hermes.binary")
      assert.is_true(binary.SUPPORTED_PLATFORMS["macos-aarch64"])
    end)

    it("contains windows-x86_64 via binary module", function()
      local binary = require("hermes.binary")
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
