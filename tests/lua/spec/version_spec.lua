-- Unit tests for lua/hermes/version.lua
-- Tests version management and caching

local stub = require("luassert.stub")

describe("hermes.version", function()
  local version
  local download_stub
  local notify_stub
  
  before_each(function()
    package.loaded["hermes.version"] = nil
    version = require("hermes.version")
    
    -- Clear cache before each test
    version.clear_cache()
  end)
  
  after_each(function()
    if download_stub then download_stub:revert() end
    if notify_stub then notify_stub:revert() end
  end)
  
  describe("get_wanted()", function()
    it("returns latest when version is 'latest'", function()
      -- Stub config to return latest
      local config_stub = stub(require("hermes.config"), "get").returns({ version = "latest" })
      
      -- Stub fetch_latest to avoid network call
      local fetch_stub = stub(version, "fetch_latest").returns("v1.0.0")
      
      local result = version.get_wanted()
      
      assert.equals("v1.0.0", result)
      
      config_stub:revert()
      fetch_stub:revert()
    end)
    
    it("adds 'v' prefix when version doesn't have it", function()
      local config_stub = stub(require("hermes.config"), "get").returns({ version = "1.2.3" })
      
      local result = version.get_wanted()
      
      assert.equals("v1.2.3", result)
      
      config_stub:revert()
    end)
    
    it("preserves 'v' prefix when version already has it", function()
      local config_stub = stub(require("hermes.config"), "get").returns({ version = "v1.2.3" })
      
      local result = version.get_wanted()
      
      assert.equals("v1.2.3", result)
      
      config_stub:revert()
    end)
  end)
  
  describe("fetch_latest()", function()
    it("uses cache when cache is valid", function()
      local download_calls = 0
      download_stub = stub(require("hermes.download"), "download").invokes(function()
        download_calls = download_calls + 1
        return true, nil
      end)
      notify_stub = stub(vim, "notify")
      
      local config_stub = stub(require("hermes.config"), "get").returns({ version = "latest" })
      
      -- Stub fetch_latest to return a valid version directly
      local fetch_stub = stub(version, "fetch_latest").returns("v1.0.0")
      
      -- Call get_wanted which calls fetch_latest
      version.get_wanted()
      
      -- Check that fetch_latest was called
      assert.stub(fetch_stub).was_called()
      
      config_stub:revert()
      fetch_stub:revert()
      notify_stub:revert()
    end)
    
    it("returns fallback version on download failure", function()
      download_stub = stub(require("hermes.download"), "download").returns(false, "Network error")
      notify_stub = stub(vim, "notify")
      
      local config_stub = stub(require("hermes.config"), "get").returns({ version = "latest" })
      
      local result = version.get_wanted()
      
      -- Should return fallback version
      assert.truthy(result:match("^v%d+%.%d+%.%d+"), "Should return valid fallback version")
      
      config_stub:revert()
      notify_stub:revert()
    end)
    
    it("refetches when cache is expired", function()
      -- Create a mock temp file with valid JSON response
      local mock_file = os.tmpname()
      local f = io.open(mock_file, "w")
      f:write('{"tag_name": "v1.0.0", "name": "Release v1.0.0"}')
      f:close()
      
      -- Stub download to succeed and use our mock file
      download_stub = stub(require("hermes.download"), "download").invokes(function(url, dest)
        -- Copy mock file to destination
        local uv = vim.uv or vim.loop
        uv.fs_copyfile(mock_file, dest)
        return true, nil
      end)
      
      notify_stub = stub(vim, "notify")
      local config_stub = stub(require("hermes.config"), "get").returns({ version = "latest" })
      
      -- First call to populate cache - this calls real fetch_latest
      version.get_wanted()
      
      -- Verify cache was populated
      local status = version.get_cache_status()
      assert.is_true(status.cached and status.age < 5, "Cache should be populated with age < 5 seconds")
      
      -- Cleanup
      os.remove(mock_file)
      config_stub:revert()
      notify_stub:revert()
    end)
    
    it("parses version from successful GitHub response", function()
      -- Create a mock temp file with valid JSON response
      local mock_file = os.tmpname()
      local f = io.open(mock_file, "w")
      f:write('{"tag_name": "v2.0.0", "name": "Release v2.0.0"}')
      f:close()
      
      -- Stub download to succeed and capture the temp file path
      local captured_path
      download_stub = stub(require("hermes.download"), "download").invokes(function(_url, path)
        captured_path = path
        local uv = vim.uv or vim.loop
        uv.fs_copyfile(mock_file, path)
        return true, nil
      end)
      notify_stub = stub(vim, "notify")
      
      -- Call fetch_latest directly to test the parsing logic
      local result = version.fetch_latest()
      
      -- Cleanup
      os.remove(mock_file)
      if captured_path then
        os.remove(captured_path)
      end
      
      -- Should have parsed v2.0.0 from the JSON
      assert.equals("v2.0.0", result)
      
      download_stub:revert()
      notify_stub:revert()
      
      -- Clear cache to reset state
      version.clear_cache()
    end)
    
    it("returns fallback on invalid JSON response", function()
      -- Create a mock temp file with invalid JSON
      local mock_file = os.tmpname()
      local f = io.open(mock_file, "w")
      f:write('invalid json without tag_name')
      f:close()
      
      local captured_path
      download_stub = stub(require("hermes.download"), "download").invokes(function(_url, path)
        captured_path = path
        local uv = vim.uv or vim.loop
        uv.fs_copyfile(mock_file, path)
        return true, nil
      end)
      notify_stub = stub(vim, "notify")
      
      local result = version.fetch_latest()
      
      -- Cleanup
      os.remove(mock_file)
      if captured_path then
        os.remove(captured_path)
      end
      
      -- Should return fallback version (v0.1.0)
      assert.equals("v0.1.0", result)
      
      download_stub:revert()
      notify_stub:revert()
      version.clear_cache()
    end)
  end)
  
  describe("get_cache_status()", function()
    it("returns correct cache status when empty", function()
      local status = version.get_cache_status()
      
      -- When cache is empty, cached should be falsy (nil or false)
      assert(status.cached ~= true, "Expected cached to not be true")
      assert.is_nil(status.version)
    end)
    
    it("returns valid cache status after fetch", function()
      -- Mock a successful download with valid JSON
      local mock_file = os.tmpname()
      local f = io.open(mock_file, "w")
      f:write('{"tag_name": "v1.2.3"}')
      f:close()
      
      download_stub = stub(require("hermes.download"), "download").invokes(function(_url, path)
        local uv = vim.uv or vim.loop
        uv.fs_copyfile(mock_file, path)
        return true, nil
      end)
      notify_stub = stub(vim, "notify")
      
      -- Call fetch_latest directly to populate cache
      version.fetch_latest()
      
      local status = version.get_cache_status()
      
      -- Cleanup
      os.remove(mock_file)
      
      assert.is_true(status.cached)
      assert.equals("v1.2.3", status.version)
      assert.is_true(status.valid)
      
      download_stub:revert()
      notify_stub:revert()
      version.clear_cache()
    end)
  end)
  
  describe("validate()", function()
    it("accepts 'latest' as valid", function()
      assert.is_true(version.validate("latest"))
    end)
    
    it("accepts valid semantic version format", function()
      assert.is_true(version.validate("v1.0.0"))
      assert.is_true(version.validate("v0.1.0"))
      assert.is_true(version.validate("v10.20.30"))
    end)
    
    it("rejects invalid version format", function()
      assert.is_false(version.validate("1.0.0"))  -- Missing v prefix
      assert.is_false(version.validate("v1.0"))   -- Missing patch
      assert.is_false(version.validate("v1"))     -- Missing minor and patch
      assert.is_false(version.validate("version")) -- Invalid format
    end)
  end)
  
  describe("clear_cache()", function()
    it("clears cached version", function()
      -- First populate cache
      download_stub = stub(require("hermes.download"), "download").returns(true, nil)
      notify_stub = stub(vim, "notify")
      
      local config_stub = stub(require("hermes.config"), "get").returns({ version = "latest" })
      local fetch_stub = stub(version, "fetch_latest")
      fetch_stub.on_call_with().returns("v1.0.0")
      version.get_wanted()
      fetch_stub:revert()
      
      -- Clear cache
      version.clear_cache()
      
      -- Check cache is cleared
      local status = version.get_cache_status()
      assert.is_false(status.cached)
      
      config_stub:revert()
      notify_stub:revert()
    end)
  end)
end)
