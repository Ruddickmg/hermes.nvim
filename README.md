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
- [ ] Speech to text for audio prompting (If no audio capability is present for the agent)
- [ ] Lsp integration
- [ ] [Recursive language model](https://arxiv.org/abs/2512.24601) integration

## API

Hermes exposes the following functions for sending requests to AI assistants. However, not all assistants support every method, and the level of support may vary by agent.

Methods marked “Optional” are implemented by Hermes but are not mandatory for agent implementations.

### Connect

This method allows you to connect to an agent, it takes the agent name and the protocol for the connection (defaults to `stdio`).

Options for protocol (currently supported)
- stdio (Default)

Planned future protocols (not yet supported)
- http
- socket

Options for agent (pre-defined)
- copilot (GitHub Copilot)
- opencode

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
local auth_method_id = "example-auth-method-id"

hermes.authenticate(auth_method_id)
```

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
    pattern = "CreatedSession",
    callback = function(args)
        local sessionId = args.data.sessionId

        hermes.loadSession(sessionId)
    end,
})
```

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
    pattern = "CreatedSession",
    callback = function(args)
        local sessionId = args.data.sessionId

        hermes.prompt(sessionId, {
          type = "text",
          text = "What time is it?"
        })
    end,
})
```

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
    pattern = "CreatedSession",
    callback = function(args)
        local sessionId = args.data.sessionId

        hermes.cancel(sessionId)
        
    end,
})

```

### Cancel

Cancel the current operation of the agent (e.g., stop generating text, stop a tool call in progress, etc)

```lua
local hermes = require("hermes")
local sessionId = 'session-id-from-create-session-response'

-- call signature
hermes.cancel(sessionId)

-- example
vim.api.nvim_create_autocmd("User", {
    group = "hermes",
    pattern = "CreatedSession",
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
    pattern = "CreatedSession",
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

### Respond

Respond to agent requests

#### Permission request responses

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

### Respond

Respond to agent requests

#### Permission request responses

```lua
local hermes = require("hermes")

-- call signature
hermes.respond("requestId", "optionId")

-- example: 
vim.api.nvim_create_autocmd("User", {
    group = "hermes",
    pattern = "PermissionRequested",
    callback = function(args)
        local selectedOptionId = table.remove(args.data.options).optionId -- select id somehow
        local requestId = args.data.requestId

        hermes.respond(requestId, selectedOptionId)
        
    end,
})
```

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
      <th>Message Schema (args.data)</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>UserTextMessage</code></td>
      <td>Message text sent from the client</td>
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
    <tr>
      <td><code>UserImageMessage</code></td>
      <td>An image sent from the client</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>UserResourceMessage</code></td>
      <td>A resource sent from the client</td>
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
      <td><code>UserResourceLinkMessage</code></td>
      <td>A resource link from the client</td>
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
      <td><code>AgentTextMessage</code></td>
      <td>A text message from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentImageMessage</code></td>
      <td>An image from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceMessage</code></td>
      <td>A resource from the agent</td>
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
      <td><code>AgentResourceLinkMessage</code></td>
      <td>A resource link from the agent</td>
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
      <td><code>AgentTextThought</code></td>
      <td>Textual thought/reasoning from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentImageThought</code></td>
      <td>Visual reasoning/thought from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 }
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceThought</code></td>
      <td>Resource-based thought from the agent</td>
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
      <td><code>AgentResourceLinkThought</code></td>
      <td>Resource link thought from the agent</td>
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
      <td><code>ToolCall</code></td>
      <td>Agent makes a tool call</td>
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
      <td><code>AvailableCommands</code></td>
      <td>Available commands are updated</td>
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
      <td><code>Plan</code></td>
      <td>Agent generates a plan</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "entries": [
    { "content": "string", "priority": "High | Medium | Low" }
  ]
}</code></pre></td>
    </tr>
    <tr>
      <td><code>CurrentMode</code></td>
      <td>Current mode changes</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "id": "string"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ConfigurationOption</code></td>
      <td>Configuration option updates</td>
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
    <tr>
      <td><code>PermissionRequest</code></td>
      <td>Request permission from the user</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "toolCall": {
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
  },
  "options": [
    {
      "id": "string",
      "label": "string",
      "description": "string (optional)"
    }
  ]
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ConnectionInitialized</code></td>
      <td>Connection established with agent</td>
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
    <tr>
      <td><code>CreatedSession</code></td>
      <td>New session created</td>
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
    <tr>
      <td><code>Prompted</code></td>
      <td>Agent response received</td>
      <td><pre><code class="language-json">{
  "stopReason": "string (e.g., 'Stop', 'Cancelled', 'Error')"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>Authenticated</code></td>
      <td>Authentication completed</td>
      <td><pre><code class="language-json">{
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ConfigurationUpdated</code></td>
      <td>Session configuration updated</td>
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
    <tr>
      <td><code>ModeUpdated</code></td>
      <td>Session mode changed</td>
      <td><pre><code class="language-json">{
}</code></pre></td>
    </tr>
    <tr>
      <td><code>LoadedSession</code></td>
      <td>Session loaded successfully</td>
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
    <tr>
      <td><code>ListedSessions</code></td>
      <td>Session list received</td>
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
      <td><code>ForkedSession</code></td>
      <td>Session forked successfully</td>
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
    <tr>
      <td><code>ResumedSession</code></td>
      <td>Session resumed successfully</td>
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
    <tr>
      <td><code>SessionModelUpdated</code></td>
      <td>Session model updated</td>
      <td><pre><code class="language-json">{
}</code></pre></td>
    </tr>
  </tbody></table>


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
- [ ] Allow agent to write to files
  - [ ] Automatically refresh open buffers that have been modified
- [ ] Allow agent to read files
- [ ] Allow agent to use terminal
  - [ ] Create autocommands for Agent progress in the terminal

- [x] Allow user to configure/turn off any/all aspects of ACP (if, for example, you just want to send data to the agent but still interact with it via the CLI)

- [ ] look into ways of improving ai integration
  - [ ] research RLM ([example](https://github.com/JaredStewart/coderlm))
  - [ ] connect agent to lsp (try to set it up as a tool call/connect to neovim lsp)
  - [ ] use [whisper.rs](https://crates.io/crates/whisper-rs) to facilitate speech to text
