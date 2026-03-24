---HTTP download utilities
---@module hermes.download
---Provides a clean wrapper around HTTP download functionality with curl/wget fallback

local M = {}

---Check if curl is available on the system
---@return boolean available
function M.is_curl_available()
  return vim.fn.executable("curl") == 1
end

---Check if wget is available on the system
---@return boolean available
function M.is_wget_available()
  return vim.fn.executable("wget") == 1
end

---Get available download tool (curl preferred, wget fallback)
---@return string|nil tool_name Name of available tool, or nil if none
function M.get_available_tool()
  if M.is_curl_available() then
    return "curl"
  elseif M.is_wget_available() then
    return "wget"
  end
  return nil
end

---Download file from URL using available tool (curl or wget)
---@param url string URL to download
---@param dest_path string Destination path
---@return boolean success Whether download succeeded
---@return string|nil error Error message if failed
function M.download(url, dest_path)
  local tool = M.get_available_tool()
  
  if not tool then
    return false, "Neither curl nor wget is available. Please install one of them."
  end
  
  local cmd
  if tool == "curl" then
    cmd = { "curl", "-sL", "-o", dest_path, url }
  else
    -- wget fallback
    cmd = { "wget", "-q", "-O", dest_path, url }
  end
  
  local result = vim.fn.system(cmd)
  
  if vim.v.shell_error ~= 0 then
    -- Check if it's a command not found error vs network error
    if result:match("command not found") or result:match("not installed") then
      return false, tool .. " appears to be installed but execution failed: " .. result
    end
    return false, result
  end
  
  -- Verify file was downloaded and has reasonable size
  local stat = vim.loop.fs_stat(dest_path)
  if not stat or stat.size < 100 then
    vim.fn.delete(dest_path)
    return false, "Downloaded file is too small or empty"
  end
  
  return true, nil
end

---Execute a shell command and return result
---Simple wrapper around vim.fn.system for consistency
---@param cmd table|string Command as array or string
---@return string output Command output
---@return number exit_code Exit code (0 = success)
function M.system(cmd)
  local output = vim.fn.system(cmd)
  return output, vim.v.shell_error
end

return M
