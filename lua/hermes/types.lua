-- luacov: disable
-- ============================================================================
-- Type Definitions for IDE/LSP Support
-- ============================================================================

---@class HermesDownloadConfig
---Download configuration for binary management
---@field version? string Version to use ("latest" or specific version like "v0.1.0")
---@field auto? boolean Whether to auto-download pre-built binary (default: true)
---@field timeout? number Download timeout in seconds (default: 60)

---@class HermesProgressConfig
---@field cmdline? boolean Show progress messages in cmdline (0.12+ only, default: false)
---@field update_frequency? integer How often to poll for progress in milliseconds (default: 150)

---@class HermesConfig
---Hermes plugin configuration options
---@field download? HermesDownloadConfig Download configuration for binary management
---@field root_markers? string[] Files/directories to identify project root (default: {".git"})
---@field permissions? HermesPermissions Permission settings for agent operations
---@field terminal? HermesTerminalConfig Terminal configuration
---@field buffer? HermesBufferConfig Buffer configuration
---@field session? HermesSessionConfig Session configuration
---@field log? HermesLogConfig Logging configuration
---@field progress? HermesProgressConfig Progress display configuration
---@field agents? table<string, HermesAgentConnection> User-defined agent connections, keyed by agent id

---@class HermesAgentConnection
---User-defined connection for an agent. When the id matches a registry entry,
---only the connection fields are used and the registry metadata is kept.
---Otherwise the agent is treated as a brand new agent.
---@field name? string Display name (defaults to the agent id for new agents)
---@field description? string Description (new agents only)
---@field icon? string HTTP(S) URL or local path to an SVG icon
---@field protocol? "stdio"|"http"|"socket"|"tcp"|"https" Connection protocol (default: "stdio")
---@field command? string Command to run (required for stdio connections)
---@field args? string[] Command arguments for stdio connections
---@field host? string Host (required for tcp/http/https connections)
---@field port? number Port (required for tcp/http/https connections)
---@field path? string Path (required for socket connections)

---@class HermesPermissions
---Permission settings for agent operations
---@field fs_write_access? boolean Allow agent to write files (default: true)
---@field fs_read_access? boolean Allow agent to read files (default: true)
---@field terminal_access? boolean Allow agent to execute terminal commands (default: true)
---@field request_permissions? boolean Allow agent to send permission requests (default: true)
---@field send_notifications? boolean Allow agent to send notifications (default: true)

---@class HermesTerminalConfig
---Terminal configuration options
---@field delete? boolean Auto-delete terminals on exit (default: false)
---@field enabled? boolean Enable terminal functionality (default: true)
---@field hidden? boolean Hide terminal windows (default: true)
---@field buffered? boolean Buffer terminal output (default: true)

---@class HermesBufferConfig
---Buffer configuration options
---@field auto_save? boolean Auto-save modified files after writing (default: false)

---@class HermesSessionConfig
---Session configuration options
---@field store_history? boolean Store session history locally when agent does not support load_session (default: true)

---@class HermesLogConfig
---Logging configuration options
---@field stdio? HermesLogTargetConfig Stdio logging settings
---@field notification? HermesLogTargetConfig Notification logging settings
---@field message? HermesLogTargetConfig Message logging settings (for :messages)
---@field file? HermesLogFileConfig File logging settings

---@class HermesLogTargetConfig
---Log target configuration (stdio, notification, message)
---@field level? number|string Log level (vim.log.levels.* or "trace", "debug", "info", "warn", "error", "off")
---@field format? "compact"|"pretty"|"full"|"json" Log format (default: "compact")
---@field show_ansi? boolean Show ANSI color codes in output (default: false)

---@class HermesLogFileConfig
---File logging configuration
---@field level? number|string Log level (vim.log.levels.* or string)
---@field format? "compact"|"pretty"|"full"|"json" Log format (default: "json")
---@field path? string Path to log directory (default: vim.fn.stdpath('state') .. "/hermes")
---@field name? string Log file name (default: "hermes.log")
---@field show_ansi? boolean Show ANSI color codes in output (default: false)
---@field max_size? number Maximum file size in bytes (default: 10485760 = 10MB)
---@field max_files? number Maximum number of log files to keep (default: 5)

---@class ConnectionOptions
---Options for connecting to an agent
---@field protocol? "stdio"|"http"|"socket"|"tcp" Connection protocol (default: "stdio")
---@field command? string Command to run for stdio connections
---@field args? string[] Command arguments for stdio connections
---@field host? string Host for socket/tcp/http connections
---@field port? number Port for socket/tcp/http connections
---@field url? string URL for http/sse connections
---@field distribution? "npx"|"uvx"|"binary" Preferred distribution type for registry agents (default: auto-select npx > uvx > binary)

---@class EnvVar
---Environment variable entry
---@field name string Environment variable name
---@field value string Environment variable value

---@class McpServer
---MCP (Model Context Protocol) server configuration
---@field type "http"|"sse"|"stdio" Server type
---@field name string Human-readable server name
---@field url? string URL for http/sse servers (required for http/sse types)
---@field headers? table<string, string>[] HTTP headers as array of key-value tables
---@field command? string Command to execute for stdio servers (required for stdio type)
---@field args? string[] Arguments for stdio server command
---@field env? EnvVar[] Environment variables as array of {name: string, value: string}

---@class SessionOptions
---Options for creating or loading a session
---@field cwd? string Working directory for the session
---@field additional_directories? string[] Additional workspace roots for the session (requires agent capability)
---@field mcp_servers? McpServer[] Array of MCP server configurations

---@class ListSessionsOptions
---Options for listing sessions
---@field cwd? string Filter by working directory path
---@field cursor? string Pagination cursor for fetching next page

---@class AgentsOptions
---Options for listing agents from the registry
---@field update? boolean Whether to fetch the latest registry before listing
---@field url? string Custom registry URL (default: official ACP registry)

---@class TextContent
---Text content for prompts
---@field type "text" Type identifier
---@field text string The text content to send

---@class LinkContent
---Resource link content for prompts
---@field type "link" Type identifier
---@field name string Human-readable resource name
---@field uri string Resource URI (file path or URL)
---@field description? string Optional description
---@field mimeType? string Optional MIME type

---@class EmbeddedResource
---Embedded resource data
---@field uri string Resource URI
---@field mimeType? string MIME type of the resource (e.g., "text/x-python", "application/pdf")
---@field text? string Text content (for text resources)
---@field blob? string Base64-encoded binary data (for blob resources)

---@class EmbeddedContent
---Embedded resource content for prompts
---@field type "embedded" Type identifier
---@field resource EmbeddedResource The embedded resource

---@class ImageContent
---Image content for prompts
---@field type "image" Type identifier
---@field data string Base64-encoded image data
---@field mimeType string MIME type (e.g., "image/png", "image/jpeg")
---@field uri? string Optional resource URI

---@class AudioContent
---Audio content for prompts
---@field type "audio" Type identifier
---@field data string Base64-encoded audio data
---@field mimeType string MIME type (e.g., "audio/wav", "audio/mpeg")

---@alias PromptContent TextContent|LinkContent|EmbeddedContent|ImageContent|AudioContent|table

---@class ModeOption
---@field value string Option ID
---@field name string Human-readable name
---@field description? string Optional description
---@field group? string Optional group name

---@class ModelOption
---@field value string Option ID
---@field name string Human-readable name
---@field description? string Optional description
---@field group? string Optional group name

---@class ThoughtLevelOption
---@field value string Option ID
---@field name string Human-readable name
---@field description? string Optional description
---@field group? string Optional group name

---@class SelectionOption
---@field value string Option ID
---@field name string Human-readable name
---@field description? string Optional description
---@field group? string Optional group name

---@class ModelConfigSelection
---@field options SelectionOption[] Available values
---@field current SelectionOption Currently selected value

---@class ModelConfigOption
---@field id string Config option ID
---@field name string Human-readable name
---@field description? string Optional description
---@field selection ModelConfigSelection Current selection and available options

---@class ConfigureModelConfig
---@field id string Config option ID to set
---@field value string Value to set

---@class HermesConfigSelectGroup
---A group of values under a header (select options only)
---@field group string Group ID
---@field name string Human-readable group label
---@field options SelectionOption[] Option values in this group

---@class ConfigOption
---Configuration option for a session (delivered via the ConfigOptions autocommand)
---@field id string Config option ID
---@field name string Human-readable name
---@field description? string Optional description
---@field category? string Option category (e.g. "mode", "model", "thought_level", "model_config")
---@field type "select"|"boolean" Option kind
---@field options? (SelectionOption|HermesConfigSelectGroup)[] Available values (select options only; entries are either options or groups)
---@field currentValue string|boolean Current value (value ID for select options, boolean for boolean options)

---@class SetConfigOptionConfig
---@field id string Config option ID to set
---@field value string|boolean Value to set (string for select options, boolean for boolean options)

-- ============================================================================
-- Autocommand Payload Types
--
-- Every autocommand's `args.data` payload has a corresponding class named
-- after the autocommand (e.g. the AgentTextMessage autocommand delivers a
-- HermesAgentTextMessage payload). Cast `args.data` inside callbacks to get
-- completion and type checking:
--
--   vim.api.nvim_create_autocmd("User", {
--     group = "hermes",
--     pattern = "AgentTextMessage",
--     callback = function(args)
--       ---@type HermesAgentTextMessage
--       local data = args.data
--       print(data.update.content.text)
--     end,
--   })
-- ============================================================================

---@class HermesAnnotations
---@field audience? string[]
---@field lastModified? string ISO 8601 timestamp
---@field priority? number

---@class HermesTextChunk
---@field type "text"
---@field text string
---@field annotations? HermesAnnotations

---@class HermesImageChunk
---@field type "image"
---@field data string Base64-encoded image data
---@field mimeType string
---@field uri? string
---@field annotations? HermesAnnotations

---@class HermesAudioChunk
---@field type "audio"
---@field data string Base64-encoded audio data
---@field mimeType string
---@field annotations? HermesAnnotations

---@class HermesResourceLinkChunk
---@field type "resource_link"
---@field name string
---@field uri string
---@field description? string
---@field mimeType? string
---@field size? number
---@field title? string
---@field annotations? HermesAnnotations

---@class HermesResourceContents
---@field text? string Text content (text resources)
---@field blob? string Base64-encoded binary data (blob resources)
---@field uri string
---@field mimeType? string

---@class HermesResourceChunk
---@field type "resource"
---@field resource HermesResourceContents
---@field annotations? HermesAnnotations

---Base payload for session notifications sent by the agent
---@class HermesSessionNotification
---@field sessionId string
---@field promptId string UUID of the prompt turn

---@class HermesAgentTextMessage: HermesSessionNotification
---@field update { sessionUpdate: "agent_message_chunk", content: HermesTextChunk }

---@class HermesAgentImageMessage: HermesSessionNotification
---@field update { sessionUpdate: "agent_message_chunk", content: HermesImageChunk }

---@class HermesAgentResourceLinkMessage: HermesSessionNotification
---@field update { sessionUpdate: "agent_message_chunk", content: HermesResourceLinkChunk }

---@class HermesAgentResourceMessage: HermesSessionNotification
---@field update { sessionUpdate: "agent_message_chunk", content: HermesResourceChunk }

---@class HermesAgentTextThought: HermesSessionNotification
---@field update { sessionUpdate: "agent_thought_chunk", content: HermesTextChunk }

---@class HermesAgentImageThought: HermesSessionNotification
---@field update { sessionUpdate: "agent_thought_chunk", content: HermesImageChunk }

---@class HermesAgentResourceLinkThought: HermesSessionNotification
---@field update { sessionUpdate: "agent_thought_chunk", content: HermesResourceLinkChunk }

---@class HermesAgentResourceThought: HermesSessionNotification
---@field update { sessionUpdate: "agent_thought_chunk", content: HermesResourceChunk }

---@class HermesUserTextMessage: HermesSessionNotification
---@field update { sessionUpdate: "user_message_chunk", content: HermesTextChunk }

---@class HermesUserImageMessage: HermesSessionNotification
---@field update { sessionUpdate: "user_message_chunk", content: HermesImageChunk }

---@class HermesUserResourceLinkMessage: HermesSessionNotification
---@field update { sessionUpdate: "user_message_chunk", content: HermesResourceLinkChunk }

---@class HermesUserResourceMessage: HermesSessionNotification
---@field update { sessionUpdate: "user_message_chunk", content: HermesResourceChunk }

---@class HermesAvailableCommand
---@field id string
---@field name string
---@field description? string

---@class HermesAvailableCommands: HermesSessionNotification
---@field update { sessionUpdate: "available_commands_update", availableCommands: HermesAvailableCommand[] }

---@class HermesConfigurationOption: HermesSessionNotification
---@field update { sessionUpdate: "config_option_update", configOptions: ConfigOption[] }

---@class HermesModeCurrent: HermesSessionNotification
---@field update { sessionUpdate: "current_mode_update", currentModeId: string }

---@class HermesPlanEntry
---@field content string
---@field priority "High"|"Medium"|"Low"
---@field status "Pending"|"InProgress"|"Completed"|"Cancelled"

---@class HermesPlan: HermesSessionNotification
---@field update { sessionUpdate: "plan", entries: HermesPlanEntry[] }

---@class HermesSessionUpdate: HermesSessionNotification
---@field update { sessionUpdate: "session_info_update", title?: string, updatedAt?: string }

---@class HermesUsageUpdate: HermesSessionNotification
---@field update { sessionUpdate: "usage_update", used: number, size: number, cost: { amount: number, currency: string } }

---@alias HermesNoticeSeverity "info"|"warning"|"error"

---@class HermesNotice: HermesSessionNotification
---@field update { sessionUpdate: "notice", severity: HermesNoticeSeverity, title: string, description?: string }

---@alias HermesCompactionStatus "in_progress"|"completed"|"failed"|"cancelled"

---@class HermesCompactionUpdate: HermesSessionNotification
---@field update { sessionUpdate: "compaction_update", compactionId: string, status: HermesCompactionStatus, summary?: (HermesTextChunk|HermesImageChunk|HermesAudioChunk|HermesResourceChunk|HermesResourceLinkChunk)[], error?: string }

---@class HermesCompactionSummaryChunk: HermesSessionNotification
---@field update { sessionUpdate: "compaction_summary_chunk", compactionId: string, content: HermesTextChunk|HermesImageChunk|HermesAudioChunk|HermesResourceChunk|HermesResourceLinkChunk }

---@alias HermesToolCallKind "Read"|"Edit"|"Delete"|"Move"|"Search"|"Execute"|"Think"|"Fetch"|"SwitchMode"|"Other"

---@class HermesToolCallContentBlock
---@field type "content"
---@field content HermesTextChunk|HermesImageChunk|HermesAudioChunk|HermesResourceChunk|HermesResourceLinkChunk

---@class HermesToolCallDiff
---@field type "diff"
---@field path string
---@field oldText string
---@field newText string

---@class HermesToolCallTerminalRef
---@field type "terminal"
---@field terminalId string

---@alias HermesToolCallContent HermesToolCallContentBlock|HermesToolCallDiff|HermesToolCallTerminalRef

---@class HermesToolCallLocation
---@field path string
---@field line? number

---Tool call details attached to a permission request
---@class HermesToolCallInfo
---@field toolCallId string
---@field kind? HermesToolCallKind
---@field status? HermesToolCallStatus
---@field title? string
---@field name? string Programmatic name of the invoked tool (e.g. read_file)
---@field content? HermesToolCallContent[]
---@field locations? HermesToolCallLocation[]
---@field rawInput? any
---@field rawOutput? any

---@class HermesToolCall: HermesSessionNotification
---@field update { sessionUpdate: "tool_call", toolCallId: string, title: string, name?: string, kind: HermesToolCallKind, status: HermesToolCallStatus, content: HermesToolCallContent[], locations: HermesToolCallLocation[], rawInput?: any, rawOutput?: any }

---@class HermesToolCallUpdate: HermesSessionNotification
---@field update { sessionUpdate: "tool_call_update", toolCallId: string, kind?: HermesToolCallKind, status?: HermesToolCallStatus, title?: string, name?: string, content?: HermesToolCallContent[], locations?: HermesToolCallLocation[], rawInput?: any, rawOutput?: any }

---@class HermesPermissionOption
---@field id string Option ID to pass to respond()
---@field label string Human-readable label
---@field description? string

---@class HermesPermissionRequest
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field toolCall HermesToolCallInfo
---@field options HermesPermissionOption[]

---@class HermesReadTextFile
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field path string
---@field line? number 1-based line to start reading from
---@field limit? number Maximum number of lines to read

---@class HermesWriteTextFile
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field path string
---@field content string

---@class HermesTerminalCreate
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field command string
---@field args string[]
---@field env EnvVar[]
---@field cwd? string
---@field outputByteLimit? number

---@class HermesTerminalExit
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field terminalId string

---@class HermesTerminalKill
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field terminalId string
---@field signal? string e.g. "SIGTERM", "SIGKILL"

---@class HermesTerminalOutput
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field terminalId string
---@field byteLimit? number

---@class HermesTerminalRelease
---@field requestId string UUID to pass to respond()
---@field sessionId string
---@field terminalId string

---@class HermesFormElicitation
---@field requestId string UUID to pass to respond()
---@field mode "form"
---@field scope? table
---@field schema? table JSON schema describing the expected form fields
---@field message string Prompt shown to the user

---@class HermesUrlElicitation
---@field requestId string UUID to pass to respond()
---@field mode "url"
---@field scope? table
---@field url string URL to direct the user to
---@field message string Prompt shown to the user

---@class HermesElicitationComplete
---@field elicitationId string UUID of the completed elicitation

---@class HermesRequestCancelled
---@field requestId string UUID of the request the agent cancelled
---@field sessionId string

---@class HermesAgentInfo
---@field id string Agent identifier
---@field name string Human-readable name
---@field version string Agent version
---@field license? string License identifier
---@field description string Description
---@field website? string Website URL
---@field repository? string Repository URL
---@field icon? string Icon URL or path
---@field icon_pixel_data? { width: number, height: number, pixels: number[] } Rendered icon pixels
---@field distributions string[] e.g. { "binary", "npx", "uvx" }

---@class HermesAgentList
---@field agents HermesAgentInfo[]

---@class HermesAuthMethod
---@field id string Authentication method ID to pass to authenticate()
---@field name string
---@field description? string

---@class HermesPromptCapabilities
---@field image boolean
---@field audio boolean
---@field embeddedContext boolean

---@class HermesMcpCapabilities
---@field http boolean
---@field sse boolean
---@field acp boolean

---@class HermesSessionCapabilities
---@field list boolean
---@field fork boolean
---@field resume boolean
---@field close boolean
---@field additionalDirectories boolean
---@field delete boolean

---@class HermesAgentCapabilities
---@field loadSession boolean
---@field promptCapabilities HermesPromptCapabilities
---@field mcpCapabilities HermesMcpCapabilities
---@field sessionCapabilities HermesSessionCapabilities
---@field auth { logout: boolean }

---@class HermesAgentImplementation
---@field name string
---@field version string
---@field title? string

---@class HermesConnectionInitialized
---@field protocolVersion string
---@field agentCapabilities HermesAgentCapabilities
---@field authMethods HermesAuthMethod[]
---@field agentInfo HermesAgentImplementation

---@class HermesPrompted
---@field stopReason string e.g. "Stop", "Cancelled", "Error"

---@class HermesProgress
---@field id string
---@field title string
---@field source string
---@field status "running"|"success"|"failure"
---@field percent? number
---@field text? string[]

---@class HermesSessionMode
---@field id string
---@field name string
---@field description? string

---@class HermesSessionModes
---@field currentModeId string
---@field availableModes HermesSessionMode[]

---@class HermesSessionCreated
---@field sessionId string
---@field modes HermesSessionModes
---@field configOptions ConfigOption[]

---@class HermesSessionForked: HermesSessionCreated

---@class HermesSessionLoaded
---@field modes HermesSessionModes
---@field configOptions ConfigOption[]

---@class HermesSessionResumed: HermesSessionLoaded

---@class HermesSessionClosed
---@field sessionId string

---@class HermesSessionDeleted
---@field sessionId string

---@class HermesSessionListItem
---@field sessionId string
---@field cwd string
---@field additionalDirectories string[]
---@field title? string
---@field updatedAt? string

---@class HermesSessionsListed
---@field sessions HermesSessionListItem[]
---@field nextCursor? string Pagination cursor for fetching the next page

---@class HermesModes
---@field options ModeOption[]
---@field current ModeOption

---@class HermesModels
---@field options ModelOption[]
---@field current ModelOption

---@class HermesThoughtLevels
---@field options ThoughtLevelOption[]
---@field current ThoughtLevelOption

---@class HermesConfigurationUpdated
---@field configOptions ConfigOption[]

---@class HermesTerminalAuthentication
---@field requestId string UUID to pass to respond()
---@field id string Authentication method ID
---@field name string
---@field description? string
---@field args? string[] Additional arguments to append when invoking the command
---@field env? table<string, string> Additional environment variables
---@field command string Program required to run the agent in a terminal

---@alias HermesConfigOptions ConfigOption[]
---@alias HermesModelConfigurations ModelConfigOption[]
---@alias HermesModelConfigurationUpdated ModelConfigOption[]
---@alias HermesModeUpdated ModeOption
---@alias HermesSessionModelUpdated ModelOption
---@alias HermesThoughtLevelUpdated ThoughtLevelOption
-- luacov: enable

-- ============================================================================
-- Enums for type-safe API usage
-- ============================================================================

---@enum HermesProtocol
local Protocol = {
	stdio = "stdio",
	http = "http",
	socket = "socket",
	tcp = "tcp",
	https = "https",
}

---@enum HermesDistribution
local Distribution = {
	npx = "npx",
	uvx = "uvx",
	binary = "binary",
}

---@enum HermesMcpServerType
local McpServerType = {
	http = "http",
	sse = "sse",
	stdio = "stdio",
}

---@enum HermesPermissionKind
local PermissionKind = {
	allow_once = "allow_once",
	allow_always = "allow_always",
	reject_once = "reject_once",
	reject_always = "reject_always",
}

---@enum HermesElicitationAction
local ElicitationAction = {
	accept = "accept",
	cancel = "cancel",
	decline = "decline",
}

---@enum HermesElicitationOutcome
local ElicitationOutcome = {
	accepted = "accepted",
	cancelled = "cancelled",
	declined = "declined",
}

---@enum HermesPermissionOutcome
local PermissionOutcome = {
	selected = "selected",
	cancelled = "cancelled",
}

---@enum HermesToolCallStatus
local ToolCallStatus = {
	pending = "pending",
	in_progress = "in_progress",
	completed = "completed",
	failed = "failed",
}

---@enum HermesPlanPriority
local PlanPriority = {
	high = "high",
	medium = "medium",
	low = "low",
}

---@enum HermesPlanStatus
local PlanStatus = {
	pending = "pending",
	in_progress = "in_progress",
	completed = "completed",
}

---@enum HermesConfigOptionType
local ConfigOptionType = {
	select = "select",
	boolean = "boolean",
}

---@enum HermesContentType
local ContentType = {
	text = "text",
	link = "link",
	embedded = "embedded",
	image = "image",
	audio = "audio",
	resource = "resource",
}

---@enum HermesLogLevel
local LogLevel = {
	trace = 0,
	debug = vim.log and vim.log.levels and vim.log.levels.DEBUG or 1,
	info = vim.log and vim.log.levels and vim.log.levels.INFO or 2,
	warn = vim.log and vim.log.levels and vim.log.levels.WARN or 3,
	error = vim.log and vim.log.levels and vim.log.levels.ERROR or 4,
	off = vim.log and vim.log.levels and vim.log.levels.OFF or 0,
}

---@enum HermesLogFormat
local LogFormat = {
	compact = "compact",
	pretty = "pretty",
	full = "full",
	json = "json",
}

---@enum HermesAutocommand
local Autocommand = {
	AgentImageMessage = "AgentImageMessage",
	AgentImageThought = "AgentImageThought",
	AgentList = "AgentList",
	AgentResourceLinkMessage = "AgentResourceLinkMessage",
	AgentResourceLinkThought = "AgentResourceLinkThought",
	AgentResourceMessage = "AgentResourceMessage",
	AgentResourceThought = "AgentResourceThought",
	AgentTextMessage = "AgentTextMessage",
	AgentTextThought = "AgentTextThought",
	Authenticated = "Authenticated",
	AvailableCommands = "AvailableCommands",
	CompactionSummaryChunk = "CompactionSummaryChunk",
	CompactionUpdate = "CompactionUpdate",
	ConfigOptions = "ConfigOptions",
	ConfigurationOption = "ConfigurationOption",
	ConfigurationUpdated = "ConfigurationUpdated",
	ConnectionInitialized = "ConnectionInitialized",
	ElicitationComplete = "ElicitationComplete",
	FormElicitation = "FormElicitation",
	LoggedOut = "LoggedOut",
	ModeCurrent = "ModeCurrent",
	ModelConfigurations = "ModelConfigurations",
	ModelConfigurationUpdated = "ModelConfigurationUpdated",
	Models = "Models",
	Modes = "Modes",
	ModeUpdated = "ModeUpdated",
	Notice = "Notice",
	PermissionRequest = "PermissionRequest",
	Plan = "Plan",
	Progress = "Progress",
	Prompted = "Prompted",
	ReadTextFile = "ReadTextFile",
	RequestCancelled = "RequestCancelled",
	SessionClosed = "SessionClosed",
	SessionCreated = "SessionCreated",
	SessionDeleted = "SessionDeleted",
	SessionForked = "SessionForked",
	SessionLoaded = "SessionLoaded",
	SessionModelUpdated = "SessionModelUpdated",
	SessionResumed = "SessionResumed",
	SessionsListed = "SessionsListed",
	SessionUpdate = "SessionUpdate",
	TerminalAuthentication = "TerminalAuthentication",
	TerminalCreate = "TerminalCreate",
	TerminalExit = "TerminalExit",
	TerminalKill = "TerminalKill",
	TerminalOutput = "TerminalOutput",
	TerminalRelease = "TerminalRelease",
	ThoughtLevels = "ThoughtLevels",
	ThoughtLevelUpdated = "ThoughtLevelUpdated",
	ToolCall = "ToolCall",
	ToolCallUpdate = "ToolCallUpdate",
	UrlElicitation = "UrlElicitation",
	UsageUpdate = "UsageUpdate",
	UserImageMessage = "UserImageMessage",
	UserResourceLinkMessage = "UserResourceLinkMessage",
	UserResourceMessage = "UserResourceMessage",
	UserTextMessage = "UserTextMessage",
	WriteTextFile = "WriteTextFile",
}

local enums = {
	Protocol = Protocol,
	Distribution = Distribution,
	McpServerType = McpServerType,
	PermissionKind = PermissionKind,
	ElicitationAction = ElicitationAction,
	ElicitationOutcome = ElicitationOutcome,
	PermissionOutcome = PermissionOutcome,
	ToolCallStatus = ToolCallStatus,
	PlanPriority = PlanPriority,
	PlanStatus = PlanStatus,
	ConfigOptionType = ConfigOptionType,
	ContentType = ContentType,
	LogLevel = LogLevel,
	LogFormat = LogFormat,
  Autocommand = Autocommand,
}

for key, value in pairs(enums) do
	setmetatable(value, {
		__newindex = function()
			error('Attempt to modify a read-only enum: "' .. key .. '"')
		end,
	})
end

return enums

