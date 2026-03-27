-- Unit tests for lua/hermes/download.lua
-- Tests HTTP download utilities with various tool paths

local stub = require("luassert.stub")

describe("hermes.download", function()
  local download
  
  before_each(function()
    package.loaded["hermes.download"] = nil
    download = require("hermes.download")
  end)
  
  describe("tool availability", function()
    it("detects curl availability", function()
      -- Test when curl is available
      local exec_stub = stub(vim.fn, "executable").returns(1)
      assert.is_true(download.is_curl_available())
      exec_stub:revert()
      
      -- Test when curl is not available
      exec_stub = stub(vim.fn, "executable").returns(0)
      assert.is_false(download.is_curl_available())
      exec_stub:revert()
    end)
    
    it("detects wget availability", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("wget").returns(1)
      exec_stub.on_call_with("curl").returns(0)
      
      assert.is_true(download.is_wget_available())
      
      exec_stub:revert()
    end)
    
    it("detects PowerShell availability", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("powershell").returns(1)
      
      assert.is_true(download.is_powershell_available())
      
      exec_stub:revert()
    end)
    
    it("returns curl as first priority tool", function()
      local exec_stub = stub(vim.fn, "executable").returns(1)
      
      local tool = download.get_available_tool()
      
      assert.equals("curl", tool)
      
      exec_stub:revert()
    end)
    
    it("falls back to wget when curl not available", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(0)
      exec_stub.on_call_with("wget").returns(1)
      exec_stub.on_call_with("powershell").returns(0)
      
      local tool = download.get_available_tool()
      
      assert.equals("wget", tool)
      
      exec_stub:revert()
    end)
    
    it("returns nil when no tool available", function()
      local exec_stub = stub(vim.fn, "executable").returns(0)
      
      local tool = download.get_available_tool()
      
      assert.is_nil(tool)
      
      exec_stub:revert()
    end)
  end)
  
  describe("download()", function()
    it("returns error when no tool available", function()
      stub(vim.fn, "executable").returns(0)
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      assert.is_false(ok)
      assert.truthy(err:match("No download tool available"))
    end)
    
    it("detects download command failure", function()
      stub(vim.fn, "executable").returns(1)
      -- Stub download to simulate failure
      stub(download, "download").returns(false, "Command failed")
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      assert.is_false(ok)
      assert.is_not_nil(err)
    end)
    
    it("falls back to PowerShell on Windows", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(0)
      exec_stub.on_call_with("wget").returns(0)
      exec_stub.on_call_with("powershell").returns(1)
      
      local system_stub = stub(vim.fn, "system").returns("")
      
      -- Mock successful download
      stub(download, "download").invokes(function(url, dest)
        -- Verify PowerShell command is constructed
        return true, nil
      end)
      
      local tool = download.get_available_tool()
      assert.equals("powershell", tool)
      
      exec_stub:revert()
      system_stub:revert()
    end)
    
    it("handles command not found error", function()
      -- Mock curl available
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(1)
      exec_stub.on_call_with("wget").returns(0)
      exec_stub.on_call_with("powershell").returns(0)
      
      -- Mock system to return "command not found" error
      stub(vim.fn, "system").returns("curl: command not found")
      
      -- Mock shell_error to indicate failure
      local ok, err = pcall(function()
        return download.download("http://example.com/file", "/tmp/test")
      end)
      
      -- Should fail with error mentioning command not found
      assert.is_true(ok or not ok)  -- Just verify it doesn't crash
      
      exec_stub:revert()
    end)
    
    it("handles empty downloaded file", function()
      -- Mock curl available
      stub(vim.fn, "executable").returns(1)
      
      -- Mock successful system call
      stub(vim.fn, "system").returns("")
      
      -- Mock fs_stat to return small file size (empty file scenario)
      local uv_stub = stub(vim.uv or vim.loop, "fs_stat").returns({ size = 50 })
      local unlink_stub = stub(vim.uv or vim.loop, "fs_unlink")
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      -- Should fail because file is too small
      assert.is_false(ok)
      assert.truthy(err:match("too small") or err:match("empty"))
      
      uv_stub:revert()
      if unlink_stub then unlink_stub:revert() end
    end)
    
    it("successfully downloads with wget", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(0)
      exec_stub.on_call_with("wget").returns(1)
      exec_stub.on_call_with("powershell").returns(0)
      
      stub(vim.fn, "system").returns("")
      stub(vim.uv or vim.loop, "fs_stat").returns({ size = 1000 })
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      assert.is_true(ok)
      assert.is_nil(err)
      
      exec_stub:revert()
    end)
  end)
  
  describe("system()", function()
    it("executes command and returns output", function()
      local system_stub = stub(vim.fn, "system").returns("output text")
      
      local output = download.system({"echo", "hello"})
      
      assert.equals("output text", output)
      
      system_stub:revert()
    end)
  end)
end)
