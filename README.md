# Hermes

An [ACP (Agent Client Protocol)](https://agentclientprotocol.com) client designed for integration with Neovim. 

## Overview

Hermes is a messaging layer for Neovim. It has no built-in UI, instead it provides APIs and hooks for building your own workflow while routing client-agent communication.

Hermes focuses on:
- APIs for making requests to AI Assistants (prompt, connect, authenticate, etc)
- Hooks into requests from AI assistants that require responses (permission requests, access requests, etc)
- Autocommands for updates on communication between the user (client) and assistant (agent) 

## Installation

**lazy.nvim**
```lua
{
  "Ruddickmg/hermes.nvim",
  config = function()
    require("hermes").setup()
  end
}
```

**paq.nvim**
```lua

require("paq") {
  "Ruddickmg/hermes.nvim"
}
require("hermes").setup()
```


### Requirements

- Neovim 0.11 or later

### Pre-built Binary

Hermes is built in Rust and so must be integrated with lua during installation, pre built binaries are provided for convenience

#### Supported Platforms

Binaries are available for:

- **Linux:** x86_64, aarch64 (arm64)
- **macOS:** x86_64, arm64
- **Windows:** x86_64

> [!NOTE]
> Hermes will automatically detect and download a pre-built binary for supported platforms.
>
> It will:
> 1. Check if your platform is supported
> 2. Download the appropriate pre-built binary from GitHub releases
> 3. Load the binary
>
> This happens automatically on first API call.
>
> ```lua
> -- sets up pre-built binary for your system
> require("hermes").setup({ 
>   download = {
>     version = "latest",
>   }
> })
> ```
>
> You can also disable this if you would prefer to build from source
> ```lua
> -- Will have to set up manually with `:Hermes build` or build manually from source
> require("hermes").setup({ 
>   download = {
>     auto = false,
>   },
> })
> ```

### Building from Source (Unsupported Platforms)

If your platform is not in the supported list above, you can build from source:

**Requirements:**
- [Rust toolchain](https://rustup.rs/) (1.70 or later)

**Scripted**

Run the build command in Neovim:
  ```
  :Hermes build
  ```

  This will:
  - Compile the Rust code with `cargo build --release`
  - Install the resulting binary in the correct location

**Manual** 

```bash
git clone https://github.com/Ruddickmg/hermes.nvim.git
cd hermes.nvim
cargo build --release

# Copy target/release/libhermes.* to your Neovim data directory
```

## Commands

**Check status:**
```
:Hermes status
```
Shows loading state, configuration, and current status of Hermes.

**View logs:**
```
:Hermes log
```
Shows recent log messages and current state information.

**Check current version:**
```
:Hermes version
```
Shows installed version, platform info, and binary status.

**Update to latest:**
```
:Hermes update
```
Fetches the latest release from GitHub and replaces the current binary.

**Manual install:**
```
:Hermes install
```
Download the currently configured version.

**Clean installation:**
```
:Hermes clean
```
Removes the binary. Run `:Hermes install` or use Hermes API to re-download.

**Build from source:**
```
:Hermes build
```
Compiles from source (requires Rust toolchain). Runs asynchronously without blocking Neovim.

**Cancel build:**
```
:Hermes cancel
```
Cancels an in-progress source build. Shows warning if no build is running.

**View configuration:**
```
:Hermes setup
```
Shows current Hermes configuration settings.

## Features

- [x] Full implementation of ACP Client (Built on the official [Rust ACP Sdk](https://github.com/agentclientprotocol/rust-sdk))
- [x] Configurable capabilities (filesystem, terminal, etc)
- [x] Autocommands for messages/notifications

## API

Hermes exposes the following functions for sending requests to AI assistants.

> [!WARNING]
> Methods marked “Optional” are implemented by Hermes but are not mandatory for agent implementations.

### Setup

Configure Hermes plugin settings.

```lua
local hermes = require("hermes")

-- Basic usage with no arguments (uses all defaults)
hermes.setup()

-- Configure specific permissions
hermes.setup({
  permissions = {
    fs_write_access = true,
    terminal_access = true,
  }
})

-- Full configuration defaults
hermes.setup({
  download = {
    version = "latest", -- specify which hermes release to use
    auto = true, -- automatically download pre-built binary (set to false to build manually)
    timeout = 60, -- timeout in seconds for download
  },
  root_markers = { ".git" }, -- used to detect the project root by matching file names in the root directory
  permissions = {
    fs_write_access = true,      -- Allow file writes to the agent 
    fs_read_access = true,       -- Allow file reads to the agent 
    terminal_access = true,      -- Allow terminal access to the agent 
    request_permissions = true,  -- Allow agent to send permission requests 
    send_notifications = true,  -- Allow the agent to send notifications 
  },
  terminal = {
    delete = true,    -- Auto-delete terminals on exit
    enabled = true,    -- Enable terminal functionality
    buffered = true,   -- Buffer terminal output 
  },
  buffer = {
    auto_save = false,  -- Auto-save modified files after writing to them 
  },
  log = {
    -- send logs to stdio
    stdio = {
      -- only logs of the set value and above will be sent
      level = vim.log.levels.OFF or "off",
      -- logs  stdio logs will be formatted with the selected format 
      format = "compact",
    },
    -- send logs to Neovim "notify"
    notification = {
      level = vim.log.levels.ERROR or "error",
      format = "compact",
    },
    -- send logs to Neovim ":messages"
    message = {
      level = vim.log.levels.OFF or "off",
      format = "compact",
    },
    -- send logs to log files
    file = {
      level = vim.log.levels.OFF or "off",
      format = "json",
      path = vim.fn.stdpath('state') .. "/nvim/hermes/", -- path to log file(s)
      name = "hermes.log", -- name of log file
      max_size = 10485760, -- 10mb in bytes
      max_files = 5, -- Max log files to generate
    },
  },
})
```

> [!NOTE]
> - `setup()` does not have to be called, hermes will operate off of defaults without it
> - Configuration changes are applied immediately
> - Multiple `setup()` calls merge configurations - only specified fields are updated
> - All unspecified fields preserve their existing values

### Connect

This method allows you to connect to an agent, it takes the agent name and the protocol for the connection (defaults to `stdio`).

supported agents:
- opencode
- copilot
- gemini

```lua
local hermes = require("hermes")

-- connect to pre-defined agent
hermes.connect("copilot")

-- configure protocol
hermes.connect("opencode", {
  protocol = "http",
})

-- connect to custom agent (not pre-defined)
hermes.connect(
  "my-claude", -- this will be the key you use for other methods (disconnect for example) 
  {
    protocol = "socket", -- optional (Defaults to "stdio")
    command = "claude-acp",
    args = { "--socket", "/tmp/claude.sock" },
  }
)

-- connect to TCP socket
hermes.connect(
  "copilot",
  {
    protocol = "tcp",
    host = "localhost",
    port = 8080,
  }
)
```

> **Triggers:** [ConnectionInitialized](#connectioninitialized) autocommand upon completion.

### Disconnect

Below are examples of how you can disconnect from agent(s).

```lua
local hermes = require("hermes")

-- disconnect from a single agent
hermes.disconnect("copilot");

-- disconnect from a list of agents
hermes.disconnect({ "copilot", "opencode" })

-- disconnect from all agents
hermes.disconnect()
```

### Authenticate

Handle agent authentication.

```lua
local hermes = require("hermes")

-- function signature
hermes.authenticate(auth_method_id)

-- example
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "ConnectionInitialized",
  callback = function(args)
    local auth_method_id = table.remove(args.data.authMethods).id -- select auth method id somehow

    hermes.authenticate(auth_method_id)
  end,
})
```

> **Triggers:** [Authenticated](#authenticated) autocommand upon completion.

### Prompt

Send prompts to the agent 

There are five types of prompts you can send to an agent
 - [text](https://agentclientprotocol.com/protocol/content#text-content): Human readable prompts
 - [link](https://agentclientprotocol.com/protocol/content#resource-link): Links to resources (url, file path, etc)
 - [embedded](https://agentclientprotocol.com/protocol/content#embedded-resource): Similar to a link, but including the contents of the resource link (preferred over link if available) 
 - [image](https://agentclientprotocol.com/protocol/content#image-content): An image (encoded as a base64)
 - [audio](https://agentclientprotocol.com/protocol/content#audio-content): Audio content for communication (encoded as base64)

```lua
local hermes = require("hermes")
local sessionId = "current-session-id";

-- single prompt call signature
hermes.prompt(sessionId, {
  type = "text",
  text = "What time is it?"
})

-- multiple prompt call signature
hermes.prompt(sessionId, {
  {
  type = "text",
  text = "What time is it?"
  },
  {
  type = "link",
  name = "Example file",
  uri = "/path/to/example.txt"
  },
  { -- text
  type = "embedded",
  resource = {
    uri = "file:///home/user/script.py",
    mimeType = "text/x-python",
    text = "def hello():\n    print('Hello, world!')"
  }
  },
  { -- blob
  type = "embedded",
  resource = {
    uri = "file:///home/user/script.py",
    mimeType = "application/pdf",
    blob = "Base64-encoded binary data"
  }
  },
  {
  type = "image",
  data = "base64-encoded-image-data",
  mimeType = "image/png"
  },
  {
  type = "audio",
  data = "base64-encoded-audio-data",
  mimeType = "audio/wav"
  }
}

-- example
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "SessionCreated",
  callback = function(args)
    local sessionId = args.data.sessionId

    hermes.prompt(sessionId, {
      type = "text",
      text = "What time is it?"
    })
  end,
})
```

> **Triggers:** [Prompted](#prompted) autocommand upon completion.

### Create Session

Create a new session. If no arguments are provided, the session defaults to either the project root or the current directory. 

```lua
local hermes = require("hermes")

-- use default session configuration
hermes.create_session()

-- customize connection configuration
hermes.create_session({
  cwd = ".", -- path to create the session in (optional)
  mcpServers = {
  { -- Http or Sse MCP server definition
    type = "http", -- or "sse"
    name = "Human readable name for MCP server",
    url = "http://url-to-mcp-server.com",
    headers = {
      { ["Content-Type"] = "application/json" },
      { headerName = "header value" },
    },
  },
  {  -- Stdio MCP server definition
    type = "stdio",
    name = "Human readable name for MCP server",
    command = "/path/to/the/MCP/server/executable",
    args = { "run", "--flag", "something" },
    -- Environment variables to set when launching the MCP server.
    env = {
      { name = "ENVIRONMENT_VAR_NAME", value = "value" },
    },
  },
  },
})
```

> **Triggers:** [SessionCreated](#sessioncreated) autocommand upon completion.

### Load Session (**Optional**)

Load an existing session

```lua
local hermes = require("hermes")

-- call signature (uses defaults)
hermes.load_session(sessionId)

-- call signature (with further configuration)
hermes.load_session(sessionId, {
  cwd = ".", -- path to load the session from (optional, defaults to either project root or current directory)
  mcpServers = {
    { -- Http or Sse MCP server definition
      type = "http", -- or "sse"
      name = "Human readable name for MCP server",
      url = "http://url-to-mcp-server.com",
      headers = {
        { ["Content-Type"] = "application/json" },
        { headerName = "header value" },
      },
    },
    {  -- Stdio MCP server definition
      type = "stdio",
      name = "Human readable name for MCP server",
      command = "/path/to/the/MCP/server/executable",
      args = { "run", "--flag", "something" },
      -- Environment variables to set when launching the MCP server.
      env = {
        { name = "ENVIRONMENT_VAR_NAME", value = "value" },
      },
    },
  },
})

-- example
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "SessionCreated",
  callback = function(args)
    local sessionId = args.data.sessionId

    hermes.load_session(sessionId)
  end,
})
```

> **Triggers:** [SessionLoaded](#sessionloaded) autocommand upon completion

### List Sessions (**Optional**)

List sessions, can be filtered by project path or cursor pagination.

```lua
local hermes = require("hermes")

-- list all sessions
hermes.list_sessions()

-- filter by directory
hermes.list_sessions({
  cwd = "/path/to/directory",
})

-- filter by cursor based pagination
hermes.list_sessions({
  cursor = "abc123",
})

-- example
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "SessionsListed",
  callback = function(args)
    local next_page = args.data.nextCursor

    -- get next page of sessions with this cursor in the current directory
    hermes.list_sessions({
      cwd = vim.fn.getcwd(),
      cursor = next_page,
    })
  end,
})
```

> **Triggers:** [SessionsListed](#sessionslisted) autocommand upon completion

### Cancel (**Optional**)

Cancel the current operation of the agent (e.g., stop generating text, stop a tool call in progress, etc)

```lua
local hermes = require("hermes")
local sessionId = 'session-id-from-create-session-response'

-- call signature
hermes.cancel(sessionId)

-- example
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "SessionCreated",
  callback = function(args)
    local sessionId = args.data.sessionId

    hermes.cancel(sessionId)
    
  end,
})
```

### Set mode (**Optional**)

Set what mode the agent is in (the plan/build modes for opencode for example)

```lua
local hermes = require("hermes")

-- call signature
hermes.set_mode(sessionId, modeId)

-- example
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "SessionCreated",
  callback = function(args)
    local modes = args.data.modes
    -- modes is optional for an agent, some may not have different modes to select
    if modes ~= nil then
      local selectedModeId = table.remove(modes.availableModes).id -- select mode id somehow
      local sessionId = args.data.sessionId

      hermes.set_mode(sessionId, selectedModeId)
    end
  end,
})
```

> **Triggers:** [ModeUpdated](#modeupdated) autocommand upon completion.

### Respond

When an agent makes a request that requires user input (such as a permission request), it triggers an autocommand and pauses until the user responds. Use the `respond` method with the request ID to resume the agent's operation. If no autocommand handler is defined, a default workflow will be triggered. Requests can be disabled via the setup configuration. 

> [!WARNING]
> While Hermes is a complete ACP client, most agents available today don't fully utilize the protocol. The following autocommands are [optional features](https://agentclientprotocol.com/protocol/overview#optional-methods-2) and often handled through agent-specific tools rather than calling the ACP methods that trigger them. This means some Hermes capabilities may not be exercised depending on which agent you use.

#### Permission request

```lua
local hermes = require("hermes")

-- call signature
hermes.respond("requestId", "optionId")

-- example: 
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "PermissionRequest",
  callback = function(args)
    local selectedOptionId = table.remove(args.data.options).optionId -- select id somehow
    local requestId = args.data.requestId

    hermes.respond(requestId, selectedOptionId)
    
  end,
})
```

> **Responds to:** [PermissionRequest](#permissionrequest) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for `PermissionRequest`, Hermes will use the native Neovim select menu to gather a response from the user.


#### Write to file

```lua
local hermes = require("hermes")

-- call signature
hermes.respond("requestId")

-- example: 
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "WriteTextFile",
  callback = function(args)
    local requestId = args.data.requestId

    -- writing to a file doesn't take any data, but a notification is required when it is finished
    hermes.respond(requestId)
  end,
})
```

> **Responds to:** [WriteTextFile](#writetextfile) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for `WriteTextFile`, Hermes will:
> - Update buffers if they are open and mark them as modified (will not automatically save)
> - Refresh the view of any modified buffers
> - Write to the file on disk if it is not open in a buffer


#### Read from file

```lua
local hermes = require("hermes")

-- call signature
hermes.respond("requestId", "Hello World!")

-- example: 
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "ReadTextFile",
  callback = function(args)
    local requestId = args.data.requestId
    local filename = args.data.path
    local _start = args.data.line -- optional, may not be provided by the agent
    local _end = args.data.limit -- optional, may not be provided by the agent 
    local file = io.open(filename, "r")
    local content = file:read("*all")
    file:close()

    hermes.respond(requestId, content)
  end,
})
```

> **Responds to:** [ReadTextFile](#readtextfile) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for `ReadTextFile`, Hermes will:
> - Read the file from disk if one is not open in a buffer
> - Read the current state of the open buffer if the target file is open
> - Start at the `line` number if defined
> - End at the `limit` number if defined


#### Create Terminal for agent communication 

```lua
local hermes = require("hermes")
local terminals = {}

-- call signature
hermes.respond("requestId", "terminalId")

-- example:     
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "TerminalCreate",
  callback = function(event)
    local terminalId = "your-generated-terminal-id" -- generate a unique id for terminal
    local requestId = event.data.requestId
    local command = event.data.command
    local term_args = event.data.args or {}
    local byte_limit = event.data.output_byte_limit

    -- lua combines args and command (add command to the beginning of args)
    table.insert(term_args, 1, command)

    terminals[terminalId] = vim.fn.jobstart(term_args, {
      env = event.data.env,
      cwd = event.data.cwd,
    })

    hermes.respond(requestId, terminalId);
  end,
})
```

> **Responds to:** [TerminalCreate](#terminalcreate) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for `TerminalCreate`, Hermes will:
> - Create a terminal attached to a buffer (hidden by default)
> - Handle byte limit constraints if defined

> [!WARNING]
> If no `TerminalCreate` autocommand is registered, Hermes will use default functionality to manage **all** subsequent terminal interaction.

#### Provide terminal output to the assistant 

```lua
local hermes = require("hermes")
local terminals = {}
local is_truncated = true;

-- call signature (truncated defaults to false)
hermes.respond("requestId", "terminal output text")

-- call signature with truncation defined
hermes.respond("requestId", {
  output = "terminal output text",
  truncated = is_truncated,
})

-- example:
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "TerminalOutput",
  callback = function(args)
    local requestId = args.data.requestId
    local terminalId = args.data.terminalId
    local terminalOutput = terminals[terminalId].output -- get output somehow

    hermes.respond(requestId, terminalOutput);
  end,
})
```

> **Responds to:** [TerminalOutput](#terminaloutput) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for [TerminalCreate](#terminalcreate), Hermes will:
> - Collect and send the terminal output to the agent

#### Reporting terminal exit

```lua
local hermes = require("hermes")
local terminals = {}

-- call signature for termination signal
hermes.respond("requestId", "SIGTERM")


-- call signature for exit code
hermes.respond("requestId", 0)

-- call signature with exit code and termination signal
hermes.respond("requestId", {
  exitCode = 9,
  signal = "SIGKILL"
})

-- example:
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "TerminalExit",
  callback = function(args)
    local requestId = args.data.requestId
    local terminalId = args.data.terminalId

    hermes.respond(requestId, {
      exitCode = terminals[terminalId].exitCode, -- get output somehow
      signal = terminals[terminalId].signal, -- get output somehow
    });
  end,
})
```

> **Responds to:** [TerminalExit](#terminalexit) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for [TerminalCreate](#terminalcreate), Hermes will:
> - Wait for and report terminal exit details

#### Kill terminal process

```lua
local hermes = require("hermes")
local terminals = {}

-- call signature with exit code and termination signal
hermes.respond("requestId")

-- example:
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "TerminalKill",
  callback = function(args)
    local requestId = args.data.requestId

    hermes.respond(requestId);
  end,
})
```

> **Responds to:** [TerminalKill](#terminalkill) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for [TerminalCreate](#terminalcreate), Hermes will:
> - Stop the process running in the terminal

#### Release terminal process

```lua
local hermes = require("hermes")
local terminals = {}

-- call signature with exit code and termination signal
hermes.respond("requestId")

-- example:
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "TerminalRelease",
  callback = function(args)
    local requestId = args.data.requestId

    hermes.respond(requestId);
  end,
})
```

> **Responds to:** [TerminalRelease](#terminalrelease) autocommand.
>
> **Default behavior:** If no autocommand handler is defined for [TerminalCreate](#terminalcreate), Hermes will:
> - Stop any process running in the terminal
> - Remove the terminal
> - Delete the attached buffer (can be configured to omit this step)

## Autocommands

Hermes generates autocommands for all communication between agent and client. Here's an example of hooking into one:

```lua
vim.api.nvim_create_autocmd("User", {
  group = "hermes",
  pattern = "AgentTextMessage",
  callback = function(args)
    print("Received some text from our assistant: " .. args.data.text)
  end,
})
```

Below is a list of all autocommands and their associated data (passed to the callback in the `args.data` field). Hermes will only trigger autocommands if there is a listener defined for it (I.E. You have created one like the example above)

<table>
  <thead>
    <tr>
      <th>Autocommand</th>
      <th>Description</th>
      <th>Source</th>
      <th>Schema</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>AgentImageMessage</code></td>
      <td>An image from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentImageThought</code></td>
      <td>Visual reasoning/thought from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceLinkMessage</code></td>
      <td>A resource link from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "name": "string",
  "uri": "string",
  "description": "string (optional)",
  "mimeType": "string (optional)",
  "size": "number (optional)",
  "title": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceLinkThought</code></td>
      <td>Resource link thought from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "name": "string",
  "uri": "string",
  "description": "string (optional)",
  "mimeType": "string (optional)",
  "size": "number (optional)",
  "title": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceMessage</code></td>
      <td>A resource from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "resource": {
    "text": "string (if text resource)",
    "blob": "string (if blob resource)",
    "uri": "string",
    "mimeType": "string (optional)"
  },
  "annotations": { "audience": [], "lastModified": "string" }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceThought</code></td>
      <td>Resource-based thought from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "resource": {
    "text": "string (if text resource)",
    "blob": "string (if blob resource)",
    "uri": "string",
    "mimeType": "string (optional)"
  },
  "annotations": { "audience": [], "lastModified": "string" }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentTextMessage</code></td>
      <td>A text message from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentTextThought</code></td>
      <td>Textual thought/reasoning from the agent</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr id="authenticated">
      <td><code>Authenticated</code></td>
      <td>Authentication completed</td>
      <td>⚡ <a href="#authenticate">authenticate()</a></td>
      <td><pre><code class="language-json">{
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AvailableCommands</code></td>
      <td>Available commands are updated</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "commands": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)"
    }
  ]
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ConfigurationOption</code></td>
      <td>Configuration option updates</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "options": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)",
      "category": "string (optional)",
      "kind": {
        "currentValue": "string",
        "options": [
          { "type": "ungrouped", "value": "string", "name": "string", "description": "string (optional)" },
          {
            "type": "grouped",
            "group": "string",
            "name": "string",
            "options": [
              { "value": "string", "name": "string", "description": "string (optional)" }
            ]
          }
        ]
      }
    }
  ]
}</code></pre></td>
    </tr>
    <tr id="configurationupdated">
      <td><code>ConfigurationUpdated</code></td>
      <td>Session configuration updated</td>
      <td>⚡ <a href="#load-session-optional">set_session_config_option()</a></td>
      <td><pre><code class="language-json">{
  "configOptions": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)",
      "category": "string (optional)",
      "kind": {
        "currentValue": "string",
        "options": [
          { "type": "ungrouped", "value": "string", "name": "string", "description": "string (optional)" },
          {
            "type": "grouped",
            "group": "string",
            "name": "string",
            "options": [
              { "value": "string", "name": "string", "description": "string (optional)" }
            ]
          }
        ]
      }
    }
  ]
}</code></pre></td>
    </tr>
    <tr id="connectioninitialized">
      <td><code>ConnectionInitialized</code></td>
      <td>Connection established with agent</td>
      <td>⚡ <a href="#connect">connect()</a></td>
      <td><pre><code class="language-json">{
  "protocolVersion": "string",
  "agentCapabilities": {
    "load_session": "boolean",
    "promptCapabilities": {
      "image": "boolean",
      "audio": "boolean",
      "embeddedContext": "boolean"
    },
    "mcpCapabilities": {
      "http": "boolean",
      "sse": "boolean"
    },
    "sessionCapabilities": {
      "list": "boolean",
      "fork": "boolean",
      "resume": "boolean"
    }
  },
  "authMethods": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)"
    }
  ],
  "agentInfo": {
    "name": "string",
    "version": "string",
    "title": "string (optional)"
  }
}</code></pre></td>
    </tr>
    <tr id="modecurrent">
      <td><code>ModeCurrent</code></td>
      <td>Current mode changes</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "id": "string"
}</code></pre></td>
    </tr>
    <tr id="modeupdated">
      <td><code>ModeUpdated</code></td>
      <td>Session mode changed</td>
      <td>⚡ <a href="#set-mode-optional">set_mode()</a></td>
      <td><pre><code class="language-json">{
}</code></pre></td>
    </tr>
    <tr id="permissionrequest">
      <td><code>PermissionRequest</code></td>
      <td>Agent requests permission to execute a tool</td>
      <td>🤖 Agent (requires -> <a href="#permission-request">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "toolCall": {
    "toolCallId": "string",
    "fields": {
      "kind": "Read | Edit | Delete | Move | Search | Execute | Think | Fetch | SwitchMode | Other (optional)",
      "status": "Pending | InProgress | Completed | Cancelled | Error (optional)",
      "title": "string (optional)",
      "content": [{
        "type": "text | image | resource | resourcelink | terminal | diff",
        "text": "string (if text type)",
        "data": "base64 string (if image type)",
        "mimeType": "string (if image type)",
        "uri": "string (if image/resource/resourcelink type)",
        "resource": {
          "text": "string (if text resource)",
          "blob": "string (if blob resource)",
          "uri": "string",
          "mimeType": "string (optional)"
        },
        "name": "string (if resourcelink type)",
        "description": "string (optional, if resourcelink type)",
        "terminalId": "string (if terminal type)",
        "path": "string (if diff type)",
        "newText": "string (if diff type)",
        "oldText": "string (optional, if diff type)"
      }],
      "locations": [{ "path": "string", "line": "number (optional)" }],
      "rawInput": "JSON value (optional)",
      "rawOutput": "JSON value (optional)"
    }
  },
  "options": [{ "id": "string", "label": "string", "description": "string (optional)" }]
}</code></pre></td>
    </tr>
    <tr>
      <td><code>Plan</code></td>
      <td>Agent generates a plan</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "entries": [
    { "content": "string", "priority": "High | Medium | Low" }
  ]
}</code></pre></td>
    </tr>
    <tr id="prompted">
      <td><code>Prompted</code></td>
      <td>Agent response received</td>
      <td>⚡ <a href="#prompt">prompt()</a></td>
      <td><pre><code class="language-json">{
  "stopReason": "string (e.g., 'Stop', 'Cancelled', 'Error')"
}</code></pre></td>
    </tr>
    <tr id="readtextfile">
      <td><code>ReadTextFile</code></td>
      <td>Agent requests to read a text file</td>
      <td>🤖 Agent (requires -> <a href="#read-from-file">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "path": "string",
  "line": "number (optional, 1-based)",
  "limit": "number (optional, max lines to read)"
}</code></pre></td>
    </tr>
    <tr id="sessioncreated">
      <td><code>SessionCreated</code></td>
      <td>New session created</td>
      <td>⚡ <a href="#create-session">create_session()</a></td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "modes": {
    "currentModeId": "string",
    "availableModes": [
      {
        "id": "string",
        "name": "string",
        "description": "string (optional)"
      }
    ]
  },
  "configOptions": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)",
      "category": "string (optional)",
      "kind": {
        "currentValue": "string",
        "options": [
          { "type": "ungrouped", "value": "string", "name": "string", "description": "string (optional)" },
          {
            "type": "grouped",
            "group": "string",
            "name": "string",
            "options": [
              { "value": "string", "name": "string", "description": "string (optional)" }
            ]
          }
        ]
      }
    }
  ]
}</code></pre></td>
    </tr>
    <tr id="sessionforked">
      <td><code>SessionForked</code></td>
      <td>Session forked successfully</td>
      <td>⚡ <a href="#load-session-optional">fork_session()</a></td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "modes": {
    "currentModeId": "string",
    "availableModes": [
      {
        "id": "string",
        "name": "string",
        "description": "string (optional)"
      }
    ]
  },
  "configOptions": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)",
      "category": "string (optional)",
      "kind": {
        "currentValue": "string",
        "options": [
          { "type": "ungrouped", "value": "string", "name": "string", "description": "string (optional)" },
          {
            "type": "grouped",
            "group": "string",
            "name": "string",
            "options": [
              { "value": "string", "name": "string", "description": "string (optional)" }
            ]
          }
        ]
      }
    }
  ]
}</code></pre></td>
    </tr>
    <tr id="sessionloaded">
      <td><code>SessionLoaded</code></td>
      <td>Session loaded successfully</td>
      <td>⚡ <a href="#load-session-optional">load_session()</a></td>
      <td><pre><code class="language-json">{
  "modes": {
    "currentModeId": "string",
    "availableModes": [
      {
        "id": "string",
        "name": "string",
        "description": "string (optional)"
      }
    ]
  },
  "configOptions": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)",
      "category": "string (optional)",
      "kind": {
        "currentValue": "string",
        "options": [
          { "type": "ungrouped", "value": "string", "name": "string", "description": "string (optional)" },
          {
            "type": "grouped",
            "group": "string",
            "name": "string",
            "options": [
              { "value": "string", "name": "string", "description": "string (optional)" }
            ]
          }
        ]
      }
    }
  ]
}</code></pre></td>
    </tr>
    <tr id="sessionmodelupdated">
      <td><code>SessionModelUpdated</code></td>
      <td>Session model updated</td>
      <td>⚡ <a href="#load-session-optional">set_session_model()</a></td>
      <td><pre><code class="language-json">{
}</code></pre></td>
    </tr>
    <tr id="sessionresumed">
      <td><code>SessionResumed</code></td>
      <td>Session resumed successfully</td>
      <td>⚡ <a href="#load-session-optional">resume_session()</a></td>
      <td><pre><code class="language-json">{
  "modes": {
    "currentModeId": "string",
    "availableModes": [
      {
        "id": "string",
        "name": "string",
        "description": "string (optional)"
      }
    ]
  },
  "configOptions": [
    {
      "id": "string",
      "name": "string",
      "description": "string (optional)",
      "category": "string (optional)",
      "kind": {
        "currentValue": "string",
        "options": [
          { "type": "ungrouped", "value": "string", "name": "string", "description": "string (optional)" },
          {
            "type": "grouped",
            "group": "string",
            "name": "string",
            "options": [
              { "value": "string", "name": "string", "description": "string (optional)" }
            ]
          }
        ]
      }
    }
  ]
}</code></pre></td>
    </tr>
    <tr id="sessionslisted">
      <td><code>SessionsListed</code></td>
      <td>Session list received</td>
      <td>⚡ <a href="#load-session-optional">list_sessions()</a></td>
      <td><pre><code class="language-json">{
  "sessions": [
    {
      "sessionId": "string",
      "cwd": "string",
      "title": "string (optional)",
      "updatedAt": "string (optional)"
    }
  ],
  "nextCursor": "string (optional)"
}</code></pre></td>
    </tr>
    <tr id="terminalcreate">
      <td><code>TerminalCreate</code></td>
      <td>Agent requests to create a terminal for command execution</td>
      <td>🤖 Agent (requires -> <a href="#create-terminal-for-agent-communication">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "command": "string",
  "args": ["string"],
  "env": [{"name": "string", "value": "string"}],
  "cwd": "string (optional)",
  "outputByteLimit": "number (optional)"
}</code></pre></td>
    </tr>
    <tr id="terminalexit">
      <td><code>TerminalExit</code></td>
      <td>Agent requests notification when terminal process exits</td>
      <td>🤖 Agent (requires -> <a href="#reporting-terminal-exit">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "terminalId": "string"
}</code></pre></td>
    </tr>
    <tr id="terminalkill">
      <td><code>TerminalKill</code></td>
      <td>Agent requests to kill a terminal process</td>
      <td>🤖 Agent (requires -> <a href="#kill-terminal-process">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "terminalId": "string",
  "signal": "string (optional, e.g., 'SIGTERM', 'SIGKILL')"
}</code></pre></td>
    </tr>
    <tr id="terminaloutput">
      <td><code>TerminalOutput</code></td>
      <td>Agent requests terminal output</td>
      <td>🤖 Agent (requires -> <a href="#provide-terminal-output-to-the-assistant">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "terminalId": "string",
  "byteLimit": "number (optional)"
}</code></pre></td>
    </tr>
    <tr id="terminalrelease">
      <td><code>TerminalRelease</code></td>
      <td>Agent requests to release a terminal</td>
      <td>🤖 Agent (requires -> <a href="#release-terminal-process">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "terminalId": "string"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ToolCall</code></td>
      <td>Agent makes a tool call</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "id": "string",
  "title": "string",
  "kind": "Read | Edit | Delete | Move | Search | Execute | Think | Fetch | SwitchMode | Other",
  "status": "Pending | InProgress | Completed | Cancelled | Error",
  "content": [
    {
      "type": "text | image | resource | resourcelink | terminal | diff",
      "text": "string (if text type)",
      "data": "base64 string (if image type)",
      "mimeType": "string (if image type)",
      "uri": "string (if image/resource/resourcelink type)",
      "resource": {
        "text": "string (if text resource)",
        "blob": "string (if blob resource)",
        "uri": "string",
        "mimeType": "string (optional)"
      },
      "name": "string (if resourcelink type)",
      "description": "string (optional, if resourcelink type)",
      "terminalId": "string (if terminal type)",
      "path": "string (if diff type)",
      "newText": "string (if diff type)",
      "oldText": "string (optional, if diff type)"
    }
  ],
  "locations": [
    {
      "path": "string",
      "line": "number (optional)"
    }
  ],
  "rawInput": "JSON value (optional)",
  "rawOutput": "JSON value (optional)"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ToolCallUpdate</code></td>
      <td>Tool call is updated (e.g., progress, output)</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "toolCallId": "string",
  "fields": {
    "kind": "Read | Edit | Delete | Move | Search | Execute | Think | Fetch | SwitchMode | Other (optional)",
    "status": "Pending | InProgress | Completed | Cancelled | Error (optional)",
    "title": "string (optional)",
    "content": [
      {
        "type": "text | image | resource | resourcelink | terminal | diff",
        "text": "string (if text type)",
        "data": "base64 string (if image type)",
        "mimeType": "string (if image type)",
        "uri": "string (if image/resource/resourcelink type)",
        "resource": {
          "text": "string (if text resource)",
          "blob": "string (if blob resource)",
          "uri": "string",
          "mimeType": "string (optional)"
        },
        "name": "string (if resourcelink type)",
        "description": "string (optional, if resourcelink type)",
        "terminalId": "string (if terminal type)",
        "path": "string (if diff type)",
        "newText": "string (if diff type)",
        "oldText": "string (optional, if diff type)"
      }
    ],
    "locations": [
      {
        "path": "string",
        "line": "number (optional)"
      }
    ],
    "rawInput": "JSON value (optional)",
    "rawOutput": "JSON value (optional)"
  }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>UsageUpdate</code></td>
      <td>Session usage metrics update (tokens, cost)</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "used": "number (tokens used)",
  "size": "number (max context size)",
  "cost": {
    "amount": "number",
    "currency": "string (e.g., 'USD')"
  }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>UserImageMessage</code></td>
      <td>An image sent from the client</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>UserResourceLinkMessage</code></td>
      <td>A resource link from the client</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "name": "string",
  "uri": "string",
  "description": "string (optional)",
  "mimeType": "string (optional)",
  "size": "number (optional)",
  "title": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>UserResourceMessage</code></td>
      <td>A resource sent from the client</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "resource": {
    "text": "string (if text resource)",
    "blob": "string (if blob resource)",
    "uri": "string",
    "mimeType": "string (optional)"
  },
  "annotations": { "audience": [], "lastModified": "string" }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>UserTextMessage</code></td>
      <td>Message text sent from the client</td>
      <td>🤖 Agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": {
    "audience": ["Role1", "Role2"],
    "lastModified": "ISO8601 string",
    "priority": "number"
  }
}</code></pre></td>
    </tr>
    <tr id="writetextfile">
      <td><code>WriteTextFile</code></td>
      <td>Agent requests to write to a text file</td>
      <td>🤖 Agent (requires -> <a href="#write-to-file">respond()</a>)</td>
      <td><pre><code class="language-json">{
  "requestId": "uuid string",
  "sessionId": "string",
  "path": "string",
  "content": "string"
}</code></pre></td>
    </tr>
  </tbody>
</table>


## Logging

### Level
Hermes defaults to `INFO` log level until configured via `setup()`.

Configure log levels and formats via the `setup()` function:
```lua
require("hermes").setup({
  log = {
    notification = { level = vim.log.levels.ERROR }, -- Per-target config
    message = { 
      level = vim.log.levels.INFO,
      format = "pretty"  -- Each target has its own format
    },
  }
})
```

### Format

Log formats can be configured per-target via `setup()`. Each target has its own format setting that defaults to "compact" if not specified:

```lua
require("hermes").setup({
  log = {
    -- Each target has its own format (defaults to "compact" if not set):
    notification = { format = "pretty" },
    message = { format = "json" },
    quickfix = { format = nil },  -- nil = use default ("compact")
  }
})
```

Available formats:
- **pretty** - Human-readable with colors and formatting
- **compact** - Condensed single-line format (default)
- **full** - Complete information including timestamps and metadata
- **json** - Machine-readable JSON format

## TODO:

-- functionality
- [x] Allow connecting to Agents
  - [x] Via stdio
  - [ ] Via http
  - [x] Via tcp socket
  - [ ] Via unix socket
- [ ] Add autocommand that triggers on all events
- [ ] Support "unstable" ACP methods
  - [ ] model selection
  - [ ] session methods
    - [ ] Merge sessions
    - [ ] Fork sessions

-- nice to haves
- [ ] Status bar integration
  - [ ] Configurable
  - [ ] Report mode
  - [ ] Report model
  - [ ] Report status
    - [ ] waiting on user response
    - [ ] thinking
    - [ ] Finished/Responded
    - [ ] etc?
  - [ ] Update on events (no polling required)
- [ ] quickfix list integration
  - [ ] add files updated by agent to quickfix list 
  - [ ] add references made by agent to quickfix list
- [ ] Integrate with signs
  - [ ] Configurable
  - [ ] Show lines edited by agent
- [ ] look into ways of improving ai integration
  - [ ] research RLM ([example](https://github.com/JaredStewart/coderlm))
  - [ ] connect agent to lsp (try to set it up as a tool call/connect to neovim lsp)
  - [ ] use [whisper.rs](https://crates.io/crates/whisper-rs) to facilitate speech to text
