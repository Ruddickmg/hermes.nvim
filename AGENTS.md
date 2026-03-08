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

## Practices

- **Code Style:** Adhere to "Clean Code" patterns.
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

- **Shared validation logic:** If multiple types share the same initial validation (e.g., checking for a required "type" field), test it once for one type, not for every type.
- **Trivial accessors:** Enum variant extraction methods (e.g., `into_vec()` on a simple wrapper) may not need dedicated tests if the logic is obvious and covered indirectly.
- **Collection iteration:** If individual elements are thoroughly tested, you typically need only one test for the collection wrapper to verify iteration works.

**When to keep seemingly similar tests:**

- Different error branches (e.g., missing field A vs missing field B) each need their own test
- Different input formats (e.g., single item vs array) need separate tests
- Different code paths within the same function should each be tested

**Principle:** If removing a test would leave a code path uncovered, keep it. If multiple tests hit the exact same `if` branch with the same logic, consolidate them.

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
