# Hermes

An ACP (Agent Client Protocol) Client implementation designed for integration with Neovim

## Overview

Hermes is a messaging layer for Neovim, not a complete AI assistant. It has no built-in UI, instead it provides APIs and hooks for building your own workflow while routing client-agent communication.

Hermes focuses on:
- APIs for making requests to AI Assistants (prompt, connect, authenticate, etc)
- Hooks into requests from AI assistants that require responses (permission requests, access requests, etc)
- Autocommands for updates on communication between the user (client) and assistant (agent) 

## Features

- [x] Full implementation of ACP Client
- [x] Configurable capabilities (filesystem, terminal, etc)
- [x] Trigger Autocommands for messages/notifications
- [x] Allow connecting to Agents
  - [x] Via stdio
  - [ ] Via http
  - [ ] Via linux socket
  - [x] handle authentication
- [ ] Allow mode selection
- [ ] Allow model selection
- [ ] Allow agent to write to files
  - [ ] Automatically refresh open buffers that have been modified
- [ ] Allow agent to read files
- [ ] Allow agent to use terminal
  - [ ] Create autocommands for Agent progress in the terminal
- [ ] Allow user to give permission when needed
- [ ] Allow user to configure/turn off any/all aspects of ACP (if, for example, you just want to send data to the agent but still interact with it via the CLI)
- [ ] Allow user to send prompts
  - [ ] Send files
  - [ ] Send text
  - [ ] Send images 
  - [ ] Send resource links
  - [ ] Send audio
    - [ ] allow collecting audio input
    - [ ] use [whisper.rs](https://crates.io/crates/whisper-rs) to facilitate speech to text
  - [ ] Cancel
- [ ] Speech
  

## API

Below are a list of functions that Hermes provides to send requests to ai assistants.

### Connect

This method allows you to connect to an agent, it takes the agent name as a bd
n argument.

```lua
local hermes = require("hermes")

hermes.connect({
    agent = "copilot", -- optional, defaults to "copilot", can be "copilot" | "opencode"
    protocol = "stdio", -- optional, defaults to "stdio"
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

Below is a list of all autocommands and their associated data (passed to the callback in the `args.data` field).

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
      <td><code>ClientTextMessage</code></td>
      <td>Message text sent from the client</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": {
    "audience": ["Role1", "Role2"],
    "lastModified": "ISO8601 string",
    "priority": "number"
  },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ClientImageMessage</code></td>
      <td>An image sent from the client</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ClientResourceMessage</code></td>
      <td>A resource sent from the client</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "resource": {
    "text": "string (if text resource)",
    "blob": "string (if blob resource)",
    "uri": "string",
    "mimeType": "string (optional)"
  },
  "annotations": { "audience": [], "lastModified": "string" },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>ClientResourceLinkMessage</code></td>
      <td>A resource link from the client</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "name": "string",
  "uri": "string",
  "description": "string (optional)",
  "mimeType": "string (optional)",
  "size": "number (optional)",
  "title": "string (optional)",
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentTextMessage</code></td>
      <td>A text message from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
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
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
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
  "annotations": { "audience": [], "lastModified": "string" },
  "meta": "JSON value"
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
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentTextThought</code></td>
      <td>Text-based reasoning from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "text": "string",
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentImageThought</code></td>
      <td>Image-based reasoning from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "data": "base64 string",
  "mimeType": "string",
  "uri": "string (optional)",
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceThought</code></td>
      <td>Resource-based reasoning from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "resource": {
    "text": "string (if text resource)",
    "blob": "string (if blob resource)",
    "uri": "string",
    "mimeType": "string (optional)"
  },
  "annotations": { "audience": [], "lastModified": "string" },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentResourceLinkThought</code></td>
      <td>Resource link reasoning from the agent</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "name": "string",
  "uri": "string",
  "description": "string (optional)",
  "mimeType": "string (optional)",
  "size": "number (optional)",
  "title": "string (optional)",
  "annotations": { "audience": [], "priority": 1 },
  "meta": "JSON value"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentToolCall</code></td>
      <td>Agent makes a tool call</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "id": "string",
  "title": "string",
  "kind": "Read | Edit | EditFile | Browser | Terminal | Command | MultiEdit | ReadWithEdits | WebFetch | StrReplaceEdit",
  "status": "Pending | InProgress | Completed",
  "content": [
    { "type": "text", "text": "string" },
    { "type": "image", "data": "base64", "mimeType": "image/png" },
    { "type": "resource", "resource": { "text": "string", "uri": "string" } },
    { "type": "resourcelink", "name": "string", "uri": "string" },
    { "type": "terminal", "id": "string" },
    { "type": "diff", "path": "string", "new_text": "string", "old_text": "string (optional)" }
  ],
  "locations": [
    { "path": "string", "line": "number (optional)" }
  ],
  "input": "JSON string (optional)",
  "output": "JSON string (optional)"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentToolCallUpdate</code></td>
      <td>Tool call is updated (e.g., progress, output)</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "id": "string",
  "fields": [
    { "type": "text", "text": "string" },
    { "type": "image", "data": "base64", "mimeType": "image/png" },
    { "type": "resource", "resource": { "text": "string", "uri": "string" } },
    { "type": "resourcelink", "name": "string", "uri": "string" },
    { "type": "terminal", "id": "string" },
    { "type": "diff", "path": "string", "new_text": "string", "old_text": "string (optional)" }
  ],
  "meta": "JSON value (optional)"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentAvailableCommands</code></td>
      <td>Available commands are updated</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "commands": [
    {
      "name": "string",
      "description": "string",
      "input": { "hint": "string" }
    },
    {
      "name": "string",
      "description": "string"
    }
  ],
  "meta": "JSON value (optional)"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentPlan</code></td>
      <td>Agent generates a plan</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "entries": [
    { "content": "string", "priority": "High | Medium | Low" },
    { "content": "string", "priority": "High | Medium | Low" }
  ],
  "meta": "JSON value (optional)"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentCurrentMode</code></td>
      <td>Current mode changes</td>
      <td><pre><code class="language-json">{
  "sessionId": "string",
  "id": "string",
  "meta": "JSON value (optional)"
}</code></pre></td>
    </tr>
    <tr>
      <td><code>AgentConfigOption</code></td>
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
    },
  ],
  "meta": "JSON value (optional)"
}</code></pre></td>
    </tr>
  </tbody>
</table>
