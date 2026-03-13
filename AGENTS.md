## Codebase

Hermes is an interface between [Neovim](https://neovim.io/) and [ACP](https://agentclientprotocol.com/).

### Architecture

The architecture separates Neovim logic from Rust ACP interactions:

- **Directory Structure:**
  - `src/acp`: Contains all direct interactions with the ACP SDK.
  - `src/nvim`: Contains Neovim-specific bindings and logic.
  - tests/integration: Contains integration tests.
  - tests/e2e: Contains end to end tests.

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
- **Property Testing:** [proptest documentation](https://docs.rs/proptest/latest/proptest/)
- **Mocking:** [mockall documentation](https://docs.rs/mockall/latest/mockall/)

## Practices

### ⚠️ CRITICAL: Git Safety - DO NOT MAKE DESTRUCTIVE CHANGES WITHOUT EXPLICIT PERMISSION ⚠️

**NEVER execute any git commands that modify repository state without explicit user permission. This includes:**
- `git checkout -- .` or `git checkout` (destructive reset of working directory)
- `git reset` (destructive reset of commits or staging area)
- `git clean` (removes untracked files permanently)
- `git stash` or `git stash pop` (can cause work to be lost)
- `git rebase` (rewrites commit history)
- `git push --force` or `git push -f` (destructive remote history rewrite)
- `git cherry-pick` (modifies commit history)
- `git branch -d` or `git branch -D` (deletes branches)
- `git merge` (without explicit permission)
- Any branch switching (`git checkout <branch>` or `git switch`)

**ALLOWED git operations (read-only):**
- `git status` - checking repository status
- `git log` - viewing commit history
- `git diff` - viewing changes
- `git show` - viewing specific commits
- `git branch` - listing branches (without `-d` or `-D`)

**The user MUST explicitly state they want destructive git operations before executing them. Loss of uncommitted work due to unauthorized git operations is UNACCEPTABLE and CRITICAL.**

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

- **E2E Tests** (`tests/e2e/`): Integration tests that verify the full flow from Lua API through to autocommand responses.
  - Run inside a Neovim instance using `#[nvim_oxi::test]`
  - Focus on verifying that components integrate correctly
  - Test representative scenarios rather than exhaustive coverage
  - Should not duplicate unit test coverage - if a parsing edge case is covered in unit tests, don't repeat it in E2E

- **Integration Tests** (`tests/integration/`): Tests for isolated component integration that requires Neovim but not full system flow.
  - Run inside a Neovim instance using `#[nvim_oxi::test]`
  - Test individual components that interact with Neovim APIs (e.g., file operations, buffer manipulation)
  - Use when you need to test code that makes direct Neovim API calls but doesn't require full ACP message flow
  - Focus on verifying that Neovim-interacting code works correctly in isolation
  - Use `assert_fs` for filesystem assertions in file-related tests
  - Example: Testing `Responder::WriteFileResponse` which uses `nvim_oxi::api::command` and buffer operations

**Guideline**: E2E tests verify that "the system works together", unit tests verify that "each component works correctly", integration tests verify that "Neovim-interacting components work correctly". Keep E2E tests minimal and focused on integration points.

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

### Mocking with Mockall

Use [mockall](https://docs.rs/mockall/latest/mockall/) for creating mock objects in unit tests. Mockall provides powerful mocking capabilities for traits and structs.

**When to use mockall:**
- Testing code that depends on external traits (e.g., Client trait, ResponseHandler trait)
- Isolating units under test from their dependencies
- Verifying that certain methods were called with expected arguments
- Simulating error conditions from dependencies

**Basic usage with `#[automock]`:**
```rust
use mockall::*;
use mockall::predicate::*;

#[automock]
trait MyTrait {
    fn foo(&self, x: u32) -> u32;
}

#[test]
fn test_with_mock() {
    let mut mock = MockMyTrait::new();
    mock.expect_foo()
        .with(eq(4))
        .times(1)
        .returning(|x| x + 1);
    
    assert_eq!(5, call_with_four(&mock));
}
```

**Key features:**
- **`#[automock]` attribute**: Automatically generates mock implementations for traits
- **Expectations**: Set required call counts, argument matchers, and return values
- **Argument matching**: Use `with()` or `withf()` to verify arguments
- **Return values**: Use `returning()` for closures or `return_const()` for constants
- **Sequences**: Enforce call order with `in_sequence()`
- **Async support**: Works with async traits using `#[async_trait]`

**Example with async trait:**
```rust
#[automock]
#[async_trait]
trait MyAsyncTrait {
    async fn foo(&self) -> u32;
}

#[tokio::test]
async fn test_async_mock() {
    let mut mock = MockMyAsyncTrait::new();
    mock.expect_foo()
        .times(1)
        .returning(|| 42);
    
    assert_eq!(42, mock.foo().await);
}
```

**Important:** Mockall is already included in dev-dependencies. Use `#[automock]` on traits that need mocking rather than writing manual mock implementations.

**When NOT to use mockall:**
- **Complex trait bounds with multiple traits**: When a type parameter requires multiple traits (e.g., `T: Client + ResponseHandler`), mockall's `mock!` macro can struggle with the complexity, especially when traits have:
  - Generic methods with complex lifetime bounds
  - Conflicting method names between traits
  - Associated types or async methods with different constraints
  
In such cases, a simple manual mock struct may be more practical than fighting with macro limitations. For example, in `src/acp/handler/client.rs`, we use a manual `MockClient` instead of mockall because the handler requires `Client + ResponseHandler` bounds that are difficult to mock together.

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

### Property-Based Testing

For testing functions with many possible inputs, use [proptest](https://docs.rs/proptest/latest/proptest/) for property-based testing. Proptest generates random test cases and automatically finds minimal failing examples.

**Use proptest for:**
- Round-trip conversions (parsing then serializing should yield original value)
- Input validation functions
- String parsing and formatting
- Any function with many valid input combinations

**Example:**
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_log_level_roundtrip(level in 0i64..10) {
        let log_level = LogLevel::from(level);
        // Property: converting from i64 should never panic
        // and should map unknown values to LogLevel::Off
    }

    #[test]
    fn test_string_case_insensitive(name in "[a-zA-Z]+") {
        let lower = name.to_lowercase();
        let upper = name.to_uppercase();
        // Property: case-insensitive parsing should yield same result
        assert_eq!(Assistant::from(&lower), Assistant::from(&upper));
    }
}
```

**Benefits:**
- **More coverage** with less test code
- **Automatic shrinking** finds minimal failing cases
- **Regression testing** saves failing cases for future runs

**Do NOT use proptest for:**
- **E2E tests** - Property-based testing with random inputs can:
  - Hit API rate limits by generating thousands of requests
  - Incur costs for external agent API calls
  - Cause non-deterministic test failures due to timing or external state
  - Create unpredictable load on external services
- **Tests with side effects** - Tests that modify files, databases, or external state
- **Tests with external dependencies** - Tests that require specific network conditions or external services to be available

**Where proptest IS appropriate:**
- Pure parsing/validation functions with no external dependencies
- String formatting and conversion utilities
- Mathematical operations on simple data types
- Round-trip conversions where input and output are deterministic
- Functions where the only "side effect" is CPU/memory usage

**Example of appropriate proptest use:**
```rust
// GOOD: Pure string→enum conversion with no external deps
proptest! {
    #[test]
    fn test_assistant_parsing(input in "[a-zA-Z0-9_]*") {
        let _ = Assistant::from(input.as_str()); // Never panics
    }
}

// BAD: Would generate thousands of agent API calls
proptest! {
    #[test]
    fn test_prompt_response(prompt in "[a-zA-Z]*") {
        // DON'T DO THIS - hits rate limits, costs money
        agent.prompt(prompt).await.unwrap(); 
    }
}
```

### Property-Based Testing for Lua→Rust Conversions

The codebase extensively uses `FromObject` traits to convert Lua objects to Rust types. These conversions are excellent candidates for proptest because they:
- Are pure functions (no external dependencies)
- Handle many edge cases (empty strings, unicode, nested structures)
- Use `unsafe` code that should never panic
- Should be robust against malformed input

**Current proptest coverage includes:**

1. **Logging utilities** (`src/utilities/logging.rs`):
   - `LogLevel::from(i64)` - tests all integer inputs
   - `LogLevel::from(&str)` - tests random strings
   - `LogFormat::from(&str)` - tests format string parsing

2. **Connection management** (`src/acp/connection/manager.rs`):
   - `Assistant::from(&str)` - tests agent name parsing
   - `Protocol::from(&str)` - tests protocol string parsing
   - Case-insensitive parsing validation

3. **API argument parsing**:
   - `DisconnectArgs` (`src/nvim/api/disconnect.rs`) - tests agent name list parsing
   - `Permissions` (`src/nvim/configuration/permissions.rs`) - tests boolean field handling
   - `ConnectionArgs` (`src/nvim/api/connect.rs`) - tests agent name validation
   - `McpServerType` (`src/nvim/api/create_session.rs`) - tests server type parsing
   - `ContentBlockType` (`src/nvim/api/prompt.rs`) - tests text/link content parsing from Lua dictionaries
   - `PromptContent` (`src/nvim/api/prompt.rs`) - tests single/multiple content parsing from Lua arrays

**Important:** Only test `FromObject` trait implementations (Lua→Rust conversions), not direct Rust type construction. Rust's type system already guarantees struct instantiation works. Proptest is valuable for:
- Testing `from_object()` methods that parse Lua dictionaries/arrays
- Validating that malformed Lua input produces proper errors
- Ensuring `unsafe` conversion code doesn't panic on edge cases

**Example of what NOT to test:**
```rust
// BAD: Just testing Rust struct construction
#[test]
fn test_create_text_block() {
    let content = ContentBlockType::Text { text: "hello".to_string() };
    // This tests nothing useful - Rust guarantees this works
}
```

**Example of what TO test:**
```rust
// GOOD: Testing FromObject conversion from Lua dictionary
#[test]
fn test_parse_text_from_lua_dict() {
    let dict = create_text_dict("hello");
    let obj = Object::from(dict);
    let result = ContentBlockType::from_object(obj);
    assert!(result.is_ok());
}
```

**Benefits of proptest for conversions:**
- Tests thousands of edge cases automatically
- Finds panic conditions in `unsafe` code blocks
- Validates that invalid inputs produce proper errors
- Catches regressions in parsing logic

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
