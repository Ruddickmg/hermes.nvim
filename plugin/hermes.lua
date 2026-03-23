---Plugin startup script - auto-sourced by Neovim
---Commands and initialization

-- Version check
if vim.fn.has("nvim-0.11") ~= 1 then
  vim.api.nvim_err_writeln("Hermes requires Neovim >= 0.11")
  return
end

-- Create user command
vim.api.nvim_create_user_command("Hermes", function(args)
  local subcmd = args.fargs[1]
  
  if subcmd == "install" or subcmd == "download" then
    -- Force download/install
    vim.notify("Installing Hermes binary...", vim.log.levels.INFO)
    local ok, err = pcall(function()
      local binary = require("hermes.binary")
      local version = require("hermes.version")
      local ver = version.get_wanted()
      
      -- Clear cache to ensure fresh download
      version.clear_cache()
      
      local path = binary.get_binary_path()
      -- Remove existing binary if present
      if vim.fn.filereadable(path) == 1 then
        vim.fn.delete(path)
      end
      
      -- Download fresh
      local success = binary.download(path, ver)
      if not success then
        error("Download failed")
      end
      
      -- Save version
      vim.fn.writefile({ver}, binary.get_version_file())
    end)
    
    if ok then
      vim.notify("Hermes binary installed successfully!", vim.log.levels.INFO)
    else
      vim.notify("Installation failed: " .. tostring(err), vim.log.levels.ERROR)
    end
    
  elseif subcmd == "build" then
    -- Force build from source
    vim.notify("Building Hermes from source...", vim.log.levels.INFO)
    local ok, err = pcall(function()
      local binary = require("hermes.binary")
      local data_dir = binary.get_data_dir()
      
      -- Remove existing binary
      local path = binary.get_binary_path()
      if vim.fn.filereadable(path) == 1 then
        vim.fn.delete(path)
      end
      
      local success = binary.build_from_source(data_dir)
      if not success then
        error("Build failed")
      end
      
      -- Save version as "built"
      vim.fn.writefile({"built"}, binary.get_version_file())
    end)
    
    if ok then
      vim.notify("Hermes built successfully!", vim.log.levels.INFO)
    else
      vim.notify("Build failed: " .. tostring(err), vim.log.levels.ERROR)
    end
    
  elseif subcmd == "version" or subcmd == "info" then
    -- Show version info
    local platform = require("hermes.platform")
    local version = require("hermes.version")
    local config = require("hermes.config")
    
    local wanted = version.get_wanted()
    local cache_status = version.get_cache_status()
    local platform_str = platform.get_display_string()
    
    print("Hermes Version Information:")
    print("  Wanted version: " .. wanted)
    print("  Platform: " .. platform_str)
    print("  Version cached: " .. tostring(cache_status.cached))
    if cache_status.cached then
      print("  Cached version: " .. tostring(cache_status.version))
      print("  Cache age: " .. tostring(cache_status.age) .. " seconds")
    end
    
    -- Check if binary exists
    local binary = require("hermes.binary")
    local bin_path = binary.get_binary_path()
    local ver_file = binary.get_version_file()
    
    if vim.fn.filereadable(bin_path) == 1 then
      print("  Binary: installed")
      if vim.fn.filereadable(ver_file) == 1 then
        local current = vim.fn.readfile(ver_file)[1]
        print("  Current version: " .. current)
      else
        print("  Current version: unknown")
      end
    else
      print("  Binary: not installed (will download on first use)")
    end
    
  elseif subcmd == "clean" then
    -- Clear binary and cache
    vim.notify("Cleaning Hermes installation...", vim.log.levels.INFO)
    local binary = require("hermes.binary")
    local version = require("hermes.version")
    local data_dir = binary.get_data_dir()
    
    -- Remove data directory
    if vim.fn.isdirectory(data_dir) == 1 then
      vim.fn.system({"rm", "-rf", data_dir})
    end
    
    -- Clear version cache
    version.clear_cache()
    
    vim.notify("Hermes cleaned successfully!", vim.log.levels.INFO)
    
  elseif subcmd == "setup" or subcmd == "config" then
    -- Show current configuration
    local config = require("hermes.config")
    local current = config.get()
    
    print("Hermes Configuration:")
    print(vim.inspect(current))
    
  else
    vim.notify(
      "Usage: :Hermes {install|build|version|clean|setup}\n\n" ..
      "Commands:\n" ..
      "  install  - Download and install the binary\n" ..
      "  build    - Build binary from source\n" ..
      "  version  - Show version information\n" ..
      "  clean    - Remove binary and cache\n" ..
      "  setup    - Show current configuration",
      vim.log.levels.INFO
    )
  end
end, {
  nargs = "?",
  complete = function()
    return {"install", "build", "version", "clean", "setup"}
  end,
  desc = "Hermes binary management and info"
})

-- Create highlight group for hermes notifications (optional)
vim.api.nvim_set_hl(0, "HermesInfo", { link = "DiagnosticInfo" })
vim.api.nvim_set_hl(0, "HermesWarning", { link = "DiagnosticWarn" })
vim.api.nvim_set_hl(0, "HermesError", { link = "DiagnosticError" })

-- Lazy-load on first API call - no eager initialization
-- The binary is only downloaded/built when user calls require("hermes").api_method()
