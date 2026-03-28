# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **BREAKING**: Simplified version management - binaries are now downloaded once and never auto-updated
  - Removed automatic cache and version checking on startup
  - Removed `HERMES_CACHE_DURATION` global variable
  - Removed version comparison logic that caused unnecessary HTTP requests
- **BREAKING**: Removed `HERMES_LOG_FORMAT` global variable - logging format now configured via `setup()`
- **BREAKING**: Unified log target configuration - each target (notification, message, quickfix, local_list) now uses `{ level, format }` structure
- Removed dependency on global Neovim log level - Hermes now uses defaults until configured

### Added
- `:Hermes update` command - explicitly fetch and install the latest version from GitHub
- Per-target format configuration (each target has its own format, defaults to "compact")
- `LogTargetConfig` and `LogTargetConfigPartial` structs for unified target configuration
- Each log target can have its own format (no global format, each target is independent)

### Migration Guide
Before:
```lua
vim.g.HERMES_LOG_FORMAT = "json"
require("hermes").setup({
    log = {
        level = vim.log.levels.INFO,
        notification = vim.log.levels.ERROR,
        message = vim.log.levels.DEBUG,
    }
})
```

After:
```lua
require("hermes").setup({
    log = {
        level = vim.log.levels.INFO,
        notification = { 
            level = vim.log.levels.ERROR,
            format = "json"  -- Each target has its own format
        },
        message = { 
            level = vim.log.levels.DEBUG,
            format = "pretty"  -- Different format for messages
        },
    }
})
```

## [0.1.0] - 2026-02-17

### Added
- Initial implementation of ACP client for Neovim
- `AcpClient` struct implementing the `Client` trait from agent-client-protocol
- `ClientConfig` for configurable capabilities (filesystem and terminal)
- `PluginState` for managing Neovim plugin state
- `NvimError` custom error type for Neovim-specific errors
- Comprehensive unit tests (8 tests)
- Integration tests (7 tests)
- Documentation tests (12 tests)
- Full API documentation with examples
- README with usage examples
- Support for session notifications
- Support for file system operations (read/write text files)
- Support for terminal operations (create, output, wait, release)
- Clean architecture following SOLID principles

### Dependencies
- agent-client-protocol v0.9.4
- nvim-oxi v0.6.0 (with neovim-0-10 feature)
- nvim-utils v0.1.12
- tokio v1.49+ (async runtime)
- serde v1.0 (serialization)
- async-trait v0.1 (async trait support)
- thiserror v2.0 (error handling)
- anyhow v1.0 (error context)

### Security
- Zero security vulnerabilities (CodeQL scan)
- All dependencies checked and up-to-date
- Using latest stable versions

[0.1.0]: https://github.com/Ruddickmg/hermes/releases/tag/v0.1.0
