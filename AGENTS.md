## Codebase

Hermes is an interface between [Neovim](https://neovim.io/) and [ACP](https://agentclientprotocol.com/).

### Architecture

The architecture separates Neovim logic from Rust ACP interactions:

- **Directory Structure:**
  - `src/acp`: Contains all direct interactions with the ACP SDK.
  - `src/nvim`: Contains Neovim-specific bindings and logic.

- **Concurrency Model:** The ACP SDK is single-threaded and async. We spawn a dedicated thread for each connection, each running a single-threaded Tokio runtime. This ensures every Agent has its own [independent](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html) environment. Thread handles are stored to be joined and dropped upon disconnection.

- **Communication:**
  - Agent threads communicate with the main Neovim thread via [mpsc channels](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.channel.html).
  - Since Neovim is synchronous, an [AsyncHandle callback](https://docs.rs/nvim-oxi-libuv/latest/nvim_oxi_libuv/struct.AsyncHandle.html) triggers the processing of messages on the main thread.

### Message Handling

Messages sent by agent threads to the main Neovim thread are handled in three ways:

1.  **Autocommand:** For informational events requiring no user response.
2.  **Callback:** Executes a user-defined callback when the agent requires a response.
3.  **Action:** Performs a specific action for the agent (e.g., read file, write file, terminal command).

## Documentation

Essential references for development:

- **Neovim Bindings:** [nvim-oxi documentation](https://docs.rs/nvim-oxi/latest/nvim_oxi/)
- **ACP SDK:** [Rust SDK documentation](https://docs.rs/agent-client-protocol-schema/latest/agent_client_protocol_schema/index.html)
- **Protocol:** [Agent Client Protocol documentation](https://agentclientprotocol.com/get-started/introduction)
- **Testing:** [pretty_assertions documentation](https://docs.rs/pretty_assertions/latest/pretty_assertions/)
- **Test Runner:** [cargo-nextest documentation](https://nexte.st/)

## Practices

### Tool Usage

**Always use the LSP tool when available and appropriate.** It provides:
- Precise symbol navigation (go to definition, find references)
- Accurate type information and documentation
- Efficient codebase exploration without reading entire files
- Real-time error detection and hover information

**When to use LSP:**
- Navigating to function/struct definitions
- Understanding function signatures and types
- Exploring module structure (documentSymbol)
- Finding all references to a symbol
- Getting type information at specific locations

**When LSP is not available:**
- Global text search (use grep)
- File discovery (use glob)
- Complex multi-step operations (use task agent)
- Web documentation lookup (use webfetch/codesearch)

**Priority:** LSP > grep > read > other tools when working with code.

### Code Style
Adhere to "Clean Code" patterns.
- **Design:** Apply SOLID principles where applicable.

## Testing

Tests ensure code reliability and prevent regression.

### Guidelines

- **Coverage:** Cover all code paths, including edge cases and error handling.
- **Assertions:**
  - Use `assert_eq!` to verify exact values.
  - Avoid `assert!` with boolean checks (e.g., `is_some()`) when the value itself can be verified.
- **Scope:** Each test should verify a single behavior or unit. Use only **one assertion** per test unless absolutely necessary.
- **Debugging:** Run tests locally to verify fixes.

### Test Redundancy

Aim for the **minimum number of tests that cover all code paths**. Avoid testing the same logic multiple times.

**Examples of redundancy to avoid:**

- **Language features:** Do not test Rust's built-in functionality (e.g., auto-derived traits like `PartialEq`, `Clone`, `Debug`, field access, Arc/Mutex usage). Assume the Rust compiler and standard library work correctly. Only test your own logic and custom trait implementations. Reading a field from a struct through an Arc/Mutex is standard Rust - don't test it.
- **Shared validation logic:** If multiple types share the same initial validation (e.g., checking for a required "type" field), test it once for one type, not for every type.
- **Trivial accessors:** Enum variant extraction methods (e.g., `into_vec()` on a simple wrapper) may not need dedicated tests if the logic is obvious and covered indirectly.
- **Collection iteration:** If individual elements are thoroughly tested, you typically need only one test for the collection wrapper to verify iteration works.

**When to keep seemingly similar tests:**

- Different error branches (e.g., missing field A vs missing field B) each need their own test
- Different input formats (e.g., single item vs array) need separate tests
- Different code paths within the same function should each be tested

**Principle:** If removing a test would leave a code path uncovered, keep it. If multiple tests hit the exact same `if` branch with the same logic, consolidate them.

### Test Types

We follow the [Testing Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html) approach:

- **Unit Tests** (`src/**/*.rs`): Fast, isolated tests for individual functions and modules. These should be comprehensive and cover all code paths, edge cases, and error scenarios.
  - Located alongside source code in `#[cfg(test)]` modules
  - Use `cargo test --lib` to run
  - Should cover all parsing logic, validation, and conversion functions
  - **Important:** Any test that involves actual message flow between Neovim and ACP (e.g., sending requests, receiving responses, autocommand firing) should be considered an **integration test**, not a unit test

- **E2E Tests** (`e2e/`): Integration tests that verify the full flow from Lua API through to autocommand responses.
  - Run inside a Neovim instance using `#[nvim_oxi::test]`
  - Focus on verifying that components integrate correctly
  - Test representative scenarios rather than exhaustive coverage
  - Should not duplicate unit test coverage - if a parsing edge case is covered in unit tests, don't repeat it in E2E

**Guideline**: E2E tests verify that "the system works together", unit tests verify that "each component works correctly". Keep E2E tests minimal and focused on integration points.

### Running Tests

We use [cargo-nextest](https://nexte.st/) as our test runner. Nextest provides:
- **Clear output** with progress bars and readable test listings
- **Better performance** through parallel test execution
- **Test filtering** and granular control over test execution
- **Failure handling** with automatic retries and output capture

**Run all tests:**
```bash
cargo nextest run
```

**Run unit tests only:**
```bash
cargo nextest run --lib
```

**Run E2E tests:**
```bash
cd e2e && cargo nextest run
```

**Run a specific test:**
```bash
cargo nextest run test_name
```

### Writing Tests with Pretty Assertions

When writing tests, use `pretty_assertions::assert_eq!` instead of the standard `assert_eq!`. This provides:
- **Side-by-side diffs** showing exactly what differs between expected and actual values
- **Colorized output** making differences easy to spot
- **Better formatting** for complex nested structures

**Example:**
```rust
use pretty_assertions::assert_eq;

#[test]
fn test_complex_struct() {
    let expected = vec![1, 2, 3];
    let actual = vec![1, 2, 4];
    assert_eq!(expected, actual);  // Shows a clear diff of the difference
}
```

**All test modules should import pretty_assertions:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    // ... tests
}
```

### Examples

**Good Test:** Verifies the *exact output* for a given input.

```rust
fn add_less_than_ten(a: i32, b: i32) -> Option<i32> {
    if a < 10 && b < 10 {
      Some(a + b)
    } else {
      None
    }
}

#[test]
fn test_addition_function() {
   let a = 1;
   let b = 2;
   // Correct: Verifies the exact value
   assert_eq!(add_less_than_ten(a, b), Some(3));
}
```

**Bad Test:** Only verifies *existence*, missing correctness.

```rust
#[test]
fn test_addition_function() {
   let a = 1;
   let b = 2;
   // Incorrect: Only checks if result is Some, not if it's 3
   assert!(add_less_than_ten(a, b).is_some());
}
```

**Assertion Style:**

```rust
// Bad
assert!("something" == "something");

// Good
assert_eq!("something", "something");
```
