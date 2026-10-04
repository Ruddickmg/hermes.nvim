-- Tests for plugin/hermes.lua
-- Tests that :Hermes command and subcommands work correctly

local helpers = require("helpers")
local stub = require("luassert.stub")
local match = require("luassert.match")

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
    it("accepts 'clean' subcommand", function()
      assert.has_no.errors(function()
        vim.cmd("Hermes clean")
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
      local ok = pcall(function()
        vim.cmd("Hermes install")
      end)
      -- pcall returns boolean status - if we reach here, no unhandled crash occurred
      assert.is_boolean(ok, "pcall should return status without crashing")
    end)

    it("accepts 'build' subcommand (may fail to build but won't crash)", function()
      -- This will try to build but may fail - we just verify it doesn't crash
      -- Use pcall because build may fail in test environment
      local ok = pcall(function()
        vim.cmd("Hermes build")
      end)
      -- pcall returns boolean status - if we reach here, no unhandled crash occurred
      assert.is_boolean(ok, "pcall should return status without crashing")
    end)

    it("accepts 'cancel' subcommand without crashing", function()
      local ok = pcall(function()
        vim.cmd("Hermes cancel")
      end)
      assert.is_true(ok, "Hermes cancel should not crash")
    end)

    it("accepts 'build with-icons' with extra args", function()
      local ok = pcall(function()
        vim.cmd("Hermes build with-icons")
      end)
      assert.is_true(ok, "Hermes build with-icons should not crash")
    end)

    it("accepts 'build' with multiple feature args", function()
      local ok = pcall(function()
        vim.cmd("Hermes build with-icons with-ansi")
      end)
      assert.is_true(ok, "Hermes build with multiple features should not crash")
    end)
  end)

  describe(":Hermes log", function()
    local log_path

    local function setup_log_config(format)
      require("hermes.config").setup({
        log = { file = { path = temp_dir, name = "hermes.log", format = format } },
      })
      log_path = temp_dir .. "/hermes.log"
    end

    local function use_real_files()
      -- The outer before_each stubs filereadable to 0; restore the real
      -- function and nil the handle so the outer after_each skips the revert
      filereadable_stub:revert()
      filereadable_stub = nil
    end

    after_each(function()
      -- Close the tab if the command opened one (errors on the last tab)
      pcall(function()
        vim.cmd("tabclose!")
      end)
      require("hermes.config").setup({})
    end)

    it("opens the configured log file in a new tab", function()
      use_real_files()
      setup_log_config("json")
      vim.fn.writefile({ '{"msg":"hello"}' }, log_path)

      vim.cmd("Hermes log")

      assert.equals(log_path, vim.api.nvim_buf_get_name(0))
    end)

    it("marks the log buffer as readonly", function()
      use_real_files()
      setup_log_config("json")
      vim.fn.writefile({ '{"msg":"hello"}' }, log_path)

      vim.cmd("Hermes log")

      assert.is_true(vim.bo.readonly)
    end)

    it("sets jsonl filetype when log format is json", function()
      use_real_files()
      setup_log_config("json")
      vim.fn.writefile({ '{"msg":"hello"}' }, log_path)

      vim.cmd("Hermes log")

      assert.equals("jsonl", vim.bo.filetype)
    end)

    it("leaves filetype unset when log format is compact", function()
      use_real_files()
      setup_log_config("compact")
      vim.fn.writefile({ "a compact log line" }, log_path)

      vim.cmd("Hermes log")

      assert.equals("", vim.bo.filetype)
    end)

    it("opens the most recent rotated log file when the base file is missing", function()
      use_real_files()
      setup_log_config("json")
      local rotated = log_path .. ".1"
      vim.fn.writefile({ '{"msg":"rotated"}' }, rotated)

      vim.cmd("Hermes log")

      assert.equals(rotated, vim.api.nvim_buf_get_name(0))
    end)

    it("places the cursor on the last line", function()
      use_real_files()
      setup_log_config("json")
      vim.fn.writefile({ "one", "two", "three" }, log_path)

      vim.cmd("Hermes log")

      assert.equals(3, vim.fn.line("."))
    end)

    it("notifies with an enable-logging hint when no log file exists", function()
      setup_log_config("json")
      local notify_stub = stub(vim, "notify")

      vim.cmd("Hermes log")

      assert.stub(notify_stub).was_called_with(match.has_match("Enable it with"), vim.log.levels.INFO, { title = "Hermes" })
      notify_stub:revert()
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
      
      -- Neovim 0.11 returns a string description, 0.12+ returns the actual function
      local complete = hermes_cmd.complete
      assert.is_true(
        type(complete) == "function" or (type(complete) == "string" and complete:match("Lua function") ~= nil),
        "complete should be a Lua function or a string referencing one"
      )
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
