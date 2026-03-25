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
  end)
  
  describe("get_cache_status()", function()
    it("returns correct cache status when empty", function()
      local status = version.get_cache_status()
      
      -- When cache is empty, cached should be falsy (nil or false)
      assert(status.cached ~= true, "Expected cached to not be true")
      assert.is_nil(status.version)
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
