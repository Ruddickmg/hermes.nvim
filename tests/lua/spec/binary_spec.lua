-- Unit tests for lua/hermes/binary.lua
-- Tests binary management with mocked HTTP downloads using download module

local helpers = require("helpers")
local stub = require("luassert.stub")

describe("hermes.binary", function()
  local binary
  local download
  local temp_dir
  local stdpath_stub
  local filereadable_stub
  local download_stub
  local version_stub

  before_each(function()
    temp_dir = helpers.create_temp_dir()
    stdpath_stub = stub(vim.fn, "stdpath").returns(temp_dir)

    package.loaded["hermes.binary"] = nil
    package.loaded["hermes.download"] = nil
    package.loaded["hermes.platform"] = nil
    package.loaded["hermes.config"] = nil
    package.loaded["hermes.version"] = nil

    binary = require("hermes.binary")
    download = require("hermes.download")
  end)

  after_each(function()
    helpers.cleanup_temp_dir(temp_dir)
    if stdpath_stub then stdpath_stub:revert() end
    if filereadable_stub then filereadable_stub:revert() end
    if download_stub then download_stub:revert() end
    if version_stub then version_stub:revert() end
  end)

  describe("get_data_dir()", function()
    it("returns path ending with hermes", function()
      local dir = binary.get_data_dir()
      assert.matches("hermes$", dir)
    end)

    it("returns consistent path", function()
      local dir1 = binary.get_data_dir()
      local dir2 = binary.get_data_dir()
      assert.equals(dir1, dir2)
    end)
  end)

  describe("get_version_file()", function()
    it("returns path in data directory", function()
      local ver_file = binary.get_version_file()
      local data_dir = binary.get_data_dir()

      assert.is_true(ver_file:find(data_dir) == 1, "Version file should be in data directory")
    end)
  end)

  describe("get_binary_path()", function()
    it("includes platform-specific name", function()
      local bin_path = binary.get_binary_path()
      local platform = require("hermes.platform")
      local expected_name = platform.get_binary_name()

      assert.truthy(bin_path:find(expected_name, 1, true), "Binary path should contain: " .. expected_name)
    end)
  end)

  describe("download()", function()
    it("downloads to correct path", function()
      local captured_dest
      download_stub = stub(download, "download").invokes(function(_, dest)
        captured_dest = dest
        return true, nil
      end)

      local target_path = temp_dir .. "/libhermes-linux-x86_64.so"
      binary.download(target_path, "v1.0.0")

      assert.equals(target_path, captured_dest)
    end)

    it("returns true on success", function()
      download_stub = stub(download, "download").returns(true, nil)

      local result = binary.download(temp_dir .. "/test.so", "v1.0.0")

      assert.is_true(result)
    end)

    it("returns false on failure", function()
      download_stub = stub(download, "download").returns(false, "Network error")

      local result = binary.download(temp_dir .. "/test.so", "v1.0.0")

      assert.is_false(result)
    end)
  end)
  describe("ensure_binary()", function()
    it("downloads when binary missing", function()
      filereadable_stub = stub(vim.fn, "filereadable").returns(0)
      download_stub = stub(download, "download").returns(true, nil)
      version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")

      binary.ensure_binary()

      assert.stub(download_stub).was_called()
    end)

    it("skips download when binary and version match", function()
      -- Create existing files
      local bin_path = binary.get_binary_path()
      local version_file = binary.get_version_file()
      vim.fn.mkdir(binary.get_data_dir(), "p")
      io.open(bin_path, "w"):close()
      local f = io.open(version_file, "w")
      f:write("v1.0.0")
      f:close()

      filereadable_stub = stub(vim.fn, "filereadable").returns(1)
      download_stub = stub(download, "download")
      version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")

      binary.ensure_binary()

      assert.stub(download_stub).was_not_called()
    end)

    it("re-downloads when version differs", function()
      -- Create existing files with old version
      local bin_path = binary.get_binary_path()
      local version_file = binary.get_version_file()
      vim.fn.mkdir(binary.get_data_dir(), "p")
      io.open(bin_path, "w"):close()
      local f = io.open(version_file, "w")
      f:write("v0.9.0")
      f:close()

      filereadable_stub = stub(vim.fn, "filereadable").returns(1)
      download_stub = stub(download, "download").returns(true, nil)
      version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")

      binary.ensure_binary()

      assert.stub(download_stub).was_called()
    end)
  end)

  describe("build_from_source()", function()
    it("builds successfully when cargo available", function()
      local filereadable_stub_local = stub(vim.fn, "filereadable").invokes(function(path)
        if path:match("target/release/libhermes") then return 1 end
        return 0
      end)

      local system_stub_local = stub(vim.fn, "system").returns("")
      stub(vim.fn, "executable").returns(1)

      local ok = binary.build_from_source(temp_dir)

      assert.is_true(ok)

      filereadable_stub_local:revert()
      system_stub_local:revert()
    end)
  end)

  describe("load_existing_binary()", function()
    it("returns path when binary exists", function()
      local bin_path = binary.get_binary_path()
      vim.fn.mkdir(binary.get_data_dir(), "p")
      io.open(bin_path, "w"):close()

      filereadable_stub = stub(vim.fn, "filereadable").returns(1)

      local result = binary.load_existing_binary()
      assert.equals(bin_path, result)
    end)

    it("errors when no download tools available", function()
      -- Mock no download tools available
      stub(download, "is_curl_available").returns(false)
      stub(download, "is_wget_available").returns(false)
      stub(download, "get_available_tool").returns(nil)
      
      local ok, err = pcall(function()
        binary.ensure_binary()
      end)
      
      assert.is_false(ok)
      assert.is_truthy(err:match("curl") or err:match("wget"))
    end)
  end)
end)
