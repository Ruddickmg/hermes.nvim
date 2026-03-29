-- luacov: disable
---Platform detection utilities
---@module hermes.platform
-- luacov: enable

local M = {}

---Get operating system
-- luacov: disable
---@return string os_name Operating system: "linux", "macos", or "windows"
-- luacov: enable
function M.get_os()
  local uname = vim.loop.os_uname()
  local sysname = uname.sysname
  
  if sysname == "Linux" then
    return "linux"
  elseif sysname == "Darwin" then
    return "macos"
  elseif sysname:match("Windows") or sysname:match("WIN") then
    return "windows"
  else
    -- Try to detect from vim functions
    if vim.fn.has("win32") == 1 or vim.fn.has("win64") == 1 then
      return "windows"
    elseif vim.fn.has("mac") == 1 or vim.fn.has("osx") == 1 then
      return "macos"
    elseif vim.fn.has("linux") == 1 then
      return "linux"
    end
    
    error("Unsupported operating system: " .. sysname)
  end
end

---Get architecture
-- luacov: disable
---@return string arch Architecture: "x86_64" or "aarch64"
-- luacov: enable
function M.get_arch()
  local machine = vim.loop.os_uname().machine
  
  -- Normalize architecture names
  if machine == "x86_64" or machine == "amd64" or machine == "x64" then
    return "x86_64"
  elseif machine == "aarch64" or machine == "arm64" then
    return "aarch64"
  elseif machine == "i386" or machine == "i686" then
    error("x86 (32-bit) architecture is not supported. Please use x86_64 or aarch64.")
  else
    error("Unsupported architecture: " .. machine .. ". Please use x86_64 or aarch64.")
  end
end

---Get library extension for current platform
-- luacov: disable
---@return string ext Library extension: "so", "dylib", or "dll"
-- luacov: enable
function M.get_ext()
  local os = M.get_os()
  if os == "linux" then
    return "so"
  elseif os == "macos" then
    return "dylib"
  elseif os == "windows" then
    return "dll"
  end
end

---Get binary filename for current platform
-- luacov: disable
---@return string filename Binary filename (e.g., "libhermes-linux-x86_64.so")
-- luacov: enable
function M.get_binary_name()
  local os = M.get_os()
  local arch = M.get_arch()
  local ext = M.get_ext()
  return string.format("libhermes-%s-%s.%s", os, arch, ext)
end

---Check if current platform is supported
-- luacov: disable
---@return boolean supported Whether platform is supported
---@return string|nil error Error message if not supported
-- luacov: enable
function M.is_supported()
  local ok, err = pcall(function()
    M.get_os()
    M.get_arch()
  end)
  return ok, err
end

---Get platform string for display
-- luacov: disable
---@return string platform Platform string (e.g., "Linux x86_64")
-- luacov: enable
function M.get_display_string()
  local ok, os, arch = pcall(function()
    return M.get_os(), M.get_arch()
  end)
  if ok then
    return string.format("%s %s", os:gsub("^%l", string.upper), arch)
  else
    return "Unknown Platform"
  end
end

---Get platform key for checking against supported platforms
-- luacov: disable
---@return string|nil key Platform key (e.g., "linux-x86_64") or nil if not determinable
-- luacov: enable
function M.get_platform_key()
  local ok, os, arch = pcall(function()
    return M.get_os(), M.get_arch()
  end)
  if ok then
    return string.format("%s-%s", os, arch)
  end
  return nil
end

return M
