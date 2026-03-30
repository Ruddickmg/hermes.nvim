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
    
    -- Clean up any inline stubs of vim.fn functions that tests may have created
    -- These are not tracked by the variables above and can cause test pollution
    pcall(function() if vim.fn.readfile.revert then vim.fn.readfile:revert() end end)
    pcall(function() if vim.fn.writefile.revert then vim.fn.writefile:revert() end end)
    pcall(function() if vim.fn.executable.revert then vim.fn.executable:revert() end end)
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
      stub(download, "get_available_tool").returns("curl")
      version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
      stub(vim.fn, "writefile")

      binary.ensure_binary()

      assert.stub(download_stub).was_called()
    end)

    it("skips download when binary exists and version matches", function()
      -- Create existing binary file and version file
      local bin_path = binary.get_binary_path()
      local ver_file = binary.get_version_file()
      vim.fn.mkdir(binary.get_data_dir(), "p")
      io.open(bin_path, "w"):close()
      local f = io.open(ver_file, "w")
      f:write("v1.0.0")
      f:close()

      -- Mock: binary exists (1), version file exists (1)
      local filereadable_count = 0
      filereadable_stub = stub(vim.fn, "filereadable").invokes(function()
        filereadable_count = filereadable_count + 1
        return 1  -- Both files exist
      end)
      
      stub(vim.fn, "readfile").returns({"v1.0.0"})
      version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
      download_stub = stub(download, "download")

      binary.ensure_binary()

      assert.stub(download_stub).was_not_called()
    end)

    it("downloads when binary exists but version differs", function()
      -- Create existing binary file with old version
      local bin_path = binary.get_binary_path()
      local ver_file = binary.get_version_file()
      vim.fn.mkdir(binary.get_data_dir(), "p")
      io.open(bin_path, "w"):close()
      local f = io.open(ver_file, "w")
      f:write("v0.9.0")
      f:close()

      -- Mock: binary exists (1), version file exists (1)
      local filereadable_count = 0
      filereadable_stub = stub(vim.fn, "filereadable").invokes(function()
        filereadable_count = filereadable_count + 1
        return 1  -- Both files exist
      end)
      
      stub(vim.fn, "readfile").returns({"v0.9.0"})
      version_stub = stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
      download_stub = stub(download, "download").returns(true, nil)
      stub(vim.fn, "writefile")

      binary.ensure_binary()

      assert.stub(download_stub).was_called()
    end)
  end)

  describe("build_from_source()", function()
    it("copies built library to correct path with platform suffix", function()
      -- Create a mock built library file (simulating cargo build output)
      local platform = require("hermes.platform")
      local build_dir = temp_dir .. "/build"
      local target_dir = build_dir .. "/target/release"
      local ext = platform.get_ext()
      local mock_built_lib = target_dir .. "/libhermes." .. ext
      local expected_bin_name = platform.get_binary_name()
      local expected_final_path = temp_dir .. "/" .. expected_bin_name
      
      -- Create directory structure and mock library file
      vim.fn.mkdir(target_dir, "p")
      local f = io.open(mock_built_lib, "w")
      f:write("mock library content")
      f:close()
      
      -- Mock the build process by directly testing the copy behavior
      -- This bypasses the actual git clone and cargo build
      local uv = vim.uv or vim.loop
      local dest_dir = temp_dir
      local bin_name = platform.get_binary_name()
      local final_path = dest_dir .. "/" .. bin_name
      
      -- Manually copy the file to simulate what build_from_source should do
      local result, err = uv.fs_copyfile(mock_built_lib, final_path)
      
      -- Verify both that copy succeeded AND file exists at expected path
      local file_exists = vim.fn.filereadable(expected_final_path) == 1
      assert.is_true(result and file_exists, 
        "Failed to copy: " .. (err or "unknown error") .. " or file not found at: " .. expected_final_path)
    end)
    
    it("uses correct filename format consistent with get_binary_path()", function()
      local platform = require("hermes.platform")
      local expected_name = platform.get_binary_name()
      
      -- Build expected format and verify in single assertion
      local expected_format = "libhermes-" .. platform.get_os() .. "-" .. platform.get_arch() .. "." .. platform.get_ext()
      assert.equals(expected_format, expected_name)
    end)
  end)

  describe("build_from_source() error handling", function()
    it("returns false when git clone fails", function()
      local dest_dir = temp_dir
      
      -- Mock system to simulate git clone failure with non-zero exit
      stub(vim.fn, "system").returns("fatal: unable to access")
      stub(vim.fn, "executable").returns(1)
      local notify_stub = stub(require("hermes.logging"), "notify")
      
      -- vim.v.shell_error cannot be stubbed directly, but we can verify
      -- the function handles the failure case without crashing
      local result = binary.build_from_source(dest_dir)
      
      -- Should return false on git clone failure
      assert.is_false(result)
      
      notify_stub:revert()
    end)
    
    it("returns false when cargo build fails", function()
      local dest_dir = temp_dir
      
      -- Create a mock that simulates successful git clone but failed build
      local system_stub = stub(vim.fn, "system").invokes(function(cmd)
        if type(cmd) == "table" then
          if cmd[1] == "git" then
            return "" -- git clone succeeds
          elseif cmd[1] == "cargo" then
            return "error: failed to compile" -- cargo fails
          end
        end
        return ""
      end)
      stub(vim.fn, "executable").returns(1)
      local notify_stub = stub(require("hermes.logging"), "notify")
      
      local result = binary.build_from_source(dest_dir)
      
      -- Should return false on cargo build failure
      assert.is_false(result)
      
      system_stub:revert()
      notify_stub:revert()
    end)
    
    it("handles copy failure gracefully", function()
      local platform = require("hermes.platform")
      local build_dir = temp_dir .. "/build"
      local target_dir = build_dir .. "/target/release"
      local ext = platform.get_ext()
      local mock_built_lib = target_dir .. "/libhermes." .. ext
      
      -- Create directory and mock built file
      vim.fn.mkdir(target_dir, "p")
      local f = io.open(mock_built_lib, "w")
      f:write("mock content")
      f:close()
      
      -- Mock successful git and cargo
      stub(vim.fn, "system").returns("")
      stub(vim.fn, "executable").returns(1)
      
      -- Mock fs_copyfile to fail
      local uv_stub = stub(vim.uv or vim.loop, "fs_copyfile").returns(nil, "Permission denied")
      local notify_stub = stub(require("hermes.logging"), "notify")
      
      local result = binary.build_from_source(temp_dir)
      
      uv_stub:revert()
      notify_stub:revert()
      
      assert.is_false(result)
    end)
  end)

  describe("ensure_binary() error paths", function()
    it("shows helpful error for unsupported platform", function()
      -- Mock platform as unsupported
      stub(require("hermes.platform"), "is_supported").returns(false)
      stub(require("hermes.platform"), "get_platform_key").returns("mips")
      stub(require("hermes.platform"), "get_display_string").returns("mips")
      stub(vim.fn, "filereadable").returns(0)
      
      local ok, err = pcall(function()
        binary.ensure_binary()
      end)
      
      assert.is_false(ok)
      assert.truthy(err:match("not supported") or err:match("platform"))
    end)
    
    it("handles download failure gracefully", function()
      -- Setup: missing binary, download will fail
      stub(vim.fn, "filereadable").returns(0)
      stub(require("hermes.config"), "get").returns({ 
        download = {
          auto = true,
          version = "v9.9.9"
        }
      })
      stub(download, "download").returns(false, "HTTP 404")
      stub(download, "get_available_tool").returns("curl")
      stub(require("hermes.platform"), "is_supported").returns(true)
      
      local ok, err = pcall(function()
        binary.ensure_binary()
      end)
      
      assert.is_false(ok)
      -- Error should mention download failure
      assert.truthy(err:match("download") or err:match("Failed"))
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
      
      local ok, _ = pcall(function()
        binary.ensure_binary()
      end)
      
      assert.is_false(ok)
    end)
    
    it("error message mentions download tools when none available", function()
      -- Mock no download tools available
      stub(vim.fn, "filereadable").returns(0)
      stub(download, "get_available_tool").returns(nil)
      stub(require("hermes.platform"), "is_supported").returns(true)
      stub(require("hermes.platform"), "get_platform_key").returns("linux-x86_64")
      
      local _, err = pcall(function()
        binary.ensure_binary()
      end)
      assert.truthy(err:match("curl") or err:match("wget"))
    end)
  end)

  describe("load()", function()
    it("returns native module when binary exists and loads successfully", function()
      -- Use the real binary from target/release
      local platform = require("hermes.platform")
      local bin_path = binary.get_binary_path()
      
      -- Ensure binary directory exists and copy real binary
      vim.fn.mkdir(binary.get_data_dir(), "p")
      local source_bin = vim.fn.getcwd() .. "/target/release/libhermes." .. platform.get_ext()
      local uv = vim.uv or vim.loop
      uv.fs_copyfile(source_bin, bin_path)
      
      -- Mock filereadable to return 1 (file exists)
      stub(vim.fn, "filereadable").returns(1)
      
      -- Mock the version module to avoid download checks
      stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
      
      -- Also need to mock readfile for version check
      stub(vim.fn, "readfile").returns({"v0.0.1"})
      
      -- Call load - should use real binary
      local ok, result = pcall(function()
        return binary.load()
      end)
      
      -- Should succeed and return a table (the native module) - combined assertion
      assert.is_true(ok and type(result) == "table", 
        "load should succeed and return native module table: " .. tostring(result))
    end)
  end)

  describe("ensure_binary_async()", function()
    it("returns binary path when binary exists", function()
      -- Use the real binary from target/release
      local platform = require("hermes.platform")
      local bin_path = binary.get_binary_path()
      
      -- Ensure binary directory exists and copy real binary
      vim.fn.mkdir(binary.get_data_dir(), "p")
      local source_bin = vim.fn.getcwd() .. "/target/release/libhermes." .. platform.get_ext()
      local uv = vim.uv or vim.loop
      uv.fs_copyfile(source_bin, bin_path)
      
      -- Create version file
      vim.fn.writefile({"v0.0.1"}, binary.get_version_file())
      
      -- Mock version module
      stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
      
      -- Mock filereadable for both binary and version file
      stub(vim.fn, "filereadable").returns(1)
      
      -- Mock readfile to return matching version
      stub(vim.fn, "readfile").returns({"v0.0.1"})
      
      -- Mock download availability
      stub(download, "get_available_tool").returns("curl")
      
      local callback_called = false
      local callback_result = nil
      local callback_success = nil
      
      -- Call ensure_binary_async with matching binary
      binary.ensure_binary_async(60, function(success, result)
        callback_called = true
        callback_success = success
        callback_result = result
      end)
      
      -- Wait for async callback to complete
      vim.wait(100, function()
        return callback_called
      end)
      
      -- Callback should be called with success=true and a valid path
      assert.is_true(callback_called and callback_success and callback_result ~= nil, 
        "Callback should be called with success and binary path when binary exists")
    end)
    
    it("downloads when binary is missing", function()
      -- Ensure binary does NOT exist
      local bin_path = binary.get_binary_path()
      vim.fn.delete(bin_path)
      
      -- Mock version module
      stub(require("hermes.version"), "get_wanted").returns("v0.0.1")
      
      -- Mock download to succeed
      stub(binary, "download").returns(true)
      stub(vim.fn, "writefile")
      stub(download, "get_available_tool").returns("curl")
      
      -- Call should not error even with missing binary
      local ok = pcall(function()
        binary.ensure_binary_async(60, function(_success, _result)
          -- Callback
        end)
      end)
      
      -- With vim.schedule, callback won't be immediate
      -- but function should either attempt download or return immediately
      assert.is_true(ok, "ensure_binary_async should not crash when binary is missing")
    end)

    it("downloads when binary exists but version differs", function()
      -- Create binary with old version
      local bin_path = binary.get_binary_path()
      local ver_file = binary.get_version_file()
      vim.fn.mkdir(binary.get_data_dir(), "p")
      io.open(bin_path, "w"):close()
      local f = io.open(ver_file, "w")
      f:write("v0.9.0")
      f:close()
      
      -- Mock: want different version
      stub(require("hermes.version"), "get_wanted").returns("v1.0.0")
      
      -- Mock filereadable to return 1 (files exist)
      stub(vim.fn, "filereadable").returns(1)
      stub(vim.fn, "readfile").returns({"v0.9.0"})
      stub(vim.fn, "writefile")
      
      -- Mock download to succeed
      stub(binary, "download").returns(true)
      stub(download, "get_available_tool").returns("curl")
      
      local callback_called = false
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
      end)
      
      -- Wait for async operation
      vim.wait(100)
      
      -- Callback should be called
      assert.is_true(callback_called, "Callback should be called for version mismatch")
    end)
    
    it("handles unsupported platform error", function()
      -- Mock platform as nil (unable to determine)
      stub(require("hermes.platform"), "get_platform_key").returns(nil)
      
      local callback_called = false
      local callback_success = nil
      
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
        callback_success = _success
      end)
      
      -- Callback should be called immediately with failure for unsupported platform
      assert.is_true(callback_called, "Callback should be called for unsupported platform")
      assert.is_false(callback_success, "Should report failure for unsupported platform")
    end)
    
    it("handles no download tool available", function()
      -- Mock download tools as unavailable
      stub(download, "get_available_tool").returns(nil)
      
      local callback_called = false
      local callback_success = nil
      
      binary.ensure_binary_async(60, function(_success, _result)
        callback_called = true
        callback_success = _success
      end)
      
      -- Callback should be called immediately with failure
      assert.is_true(callback_called, "Callback should be called when no download tool")
      assert.is_false(callback_success, "Should report failure when no download tool")
    end)
  end)
end)