---Binary download and compilation management
---@module hermes.binary

local M = {}

---@type string Base URL for GitHub releases
local BASE_URL = "https://github.com/Ruddickmg/hermes.nvim/releases/download"

---@type string Repository URL for building from source
local REPO_URL = "https://github.com/Ruddickmg/hermes.nvim.git"

---Get path to binary storage directory
---@return string path Path to data directory
function M.get_data_dir()
  return vim.fn.stdpath("data") .. "/hermes"
end

---Ensure directory exists
---@param path string Directory path
local function ensure_dir(path)
  if vim.fn.isdirectory(path) == 0 then
    vim.fn.mkdir(path, "p")
  end
end

---Get full path to binary file
---@return string path Full path to binary
function M.get_binary_path()
  local platform = require("hermes.platform")
  local data_dir = M.get_data_dir()
  local bin_name = platform.get_binary_name()
  return data_dir .. "/" .. bin_name
end

---Get path to version file
---@return string path Path to version file
function M.get_version_file()
  return M.get_data_dir() .. "/version.txt"
end

---Download binary from GitHub releases
---Shows progress to user during download
---@param dest_path string Destination path for binary
---@param version string Version to download
---@return boolean success Whether download succeeded
function M.download(dest_path, version)
  local platform = require("hermes.platform")
  local bin_name = platform.get_binary_name()
  local url = string.format("%s/%s/%s", BASE_URL, version, bin_name)
  
  vim.notify(
    string.format("Downloading Hermes binary for %s...", platform.get_display_string()),
    vim.log.levels.INFO
  )
  
  ensure_dir(M.get_data_dir())
  
  -- Download with curl
  local cmd = { "curl", "-L", "-o", dest_path, url }
  local result = vim.fn.system(cmd)
  local exit_code = vim.v.shell_error
  
  if exit_code ~= 0 then
    vim.notify("Download failed: " .. result, vim.log.levels.ERROR)
    return false
  end
  
  -- Check if file was actually downloaded (not empty or error page)
  local stat = vim.loop.fs_stat(dest_path)
  if not stat or stat.size < 1000 then
    vim.notify(
      "Downloaded file is too small or empty. Binary may not exist for this platform.",
      vim.log.levels.ERROR
    )
    -- Clean up failed download
    vim.fn.delete(dest_path)
    return false
  end
  
  -- Make executable on Unix
  if platform.get_os() ~= "windows" then
    vim.fn.system({ "chmod", "+x", dest_path })
  end
  
  vim.notify("Binary downloaded successfully!", vim.log.levels.INFO)
  return true
end

---Build binary from source
---Fallback when pre-built binary is not available
---@param dest_dir string Destination directory
---@return boolean success Whether build succeeded
function M.build_from_source(dest_dir)
  vim.notify(
    "Pre-built binary not available for your platform. Building from source...\n" ..
    "This may take a few minutes.",
    vim.log.levels.WARN
  )
  
  ensure_dir(dest_dir)
  local build_dir = dest_dir .. "/build"
  
  -- Clone repository
  vim.notify("Cloning Hermes repository...", vim.log.levels.INFO)
  local clone_cmd = {
    "git", "clone", "--depth", "1", "--branch", "main",
    REPO_URL, build_dir
  }
  local clone_result = vim.fn.system(clone_cmd)
  if vim.v.shell_error ~= 0 then
    vim.notify("Failed to clone repository: " .. clone_result, vim.log.levels.ERROR)
    return false
  end
  
  -- Build with cargo
  vim.notify("Building Hermes from source (this may take a few minutes)...", vim.log.levels.INFO)
  local build_cmd = { "cargo", "build", "--release", "--manifest-path", build_dir .. "/Cargo.toml" }
  local build_result = vim.fn.system(build_cmd)
  if vim.v.shell_error ~= 0 then
    vim.notify("Build failed: " .. build_result, vim.log.levels.ERROR)
    return false
  end
  
  -- Find and copy the built library
  local platform = require("hermes.platform")
  local ext = platform.get_ext()
  local built_lib = build_dir .. "/target/release/libhermes." .. ext
  
  if vim.fn.filereadable(built_lib) == 0 then
    vim.notify("Could not find built library at: " .. built_lib, vim.log.levels.ERROR)
    return false
  end
  
  -- Copy to destination
  local final_path = dest_dir .. "/libhermes." .. ext
  vim.fn.system({ "cp", built_lib, final_path })
  
  -- Clean up build directory
  vim.fn.system({ "rm", "-rf", build_dir })
  
  vim.notify("Build completed successfully!", vim.log.levels.INFO)
  return true
end

---Ensure binary is available
---Downloads or builds as needed
---@return string path Path to binary
function M.ensure_binary()
  local platform = require("hermes.platform")
  local version = require("hermes.version")
  
  -- Check if platform is supported
  local supported, err = platform.is_supported()
  if not supported then
    error("Platform not supported: " .. err)
  end
  
  local bin_path = M.get_binary_path()
  local ver_file = M.get_version_file()
  local wanted_ver = version.get_wanted()
  
  -- Check if we need to download/build
  local needs_download = false
  
  if vim.fn.filereadable(bin_path) == 0 then
    needs_download = true
  else
    -- Check if version matches
    if vim.fn.filereadable(ver_file) == 1 then
      local current_ver = vim.fn.readfile(ver_file)[1]
      if current_ver ~= wanted_ver then
        vim.notify(
          string.format("Version mismatch: have %s, want %s", current_ver, wanted_ver),
          vim.log.levels.INFO
        )
        needs_download = true
      end
    else
      needs_download = true
    end
  end
  
  if needs_download then
    -- Try download first
    local download_ok = M.download(bin_path, wanted_ver)
    
    if not download_ok then
      -- Fallback: build from source
      vim.notify("Download failed, attempting to build from source...", vim.log.levels.WARN)
      local build_ok = M.build_from_source(M.get_data_dir())
      
      if not build_ok then
        -- Both failed - provide helpful error
        local error_msg = string.format(
          "Failed to obtain Hermes binary.\n\n" ..
          "Both download and build failed.\n\n" ..
          "Platform: %s\n" ..
          "Version: %s\n\n" ..
          "This is likely because:\n" ..
          "1. No pre-built binary exists for your platform\n" ..
          "2. Rust/Cargo is not installed for building from source\n\n" ..
          "To build manually:\n" ..
          "  1. Install Rust: https://rustup.rs/\n" ..
          "  2. Clone: git clone %s\n" ..
          "  3. Build: cargo build --release\n" ..
          "  4. Copy target/release/libhermes.* to %s\n\n" ..
          "If you believe this is a bug, please create an issue:\n" ..
          "https://github.com/Ruddickmg/hermes.nvim/issues\n\n" ..
          "Include the error messages above and your platform info.",
          platform.get_display_string(),
          wanted_ver,
          REPO_URL,
          M.get_data_dir()
        )
        error(error_msg)
      end
      
      -- Use built binary path
      bin_path = M.get_data_dir() .. "/libhermes." .. platform.get_ext()
    end
    
    -- Save version
    vim.fn.writefile({wanted_ver}, ver_file)
  end
  
  return bin_path
end

---Load native module
---Ensures binary is available and loads it
---@return table native_module The loaded native module
function M.load_or_build()
  local bin_path = M.ensure_binary()
  
  vim.notify("Loading Hermes binary...", vim.log.levels.DEBUG)
  
  local ok, lib = pcall(package.loadlib, bin_path, "luaopen_hermes")
  if not ok or not lib then
    error(string.format(
      "Failed to load native module from: %s\nError: %s",
      bin_path,
      tostring(lib)
    ))
  end
  
  return lib()
end

return M
