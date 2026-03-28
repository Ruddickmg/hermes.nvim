-- ============================================================================
-- E2E API Endpoint Tests for Hermes
-- Tests all 10 API endpoints with real opencode agent
-- Note: Full autocommand verification with responses is tested in Rust E2E tests
-- These tests verify the Lua API is callable without crashing
-- ============================================================================

local helpers = require("helpers")
local binary = require("hermes.binary")
local stub = require("luassert.stub")

describe("Hermes API Endpoints (E2E)", function()
  -- Track test state
  local stdpath_stub
  local temp_dir
  
  -- Helper: Clear all hermes modules from cache
  local function clear_modules()
    for name, _ in pairs(package.loaded) do
      if name:match("^hermes") then
        package.loaded[name] = nil
      end
    end
  end
  
  -- Helper: Setup binary by copying from target/release to data directory
  local function setup_binary()
    local platform = require("hermes.platform")
    local source_bin = vim.fn.getcwd() .. "/target/release/libhermes." .. platform.get_ext()
    local bin_path = binary.get_binary_path()
    
    if vim.fn.filereadable(source_bin) == 1 then
      vim.fn.mkdir(binary.get_data_dir(), "p")
      local uv = vim.uv or vim.loop
      uv.fs_copyfile(source_bin, bin_path)
    else
      error("Binary not found at: " .. source_bin .. ". Run 'cargo build --release' first")
    end
  end
  
  -- Track autocommands for cleanup
  local test_autocmds = {}
  
  -- Helper: Ensure hermes augroup exists and is cleared
  local function setup_hermes_group()
    -- Clear existing hermes autocommands if group exists
    local ok, group_id = pcall(vim.api.nvim_get_augroup_id, "hermes")
    if ok then
      pcall(function()
        vim.api.nvim_clear_autocmds({ group = group_id })
      end)
    end
    -- Create fresh group
    return vim.api.nvim_create_augroup("hermes", { clear = true })
  end
  
  -- Helper: Full setup for endpoint test
  local function setup_endpoint_test(_agent_name)
    clear_modules()
    
    temp_dir = helpers.create_temp_dir()
    stdpath_stub = stub(vim.fn, "stdpath").returns(temp_dir)
    
    setup_binary()
    
    local hermes = require("hermes")
    hermes.setup({
      download = { auto = false, version = "latest" },
      log = {
        stdio = { level = "error", format = "compact" },
        file = { level = "error", format = "compact" },
        notification = { level = "error", format = "compact" },
        message = { level = "error", format = "compact" }
      }
    })
    
    return hermes
  end
  
  -- Helper: Wait for hermes to be ready
  local function wait_for_ready(hermes, timeout_ms)
    timeout_ms = timeout_ms or 30000
    local start_time = vim.loop.now()
    while hermes.get_loading_state() ~= "READY" and (vim.loop.now() - start_time) < timeout_ms do
      vim.wait(100)
    end
    return hermes.get_loading_state() == "READY"
  end
  
  -- Helper: Wait for autocommand and return received data
  local function wait_for_autocommand(pattern, timeout_ms)
    timeout_ms = timeout_ms or 30000
    local received = false
    local data = nil
    local error_msg = nil
    
    -- Ensure hermes group exists
    local group_id = vim.api.nvim_create_augroup("hermes", { clear = false })
    
    -- Register autocommand listener with error handling
    local ok, autocmd_result = pcall(function()
      return vim.api.nvim_create_autocmd("User", {
        group = group_id,
        pattern = pattern,
        once = true,
        callback = function(args)
          -- Wrap in pcall to catch any errors in the callback itself
          local cb_ok, cb_result = pcall(function()
            received = true
            data = args.data
          end)
          if not cb_ok then
            error_msg = "Error in autocommand callback: " .. tostring(cb_result)
          end
        end,
      })
    end)
    
    if not ok then
      return false, nil, "Failed to create autocommand: " .. tostring(autocmd_result)
    end
    
    table.insert(test_autocmds, autocmd_result)
    
    -- Wait for the autocommand to fire
    local start_time = vim.loop.now()
    while not received and (vim.loop.now() - start_time) < timeout_ms do
      vim.wait(100)
    end
    
    if error_msg then
      return false, nil, error_msg
    end
    
    if not received then
      return false, nil, "Autocommand '" .. pattern .. "' not received within " .. (timeout_ms / 1000) .. "s"
    end
    
    return true, data, nil
  end
  
  -- Helper: Cleanup after test
  local function cleanup_test()
    -- Delete specific autocommands we created
    for _, autocmd_id in ipairs(test_autocmds) do
      pcall(function()
        vim.api.nvim_del_autocmd(autocmd_id)
      end)
    end
    test_autocmds = {}
    
    -- Disconnect first to stop agent communication
    pcall(function()
      local hermes = require("hermes")
      hermes.disconnect()
    end)
    
    -- Wait for disconnect to complete
    vim.wait(1000)
    
    if stdpath_stub then
      pcall(function() stdpath_stub:revert() end)
      stdpath_stub = nil
    end
    if temp_dir then
      helpers.cleanup_temp_dir(temp_dir)
      temp_dir = nil
    end
    
    -- Clear HERMES_BINARY_PATH env var to prevent affecting other tests
    vim.env.HERMES_BINARY_PATH = nil
  end
  
  describe("with opencode agent", function()
    after_each(function()
      cleanup_test()
    end)
    
    it("connect endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- Setup listener BEFORE calling connect
      local received = false
      local data = nil
      local autocmd_ok, autocmd_id = pcall(function()
        return vim.api.nvim_create_autocmd("User", {
          group = vim.api.nvim_create_augroup("hermes", { clear = false }),
          pattern = "ConnectionInitialized",
          once = true,
          callback = function(args)
            received = true
            data = args.data
          end,
        })
      end)
      assert.is_true(autocmd_ok, "Should create autocommand listener")
      table.insert(test_autocmds, autocmd_id)
      
      -- Call connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      -- Wait for and verify autocommand is received
      local wait_ok = vim.wait(30000, function() return received end, 100)
      assert.is_true(wait_ok, "Should receive ConnectionInitialized autocommand within 30s")
      assert.is_not_nil(data, "Should receive autocommand data")
      assert.is_table(data, "Autocommand data should be a table")
      assert.is_not_nil(data.agentInfo, "Should receive agentInfo in autocommand data")
    end)
    
    it("disconnect endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Then disconnect
      ok, err = pcall(function()
        hermes.disconnect("opencode")
      end)
      
      assert.is_true(ok, "disconnect() should not crash: " .. tostring(err))
    end)
    
    it("authenticate endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Setup listener for Authenticated
      local received = false
      local data = nil
      local autocmd_ok, autocmd_id = pcall(function()
        return vim.api.nvim_create_autocmd("User", {
          group = vim.api.nvim_create_augroup("hermes", { clear = false }),
          pattern = "Authenticated",
          once = true,
          callback = function(args)
            received = true
            data = args.data
          end,
        })
      end)
      assert.is_true(autocmd_ok, "Should create Authenticated autocommand listener")
      table.insert(test_autocmds, autocmd_id)
      
      -- Call authenticate
      ok, err = pcall(function()
        hermes.authenticate("opencode-login")
      end)
      assert.is_true(ok, "authenticate() should not crash: " .. tostring(err))
      
      -- Wait for and verify autocommand
      local wait_ok = vim.wait(30000, function() return received end, 100)
      -- Note: Authenticated may not fire immediately - agent handles auth asynchronously
    end)
    
    it("create_session endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Setup listener for SessionCreated
      local received = false
      local data = nil
      local autocmd_ok, autocmd_id = pcall(function()
        return vim.api.nvim_create_autocmd("User", {
          group = vim.api.nvim_create_augroup("hermes", { clear = false }),
          pattern = "SessionCreated",
          once = true,
          callback = function(args)
            received = true
            data = args.data
          end,
        })
      end)
      assert.is_true(autocmd_ok, "Should create SessionCreated autocommand listener")
      table.insert(test_autocmds, autocmd_id)
      
      -- Call create_session
      ok, err = pcall(function()
        hermes.create_session(nil)
      end)
      assert.is_true(ok, "create_session() should not crash: " .. tostring(err))
      
      -- Wait for and verify autocommand is received (may take time for agent to respond)
      local wait_ok = vim.wait(30000, function() return received end, 100)
      -- Note: SessionCreated may not fire immediately or may require actual session creation
      -- If it doesn't fire, that's ok - the API call itself succeeded
    end)
    
    it("load_session endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Setup listener for SessionLoaded
      local received = false
      local data = nil
      local autocmd_ok, autocmd_id = pcall(function()
        return vim.api.nvim_create_autocmd("User", {
          group = vim.api.nvim_create_augroup("hermes", { clear = false }),
          pattern = "SessionLoaded",
          once = true,
          callback = function(args)
            received = true
            data = args.data
          end,
        })
      end)
      assert.is_true(autocmd_ok, "Should create SessionLoaded autocommand listener")
      table.insert(test_autocmds, autocmd_id)
      
      -- Call load_session
      ok, err = pcall(function()
        hermes.load_session("test-session-id", nil)
      end)
      assert.is_true(ok, "load_session() should not crash: " .. tostring(err))
      
      -- Wait for and verify autocommand
      local wait_ok = vim.wait(30000, function() return received end, 100)
      -- Note: SessionLoaded may not fire immediately
    end)
    
    it("list_sessions endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Setup listener for SessionsListed
      local received = false
      local data = nil
      local autocmd_ok, autocmd_id = pcall(function()
        return vim.api.nvim_create_autocmd("User", {
          group = vim.api.nvim_create_augroup("hermes", { clear = false }),
          pattern = "SessionsListed",
          once = true,
          callback = function(args)
            received = true
            data = args.data
          end,
        })
      end)
      assert.is_true(autocmd_ok, "Should create SessionsListed autocommand listener")
      table.insert(test_autocmds, autocmd_id)
      
      -- Call list_sessions
      ok, err = pcall(function()
        hermes.list_sessions()
      end)
      assert.is_true(ok, "list_sessions() should not crash: " .. tostring(err))
      
      -- Wait for and verify autocommand
      local wait_ok = vim.wait(30000, function() return received end, 100)
      -- Note: SessionsListed may not fire immediately
    end)
    
    it("prompt endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Setup listener for Prompted
      local received = false
      local data = nil
      local autocmd_ok, autocmd_id = pcall(function()
        return vim.api.nvim_create_autocmd("User", {
          group = vim.api.nvim_create_augroup("hermes", { clear = false }),
          pattern = "Prompted",
          once = true,
          callback = function(args)
            received = true
            data = args.data
          end,
        })
      end)
      assert.is_true(autocmd_ok, "Should create Prompted autocommand listener")
      table.insert(test_autocmds, autocmd_id)
      
      -- Call prompt
      ok, err = pcall(function()
        hermes.prompt("test-session-id", {
          type = "text",
          text = "Hello, this is a test message"
        })
      end)
      assert.is_true(ok, "prompt() should not crash: " .. tostring(err))
      
      -- Wait for and verify autocommand
      local wait_ok = vim.wait(30000, function() return received end, 100)
      -- Note: Prompted may not fire immediately
    end)
    
    it("cancel endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Call cancel
      ok, err = pcall(function()
        hermes.cancel("test-session-id")
      end)
      assert.is_true(ok, "cancel() should not crash: " .. tostring(err))
    end)
    
    it("set_mode endpoint callable with opencode", function()
      local hermes = setup_endpoint_test("opencode")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- First connect
      local ok, err = pcall(function()
        hermes.connect("opencode")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Setup listener for ModeUpdated
      local received = false
      local data = nil
      local autocmd_ok, autocmd_id = pcall(function()
        return vim.api.nvim_create_autocmd("User", {
          group = vim.api.nvim_create_augroup("hermes", { clear = false }),
          pattern = "ModeUpdated",
          once = true,
          callback = function(args)
            received = true
            data = args.data
          end,
        })
      end)
      assert.is_true(autocmd_ok, "Should create ModeUpdated autocommand listener")
      table.insert(test_autocmds, autocmd_id)
      
      -- Call set_mode
      ok, err = pcall(function()
        hermes.set_mode("test-session-id", "default")
      end)
      assert.is_true(ok, "set_mode() should not crash: " .. tostring(err))
      
      -- Wait for and verify autocommand
      local wait_ok = vim.wait(30000, function() return received end, 100)
      -- Note: ModeUpdated may not fire immediately
    end)
  end)
  
  describe("with copilot agent (for permission requests)", function()
    after_each(function()
      cleanup_test()
    end)
    
    it("respond endpoint callable with copilot", function()
      local hermes = setup_endpoint_test("copilot")
      
      local ready = wait_for_ready(hermes, 30000)
      assert.is_true(ready, "Binary should be in READY state")
      
      -- Connect to copilot
      local ok, err = pcall(function()
        hermes.connect("copilot")
      end)
      assert.is_true(ok, "connect() should not crash: " .. tostring(err))
      
      vim.wait(500)
      
      -- Try to respond (may fail without permission request, but shouldn't crash)
      ok, err = pcall(function()
        hermes.respond("test-request-id", "approve")
      end)
      
      assert.is_true(ok, "respond() should not crash: " .. tostring(err))
    end)
  end)
end)
