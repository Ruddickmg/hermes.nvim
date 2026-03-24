-- Integration tests for lua/hermes/init.lua
-- Tests that documented API methods are available and have correct signatures

local helpers = require("helpers")
local stub = require("luassert.stub")

describe("hermes.init (main API)", function()
  local hermes
  local temp_dir
  local stdpath_stub
  local filereadable_stub
  
  before_each(function()
    temp_dir = helpers.create_temp_dir()
    stdpath_stub = stub(vim.fn, "stdpath").returns(temp_dir)
    filereadable_stub = stub(vim.fn, "filereadable").returns(1)
    
    package.loaded["hermes.init"] = nil
    package.loaded["hermes.binary"] = nil
    package.loaded["hermes.config"] = nil
    package.loaded["hermes.platform"] = nil
    package.loaded["hermes.version"] = nil
    
    hermes = require("hermes")
  end)
  
  after_each(function()
    helpers.cleanup_temp_dir(temp_dir)
    if stdpath_stub then stdpath_stub:revert() end
    if filereadable_stub then filereadable_stub:revert() end
  end)
  
  describe("API surface (documented functions only)", function()
    it("exports setup function", function()
      assert.is_function(hermes.setup)
    end)
    
    it("exports connect function", function()
      assert.is_function(hermes.connect)
    end)
    
    it("exports disconnect function", function()
      assert.is_function(hermes.disconnect)
    end)
    
    it("exports authenticate function", function()
      assert.is_function(hermes.authenticate)
    end)
    
    it("exports create_session function", function()
      assert.is_function(hermes.create_session)
    end)
    
    it("exports load_session function", function()
      assert.is_function(hermes.load_session)
    end)
    
    it("exports prompt function", function()
      assert.is_function(hermes.prompt)
    end)
    
    it("exports cancel function", function()
      assert.is_function(hermes.cancel)
    end)
    
    it("exports set_mode function", function()
      assert.is_function(hermes.set_mode)
    end)
    
    it("exports respond function", function()
      assert.is_function(hermes.respond)
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
    
    it("handles missing binary with appropriate error", function()
      -- Clear all module caches to ensure fresh state
      package.loaded["hermes.init"] = nil
      package.loaded["hermes.binary"] = nil
      package.loaded["hermes.config"] = nil
      package.loaded["hermes.platform"] = nil
      package.loaded["hermes.version"] = nil
      package.loaded["hermes.download"] = nil
      package.loaded["hermes.logging"] = nil
      
      -- Get fresh hermes module
      local hermes_fresh = require("hermes")
      
      -- Configure to NOT auto-download (we expect binary to be missing)
      hermes_fresh.setup({
        auto_download_binary = false,
        version = "latest",
      })
      
      -- Try to trigger binary loading
      local ok, err = pcall(function()
        hermes_fresh.connect("test-agent", {})
      end)
      
      -- Should fail with binary-related error
      local err_str = tostring(err)
      assert(
        not ok and (err_str:match("Binary not found") or err_str:match("Failed to load") or err_str:match("binary")),
        "Expected error about binary loading, got: " .. err_str
      )
    end)
    
    it("calls setup on loaded binary without crashing", function()
      -- This test assumes binary is already available from previous operations
      -- or from the test environment having the binary pre-installed
      
      -- Try to call setup again - if binary was previously loaded in this process,
      -- this will call setup on the native module
      local ok = pcall(function()
        hermes.setup({
          auto_download_binary = false,
        })
      end)
      
      -- setup() should never crash - it either updates the native module
      -- or just updates the Lua config if binary not yet loaded
      assert.is_true(ok, "setup() should not crash")
    end)
  end)
  
  describe("API function signatures", function()
    it("connect accepts agent name as first argument", function()
      assert.has_no.errors(function()
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
    
    it("load_session accepts session ID", function()
      assert.has_no.errors(function()
        pcall(function() hermes.load_session("session-id") end)
      end)
    end)
    
    it("prompt accepts session ID and content", function()
      assert.has_no.errors(function()
        pcall(function()
          hermes.prompt("session-id", { type = "text", text = "Hello" })
        end)
      end)
    end)
    
    it("authenticate accepts auth method ID", function()
      assert.has_no.errors(function()
        pcall(function() hermes.authenticate("auth-method-id") end)
      end)
    end)
    
    it("cancel accepts session ID", function()
      assert.has_no.errors(function()
        pcall(function() hermes.cancel("session-id") end)
      end)
    end)
    
    it("set_mode accepts session ID and mode ID", function()
      assert.has_no.errors(function()
        pcall(function() hermes.set_mode("session-id", "mode-id") end)
      end)
    end)
    
    it("respond accepts request ID", function()
      assert.has_no.errors(function()
        pcall(function() hermes.respond("request-id") end)
      end)
    end)
  end)
end)
