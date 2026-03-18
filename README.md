# Hermes

An ACP (Agent Client Protocol) Client implementation designed for integration with Neovim

## Overview

Hermes is a messaging layer for Neovim. It has no built-in UI, instead it provides APIs and hooks for building your own workflow while routing client-agent communication.

Hermes focuses on:
- APIs for making requests to AI Assistants (prompt, connect, authenticate, etc)
- Hooks into requests from AI assistants that require responses (permission requests, access requests, etc)
- Autocommands for updates on communication between the user (client) and assistant (agent) 

## Features

- [x] Full implementation of ACP Client
- [x] Configurable capabilities (filesystem, terminal, etc)
- [x] Trigger Autocommands for messages/notifications
- [ ] Lsp integration
- [ ] [Recursive language model](https://arxiv.org/abs/2512.24601) integration
- [ ] Speech to text for audio prompting (If no audio capability is present for the agent)

## API

Hermes exposes the following functions for sending requests to AI assistants. However, not all assistants support every method, and the level of support may vary by agent.

Methods marked “Optional” are implemented by Hermes but are not mandatory for agent implementations.

### Connect

This method allows you to connect to an agent, it takes the agent name and the protocol for the connection (defaults to `stdio`).

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

### Create Session

Create a new session. If no arguments are provided, the session defaults to either the project root or the current directory. 

```lua
local hermes = require("hermes")

-- use default session configuration
hermes.createSession()

-- customize connection configuration
hermes.createSession({
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
hermes.loadSession(sessionId)

-- call signature (with further configuration)
hermes.loadSession(sessionId, {
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

        hermes.loadSession(sessionId)
    end,
})
```

> **Triggers:** [SessionLoaded](#sessionloaded) autocommand upon completion

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
hermes.setMode(sessionId, modeId)

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

            hermes.setMode(sessionId, selectedModeId)
        end
    end,
})
```

> **Triggers:** [ModeUpdated](#modeupdated) autocommand upon completion.

### Respond

When an agent makes a request that requires user input (such as a permission request), it triggers an autocommand and pauses until the user responds. Use the `respond` method with the request ID to resume the agent's operation. If no autocommand handler is defined, a default workflow will be triggered. Requests can be disabled via the setup configuration. 

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
    callback = function(args)
        local terminalId = "your-generated-terminal-id" -- generate a unique id for terminal
        local requestId = args.data.requestId
        local comand = args.data.command
        local args = args.data.args
        local byte_limit = args.data.output_byte_limit

        -- lua combines args and command (add command to the beginning of args)
        table.insert(args, 1, command)

        terminals[terminalId] = vim.fn.startJob(args, {
            env = args.data.env,
            cwd = args.data.cwd,
        })

        hermes.respond(requestId, terminalId);
    end,
})
```

> **Responds to:** [TerminalCreate](#createterminal) autocommand.
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
    output = "erminal output text",
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
> - Do nothing if the user is managing the terminal (implemented the  autocommand)
> - Collect and send the terminal output if Hermes is handling the terminal

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
> - Wait for and report terminal exit details if Hermes is handling the terminal

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
> - Stop the process running in the terminal if Hermes is managing it

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
> - Remove the tracked terminal
> - Delete the attched buffer


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
      <td>⚡ <a href="#load-session-optional">setSessionConfigOption()</a></td>
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
    "loadSession": "boolean",
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
      <td>⚡ <a href="#set-mode-optional">setMode()</a></td>
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
      <td>⚡ <a href="#create-session">createSession()</a></td>
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
      <td>⚡ <a href="#load-session-optional">forkSession()</a></td>
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
      <td>⚡ <a href="#load-session-optional">loadSession()</a></td>
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
      <td>⚡ <a href="#load-session-optional">setSessionModel()</a></td>
      <td><pre><code class="language-json">{
}</code></pre></td>
    </tr>
    <tr id="sessionresumed">
      <td><code>SessionResumed</code></td>
      <td>Session resumed successfully</td>
      <td>⚡ <a href="#load-session-optional">resumeSession()</a></td>
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
      <td>⚡ <a href="#load-session-optional">listSessions()</a></td>
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
Hermes defaults to the global neovim log level, or to `INFO` if there is no global log level set.

Global log level example:
```lua
vim.opt.verbose = vim.log.levels.DEBUG;
```

 You can also use the neovim log levels to configure Hermes logging which will override the default behavior.

Example: 
```lua
require("hermes").setup({
    logLevel: vim.log.levels.DEBUG,
})
```

### Format

Logging defaults to pretty formatting, but you can change that format by setting a global variable in vim

```lua
vim.g.HERMES_LOG_FORMAT = "json"
```

Your options for log formats are:
- json
- pretty
- compact
- full

## TODO:

-- user requests
- [x] Allow connecting to Agents
  - [x] Via stdio
  - [ ] Via http
  - [ ] Via linux socket
- [x] initialize connections
- [x] handle authentication
- [x] Allow user to send prompts
  - [x] Send files
  - [x] Send text
  - [x] Send images 
  - [x] Send resource links
  - [x] Send audio
- [x] Allow mode selection
- [x] Allow cancel command to stop ai actions
- [ ] Handle sessions
  - [x] Create session
  - [x] Load session
  - [ ] List sessions
  - [ ] Merge sessions
  - [ ] Fork sessions
- [ ] Allow model selection

-- agent requests
- [x] Allow permission request
- [x] Allow agent to write to files
  - [x] Automatically refresh open buffers that have been modified
- [x] Allow agent to read files
- [x] Allow agent to use terminal
  - [x] Create autocommands for Agent progress in the terminal
- [x] Allow user to configure/turn off any/all aspects of ACP (if, for example, you just want to send data to the agent but still interact with it via the CLI)

-- infra
- [x] separate main thread logic from background threads
- [ ] use smol instead of tokio to reduce build size
- [ ] use async for all the things

-- nice to haves
- [ ] look into ways of improving ai integration
  - [ ] research RLM ([example](https://github.com/JaredStewart/coderlm))
  - [ ] connect agent to lsp (try to set it up as a tool call/connect to neovim lsp)
  - [ ] use [whisper.rs](https://crates.io/crates/whisper-rs) to facilitate speech to text
