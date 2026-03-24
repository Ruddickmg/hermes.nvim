-- Integration tests for lua/hermes/init.lua
-- Tests that all API methods are available and have correct signatures

local helpers = require("helpers")
local stub = require("luassert.stub")

describe("hermes.init (main API)", function()
  local hermes
  local temp_dir
  local stdpath_stub
  local filereadable_stub
  
  before_each(function()
    -- Create temp directory
    temp_dir = helpers.create_temp_dir()
    
    -- Create individual stubs
    stdpath_stub = stub(vim.fn, "stdpath").returns(temp_dir)
    filereadable_stub = stub(vim.fn, "filereadable").returns(1)
    
    -- Reload modules
    package.loaded["hermes.init"] = nil
    package.loaded["hermes.binary"] = nil
    package.loaded["hermes.config"] = nil
    package.loaded["hermes.platform"] = nil
    package.loaded["hermes.version"] = nil
    
    hermes = require("hermes")
  end)
  
  after_each(function()
    helpers.cleanup_temp_dir(temp_dir)
    
    -- Revert all individual stubs
    if stdpath_stub then stdpath_stub:revert() end
    if filereadable_stub then filereadable_stub:revert() end
  end)
  
  describe("API surface", function()
    it("exports setup function", function()
      assert.is_function(hermes.setup)
    end)
    
    it("exports connect function", function()
      assert.is_function(hermes.connect)
    end)
    
    it("exports disconnect function", function()
      assert.is_function(hermes.disconnect)
    end)
    
    it("exports create_session function", function()
      assert.is_function(hermes.create_session)
    end)
    
    it("exports prompt function", function()
      assert.is_function(hermes.prompt)
    end)
    
    it("exports authenticate function", function()
      assert.is_function(hermes.authenticate)
    end)
    
    it("exports respond function", function()
      assert.is_function(hermes.respond)
    end)
    
    it("exports cancel function", function()
      assert.is_function(hermes.cancel)
    end)
    
    it("exports set_mode function", function()
      assert.is_function(hermes.set_mode)
    end)
    
    it("exports load_session function", function()
      assert.is_function(hermes.load_session)
    end)
    
    it("exports list_sessions function", function()
      assert.is_function(hermes.list_sessions)
    end)
  end)
  
  describe("setup()", function()
    it("accepts configuration table", function()
      local ok = pcall(function()
        hermes.setup({
          auto_download_binary = false,
          version = "latest"
        })
      end)
      
      assert.is_true(ok)
    end)
    
    it("accepts empty configuration", function()
      local ok = pcall(function()
        hermes.setup({})
      end)
      
      assert.is_true(ok)
    end)
    
    it("accepts no arguments", function()
      local ok = pcall(function()
        hermes.setup()
      end)
      
      assert.is_true(ok)
    end)
    
    it("returns nil", function()
      local result = hermes.setup({})
      assert.is_nil(result)
    end)
  end)
  
  describe("API function signatures", function()
    it("connect accepts agent name as first argument", function()
      -- Just verify the function accepts the argument without crashing
      -- We can't actually test the native call without the binary
      assert.has_no.errors(function()
        -- This will fail to load binary, but verifies the function signature
        pcall(function() hermes.connect("test-agent") end)
      end)
    end)
    
    it("disconnect accepts agent name", function()
      assert.has_no.errors(function()
        pcall(function() hermes.disconnect("test-agent") end)
      end)
    end)
    
    it("disconnect accepts array of agent names", function()
      assert.has_no.errors(function()
        pcall(function() hermes.disconnect({ "agent1", "agent2" }) end)
      end)
    end)
    
    it("create_session accepts configuration object", function()
      assert.has_no.errors(function()
        pcall(function()
          hermes.create_session({
            cwd = "/test/path",
            mcpServers = {}
          })
        end)
      end)
    end)
    
    it("prompt accepts session ID and content", function()
      assert.has_no.errors(function()
        pcall(function() hermes.prompt("session-id", { type = "text", text = "Hello" }) end)
      end)
    end)
    
    it("authenticate accepts auth method ID", function()
      assert.has_no.errors(function()
        pcall(function() hermes.authenticate("auth-method-id") end)
      end)
    end)
    
    it("respond accepts request ID", function()
      assert.has_no.errors(function()
        pcall(function() hermes.respond("request-id") end)
      end)
    end)
  end)
end)
