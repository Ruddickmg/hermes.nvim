local download = require("hermes.download")
local logging = require("hermes.logging")

---Binary download and compilation management
---@module hermes.binary

local M = {}

---@type string Base URL for GitHub releases
local BASE_URL = "https://github.com/Ruddickmg/hermes.nvim/releases/download"

---@type string Repository URL for building from source
local REPO_URL = "https://github.com/Ruddickmg/hermes.nvim.git"

---List of officially supported platforms for pre-built binaries
---@type table<string, boolean>
M.SUPPORTED_PLATFORMS = {
  ["linux-x86_64"] = true,
  ["linux-aarch64"] = true,
  ["macos-x86_64"] = true,
  ["macos-aarch64"] = true,
  ["windows-x86_64"] = true,
}

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
  
    logging.notify(
    string.format("Downloading Hermes binary for %s...", platform.get_display_string()),
    vim.log.levels.INFO
  )
  
  ensure_dir(M.get_data_dir())
  
  -- Download using the download module
  local ok, err = download.download(url, dest_path)
  
  if not ok then
    logging.notify("Download failed: " .. (err or "Unknown error"), vim.log.levels.ERROR)
    return false
  end
  
  -- Make executable on Unix
  if platform.get_os() ~= "windows" then
    vim.fn.system({ "chmod", "+x", dest_path })
  end
  
    logging.notify("Binary downloaded successfully!", vim.log.levels.INFO)
  return true
end

---Build binary from source
---Fallback when pre-built binary is not available
---@param dest_dir string Destination directory
---@return boolean success Whether build succeeded
function M.build_from_source(dest_dir)
    logging.notify(
    "Pre-built binary not available for your platform. Building from source...\n" ..
    "This may take a few minutes.",
    vim.log.levels.WARN
  )
  
  ensure_dir(dest_dir)
  local build_dir = dest_dir .. "/build"
  
  -- Clone repository
    logging.notify("Cloning Hermes repository...", vim.log.levels.INFO)
  local clone_cmd = {
    "git", "clone", "--depth", "1", "--branch", "main",
    REPO_URL, build_dir
  }
  local clone_result = vim.fn.system(clone_cmd)
  if vim.v.shell_error ~= 0 then
    logging.notify("Failed to clone repository: " .. clone_result, vim.log.levels.ERROR)
    return false
  end
  
  -- Build with cargo
    logging.notify("Building Hermes from source (this may take a few minutes)...", vim.log.levels.INFO)
  local build_cmd = { "cargo", "build", "--release", "--manifest-path", build_dir .. "/Cargo.toml" }
  local build_result = vim.fn.system(build_cmd)
  if vim.v.shell_error ~= 0 then
    logging.notify("Build failed: " .. build_result, vim.log.levels.ERROR)
    return false
  end
  
  -- Find and copy the built library
  local platform = require("hermes.platform")
  local ext = platform.get_ext()
  local built_lib = build_dir .. "/target/release/libhermes." .. ext
  
  if vim.fn.filereadable(built_lib) == 0 then
    logging.notify("Could not find built library at: " .. built_lib, vim.log.levels.ERROR)
    return false
  end
  
  -- Copy to destination
  local final_path = dest_dir .. "/libhermes." .. ext
  vim.fn.system({ "cp", built_lib, final_path })
  
  -- Clean up build directory
  vim.fn.system({ "rm", "-rf", build_dir })
  
    logging.notify("Build completed successfully!", vim.log.levels.INFO)
  return true
end

---Ensure binary is available
---Downloads pre-built binary if platform is supported
---Returns error with helpful message if platform not supported
---@return string path Path to binary
function M.ensure_binary()
  local platform = require("hermes.platform")
  local version = require("hermes.version")
  
  -- Check if platform is supported for pre-built binaries
  local platform_key = platform.get_platform_key()
  if not platform_key then
    error(
      "Unable to determine platform.\n\n" ..
      "Please check the installation instructions:\n" ..
      "https://github.com/Ruddickmg/hermes.nvim#installation"
    )
  end
  
  if not M.SUPPORTED_PLATFORMS[platform_key] then
    local supported_list = {}
    for plat, _ in pairs(M.SUPPORTED_PLATFORMS) do
      table.insert(supported_list, "  - " .. plat:gsub("-", " "):gsub("^%l", string.upper))
    end
    table.sort(supported_list)
    
    error(
      string.format(
        "Platform not supported for automatic binary download: %s\n\n" ..
        "Pre-built binaries are available for these platforms:\n%s\n\n" ..
        "To use Hermes on your platform, you have two options:\n\n" ..
        "Option 1 - Build manually (Recommended):\n" ..
        "  1. Install Rust: https://rustup.rs/\n" ..
        "  2. Run :Hermes build inside Neovim\n\n" ..
        "Option 2 - Build outside Neovim:\n" ..
        "  1. Clone: git clone %s\n" ..
        "  2. Build: cargo build --release\n" ..
        "  3. Copy target/release/libhermes.* to %s\n\n" ..
        "For detailed instructions, see:\n" ..
        "https://github.com/Ruddickmg/hermes.nvim#installation",
        platform.get_display_string(),
        table.concat(supported_list, "\n"),
        REPO_URL,
        M.get_data_dir()
      )
    )
  end
  
  -- Check if download tools are available
  local download_tool = download.get_available_tool()
  if not download_tool then
    error(
      "Unable to download Hermes binary.\n\n" ..
      "No download tool found. Please install one of the following:\n" ..
      "  - curl (preferred)\n" ..
      "  - wget\n\n" ..
      "Alternatively, you can build from source:\n" ..
      "  1. Install Rust: https://rustup.rs/\n" ..
      "  2. Run :Hermes build inside Neovim\n\n" ..
      "For detailed instructions, see:\n" ..
      "https://github.com/Ruddickmg/hermes.nvim#installation"
    )
  end
  
  local bin_path = M.get_binary_path()
  local ver_file = M.get_version_file()
  local wanted_ver = version.get_wanted()
  
  -- Check if we need to download
  local needs_download = false
  
  if vim.fn.filereadable(bin_path) == 0 then
    needs_download = true
  else
    -- Check if version matches
    if vim.fn.filereadable(ver_file) == 1 then
      local current_ver = vim.fn.readfile(ver_file)[1]
      if current_ver ~= wanted_ver then
        logging.notify(
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
    -- Download binary for supported platform
    local download_ok = M.download(bin_path, wanted_ver)
    
    if not download_ok then
      -- Download failed on a supposedly supported platform
      error(
        string.format(
          "Failed to download Hermes binary for %s.\n\n" ..
          "This is unexpected for a supported platform.\n\n" ..
          "Troubleshooting steps:\n" ..
          "  1. Check your internet connection\n" ..
          "  2. Check if GitHub is accessible\n" ..
          "  3. The release may not exist yet for version %s\n\n" ..
          "To build manually:\n" ..
          "  1. Install Rust: https://rustup.rs/\n" ..
          "  2. Run :Hermes build inside Neovim\n\n" ..
          "For detailed instructions, see:\n" ..
          "https://github.com/Ruddickmg/hermes.nvim#installation",
          platform.get_display_string(),
          wanted_ver
        )
      )
    end
    
    -- Save version
    vim.fn.writefile({wanted_ver}, ver_file)
  end
  
  return bin_path
end

---Load existing binary without downloading
---Checks if binary exists at expected path, errors if not found
---@return string path Path to existing binary
function M.load_existing_binary()
  local bin_path = M.get_binary_path()
  
  -- Check if binary already exists
  if vim.fn.filereadable(bin_path) == 0 then
    local platform = require("hermes.platform")
    error(
      string.format(
        "Binary not found and auto_download_binary is disabled.\n\n" ..
        "Current platform: %s\n\n" ..
        "To resolve this, choose one option:\n\n" ..
        "Option 1 - Enable auto-download in your config:\n" ..
        "  require(\"hermes\").setup({\n" ..
        "    auto_download_binary = true,\n" ..
        "  })\n\n" ..
        "Option 2 - Build manually:\n" ..
        "  1. Install Rust: https://rustup.rs/\n" ..
        "  2. Run :Hermes build inside Neovim\n\n" ..
        "For detailed instructions, see:\n" ..
        "https://github.com/Ruddickmg/hermes.nvim#installation",
        platform.get_display_string()
      )
    )
  end
  
    logging.notify("Using existing Hermes binary: " .. bin_path, vim.log.levels.INFO)
  return bin_path
end

---Load native module
---Ensures binary is available and loads it
---@return table native_module The loaded native module
function M.load_or_build()
  local bin_path = M.ensure_binary()
  
    logging.notify("Loading Hermes binary...", vim.log.levels.DEBUG)
  
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
