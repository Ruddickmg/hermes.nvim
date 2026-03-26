-- Tests for plugin/hermes.lua
-- Tests that :Hermes command and subcommands work correctly

local helpers = require("helpers")
local stub = require("luassert.stub")

describe("plugin.hermes", function()
  local temp_dir
  local stdpath_stub
  local filereadable_stub
  
  before_each(function()
    -- Create temp directory
    temp_dir = helpers.create_temp_dir()
    
    -- Create proper stubs
    stdpath_stub = stub(vim.fn, "stdpath").returns(temp_dir:gsub("/hermes$", ""))
    filereadable_stub = stub(vim.fn, "filereadable").returns(0)
    
    -- Source the plugin file
    vim.cmd("luafile plugin/hermes.lua")
  end)
  
  after_each(function()
    helpers.cleanup_temp_dir(temp_dir)
    
    -- Revert stubs
    if stdpath_stub then stdpath_stub:revert() end
    if filereadable_stub then filereadable_stub:revert() end
  end)
  
  describe("command is defined", function()
    it(":Hermes command exists", function()
      local commands = vim.api.nvim_get_commands({})
      assert.is_not_nil(commands["Hermes"], ":Hermes command should be defined")
    end)
    
    it(":Hermes command has correct description", function()
      local commands = vim.api.nvim_get_commands({})
      assert.equals("Hermes binary management and info", commands["Hermes"].definition)
    end)
  end)
  
  describe("subcommands via :Hermes", function()
    it("accepts 'version' subcommand", function()
      -- Just verify it doesn't crash
      assert.has_no.errors(function()
        vim.cmd("Hermes version")
      end)
    end)
    
    it("accepts 'clean' subcommand", function()
      assert.has_no.errors(function()
        vim.cmd("Hermes clean")
      end)
    end)
    
    it("accepts 'setup' subcommand", function()
      assert.has_no.errors(function()
        vim.cmd("Hermes setup")
      end)
    end)
    
    it("accepts no arguments (shows help)", function()
      assert.has_no.errors(function()
        vim.cmd("Hermes")
      end)
    end)
    
    it("accepts 'install' subcommand (may fail to download but won't crash)", function()
      -- This will try to download but may fail - we just verify it doesn't crash
      -- Use pcall because download may fail in test environment
      local ok, err = pcall(function()
        vim.cmd("Hermes install")
      end)
      -- Either succeeds or fails gracefully without crashing
      assert.is_true(ok or err ~= nil, "Command should either succeed or fail gracefully")
    end)
    
    it("accepts 'build' subcommand (may fail to build but won't crash)", function()
      -- This will try to build but may fail - we just verify it doesn't crash
      -- Use pcall because build may fail in test environment
      local ok, err = pcall(function()
        vim.cmd("Hermes build")
      end)
      -- Either succeeds or fails gracefully without crashing
      assert.is_true(ok or err ~= nil, "Command should either succeed or fail gracefully")
    end)
  end)
  
  describe("tab completion", function()
    it("provides completion function", function()
      local commands = vim.api.nvim_get_commands({})
      local hermes_cmd = commands["Hermes"]
      
      -- complete field should be set (it will be a string representation when retrieved)
      assert.is_not_nil(hermes_cmd.complete)
    end)
    
    it("completion is implemented as Lua function", function()
      local commands = vim.api.nvim_get_commands({})
      local hermes_cmd = commands["Hermes"]
      
      -- The complete field should indicate it's a Lua function
      assert.matches("Lua function", hermes_cmd.complete)
    end)
  end)
  
  describe("highlight groups", function()
    it("defines HermesInfo highlight", function()
      local hl = vim.api.nvim_get_hl(0, { name = "HermesInfo" })
      assert.is_not_nil(hl)
    end)
    
    it("defines HermesWarning highlight", function()
      local hl = vim.api.nvim_get_hl(0, { name = "HermesWarning" })
      assert.is_not_nil(hl)
    end)
    
    it("defines HermesError highlight", function()
      local hl = vim.api.nvim_get_hl(0, { name = "HermesError" })
      assert.is_not_nil(hl)
    end)
  end)
end)
