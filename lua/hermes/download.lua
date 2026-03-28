---HTTP download utilities
---@module hermes.download
---Provides a clean wrapper around HTTP download functionality with cross-platform support
---Uses curl (Unix), wget (Unix fallback), or PowerShell (Windows)

local M = {}

local USER_AGENT = "hermes.nvim/0.1"
M.USER_AGENT = USER_AGENT

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

---Check if PowerShell is available (Windows)
---@return boolean available
function M.is_powershell_available()
  return vim.fn.executable("powershell") == 1
end

---Get available download tool
---Priority: curl (Unix) > wget (Unix) > PowerShell (Windows)
---@return string|nil tool_name Name of available tool, or nil if none
function M.get_available_tool()
  if M.is_curl_available() then
    return "curl"
  elseif M.is_wget_available() then
    return "wget"
  elseif M.is_powershell_available() then
    return "powershell"
  end
  return nil
end

---Download file from URL using available tool
---Cross-platform: curl (Unix), wget (Unix fallback), PowerShell (Windows)
---@param url string URL to download
---@param dest_path string Destination path
---@return boolean success Whether download succeeded
---@return table|nil error Error info table if failed, containing:
---   - message: Human readable error message
---   - url: URL that was attempted
---   - http_code: HTTP status code (if available)
---   - tool: Which download tool was used
---   - exit_code: Shell exit code
---   - stderr: Raw error output from tool
function M.download(url, dest_path)
  local tool = M.get_available_tool()
  
  if not tool then
    return false, {
      message = "No download tool available (tried curl, wget, PowerShell). Please install curl or wget.",
      url = url,
      http_code = nil,
      tool = nil,
      exit_code = nil,
      stderr = nil,
    }
  end
  
  local cmd
  local http_code = nil
  
  if tool == "curl" then
    cmd = { "curl", "-sL", "-H", "User-Agent: " .. USER_AGENT, "-o", dest_path, url }
  elseif tool == "wget" then
    cmd = { "wget", "-q", "--user-agent=" .. USER_AGENT, "-O", dest_path, url }
  else
    -- PowerShell for Windows
    local ps_cmd = string.format(
      'Invoke-WebRequest -Uri "%s" -OutFile "%s" -UseBasicParsing -UserAgent "%s"',
      url, dest_path, USER_AGENT
    )
    cmd = { "powershell", "-Command", ps_cmd }
  end
  
  local result = vim.fn.system(cmd)
  local exit_code = vim.v.shell_error
  
  -- For curl, extract HTTP code from the end of output (since we used -w %{http_code})
  if tool == "curl" and result then
    -- The HTTP code is appended to stdout after the file is written
    http_code = result:match("(%d%d%d)$")
    if http_code then
      http_code = tonumber(http_code)
    end
  end
  
  if exit_code ~= 0 then
    -- Check if it's a command not found error vs network error
    local error_msg = result
    if result:match("command not found") or result:match("not installed") or result:match("is not recognized") then
      error_msg = tool .. " appears to be installed but execution failed: " .. result
    end
    
    return false, {
      message = error_msg,
      url = url,
      http_code = http_code,
      tool = tool,
      exit_code = exit_code,
      stderr = result,
    }
  end
  
  -- Verify file was downloaded and has reasonable size using vim.uv for cross-platform compatibility
  local uv = vim.uv or vim.loop
  local stat = uv.fs_stat(dest_path)
  if not stat or stat.size < 100 then
    -- Use vim.uv.fs_unlink for cross-platform file deletion
    uv.fs_unlink(dest_path)
    return false, {
      message = "Downloaded file is too small or empty",
      url = url,
      http_code = http_code or 200,
      tool = tool,
      exit_code = 0,
      stderr = "File size: " .. (stat and stat.size or 0) .. " bytes",
    }
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
