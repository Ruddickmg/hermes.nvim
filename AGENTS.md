## Codebase

Hermes is an interface between [Neovim](https://neovim.io/) and [ACP](https://agentclientprotocol.com/).

### Architecture

The architecture separates Neovim logic from Rust ACP interactions:

- **Directory Structure:**
  - `src/acp`: Contains all direct interactions with the ACP SDK (code in this directory is multi threaded as it needs to communicate back and forth with the main neovim thread).
  - `src/nvim`: Contains Neovim-specific bindings and logic (code in this direcory is generally single threaded as it must stay on the same thread as Neovim).
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

## Thread Safety in Rust

This section documents Rust's thread-safety types and how they are used in the Hermes project.

### The Send and Sync Traits

Rust's concurrency safety is enforced at compile time through two marker traits:

- **`Send`**: A type is `Send` if it is safe to transfer ownership to another thread. This means you can move a value of this type to a different thread without causing data races.
- **`Sync`**: A type is `Sync` if it is safe to share between multiple threads. This means `&T` (an immutable reference to `T`) is `Send`, so multiple threads can safely hold references to the same value.

**Key Insight**: Most Rust types automatically implement both `Send` and `Sync`. However, types that enable unsynchronized shared mutable state are explicitly **not** `Send` or `Sync`:
- `Rc` is neither `Send` nor `Sync`
- `RefCell` and `Cell` are `Send` but not `Sync`
- `Mutex` and `RwLock` are both `Send` and `Sync`

### Single-Threaded Types: `Rc`, `RefCell`, `Cell`

These types are designed for use within a single thread only. They are **not thread-safe** but have **zero runtime overhead** for thread synchronization.

#### `Rc<T>` - Reference Counting

Use `Rc` when you need **multiple owners** of the same data within a single thread.

```rust
use std::rc::Rc;

// CORRECT: Using Rc for shared ownership on a single thread
let data: Rc<String> = Rc::new(String::from("shared"));
let data2: Rc<String> = Rc::clone(&data);  // Increments refcount

// The compiler WILL NOT LET you send Rc across threads:
// std::thread::spawn(move || {
//     println!("{}", data);  // ERROR: Rc<String> is not Send
// });
```

**Why Rc is NOT Send/Sync**: `Rc` uses a non-atomic reference counter. If two threads cloned the same `Rc` simultaneously, the reference count could be corrupted, leading to memory leaks or use-after-free bugs. The compiler prevents this at compile time.

**Performance**: `Rc` uses regular increment/decrement operations - extremely fast with no synchronization overhead.

#### `RefCell<T>` - Runtime Borrow Checking

Use `RefCell` when you need **interior mutability** (mutating data through an immutable reference) within a single thread.

```rust
use std::cell::RefCell;

// CORRECT: Using RefCell for single-threaded interior mutability
let data: RefCell<Vec<i32>> = RefCell::new(vec![1, 2, 3]);

// You can mutate through an immutable reference
data.borrow_mut().push(4);

// Runtime borrow checking (panics if rules violated at runtime)
let _borrow1 = data.borrow();  // OK
let _borrow2 = data.borrow();  // OK - multiple immutable borrows
// let _borrow3 = data.borrow_mut();  // Would panic! Already borrowed immutably
```

**Compile-Time vs Runtime Borrowing**: 
- Regular references (`&T`, `&mut T`) are checked at **compile time** by the borrow checker
- `RefCell` moves these checks to **runtime** using a counter to track active borrows
- If you violate the rules (e.g., mutable borrow while immutable borrows exist), the program **panics** instead of failing to compile

**When to use**: When you're certain your code follows borrowing rules but the compiler can't prove it (e.g., complex graph structures, callback patterns).

#### `Cell<T>` - Value-Based Interior Mutability

Use `Cell` for **simple Copy types** when you need to mutate without references.

```rust
use std::cell::Cell;

// CORRECT: Using Cell for simple Copy types
let counter: Cell<i32> = Cell::new(0);
counter.set(5);
let value = counter.get();  // Returns a copy (5)

// Cell only works with Copy types (integers, bools, etc.)
// Cell<String> would NOT compile - String is not Copy
```

**`Cell` vs `RefCell`**:
- `Cell`: Works by copying values in/out. No references allowed. Only for `Copy` types. Zero runtime checks.
- `RefCell`: Works by lending references. Runtime borrow checking. Works with any type.
- **Performance**: `Cell` is faster for `Copy` types (5-10x in some cases). Use `Cell` when possible.

**Common Pattern in Hermes**: `Rc<RefCell<T>>` provides multiple ownership + interior mutability on a single thread.

```rust
use std::rc::Rc;
use std::cell::RefCell;

// CORRECT: Shared mutable state on the main Neovim thread
let shared_data: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
let shared_data2 = Rc::clone(&shared_data);

// Both can mutate the same data
shared_data.borrow_mut().push("hello");
shared_data2.borrow_mut().push("world");
```

### Multi-Threaded Types: `Arc`, `Mutex`, `RwLock`

These types are designed for sharing data across multiple threads. They use atomic operations or locking mechanisms to ensure thread safety.

#### `Arc<T>` - Atomically Reference Counted

Use `Arc` when you need **shared ownership across multiple threads**.

```rust
use std::sync::Arc;
use std::thread;

// CORRECT: Using Arc to share data between threads
let data: Arc<String> = Arc::new(String::from("shared across threads"));

for i in 0..10 {
    let data_clone = Arc::clone(&data);
    thread::spawn(move || {
        println!("Thread {}: {}", i, data_clone);  // OK: Arc is Send + Sync
    });
}
```

**Why Arc IS Send/Sync**: `Arc` uses **atomic operations** (specifically, atomic increment/decrement) for its reference counter. This ensures that even when multiple threads clone or drop `Arc` instances simultaneously, the count remains correct.

**Performance Cost**: Atomic operations are more expensive than regular memory operations. Only use `Arc` when you actually need to share across threads. If staying on one thread, `Rc` is faster.

#### `Mutex<T>` - Mutual Exclusion

Use `Mutex` when you need **interior mutability across multiple threads**.

```rust
use std::sync::{Arc, Mutex};
use std::thread;

// CORRECT: Thread-safe shared mutable state
let counter: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));

let mut handles = vec![];
for _ in 0..10 {
    let counter_clone = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        let mut num = counter_clone.lock().unwrap();  // Acquire lock
        *num += 1;  // Mutate while holding lock
        // Lock automatically released when `num` goes out of scope
    });
    handles.push(handle);
}

// Wait for all threads
for handle in handles {
    handle.join().unwrap();
}

println!("Final count: {}", *counter.lock().unwrap());
```

**How it works**: 
- `Mutex` ensures only one thread can access the data at a time
- `lock()` returns a `MutexGuard` - a smart pointer that unlocks automatically when dropped
- If another thread has the lock, `lock()` blocks until it becomes available

**RefCell vs Mutex**:
- Both provide interior mutability
- `RefCell`: Single-threaded, runtime borrow checking, panics on violations
- `Mutex`: Multi-threaded, uses OS-level locking, blocks threads on contention

#### `RwLock<T>` - Read-Write Lock

Use `RwLock` when you have **many readers and few writers**.

```rust
use std::sync::{Arc, RwLock};

let data: Arc<RwLock<Vec<i32>>> = Arc::new(RwLock::new(vec![1, 2, 3]));

// Multiple readers allowed simultaneously
let read_guard1 = data.read().unwrap();
let read_guard2 = data.read().unwrap();
println!("{:?}", *read_guard1);

// Writer gets exclusive access (blocks until all readers/writers done)
// let mut write_guard = data.write().unwrap();  // Would block if readers active
```

**Mutex vs RwLock**:
- `Mutex`: Exclusive access only (1 reader OR 1 writer at a time)
- `RwLock`: Multiple readers OR 1 writer (no readers during writes)
- `RwLock` has more overhead but better for read-heavy workloads

### Summary Table

| Type | Thread-Safe | Interior Mutability | Performance | Use Case |
|------|------------|---------------------|-------------|----------|
| `Rc` | ❌ NO | ❌ NO | Fastest | Single-threaded shared ownership |
| `RefCell` | ❌ NO | ✅ YES (runtime) | Fast | Single-threaded interior mutability |
| `Cell` | ❌ NO | ✅ YES (value) | Fastest | Simple Copy types, single-threaded |
| `Arc` | ✅ YES | ❌ NO | Slower (atomic ops) | Multi-threaded shared ownership |
| `Mutex` | ✅ YES | ✅ YES (locks) | Slowest (blocking) | Multi-threaded interior mutability |
| `RwLock` | ✅ YES | ✅ YES (locks) | Slow | Read-heavy multi-threaded access |

### Hermes-Specific Patterns

#### On the Main Neovim Thread

The main thread uses `Rc<RefCell<T>>` for data that stays on that thread:

```rust
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

// Requests struct stays on main thread, uses Rc (not Arc)
pub struct Requests {
    responders: Rc<RefCell<HashMap<RequestId, Responder>>>,
}
```

**Why Rc here?** The `Requests` struct manages in-flight ACP requests. It only needs to be accessed from the main Neovim thread, so `Rc` is sufficient and faster than `Arc`.

#### Crossing Thread Boundaries

Data sent through mpsc channels uses `Arc<Mutex<T>>`:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

// Data that crosses thread boundaries needs Arc + Mutex
let shared_state: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
let state_for_thread = Arc::clone(&shared_state);

thread::spawn(move || {
    // Thread-safe mutation
    state_for_thread.lock().unwrap().push(Message::new());
});
```

**Key Rule**: Only use `Arc`/`Mutex` when data actually needs to be shared across threads. For single-threaded data, `Rc`/`RefCell` is faster and semantically clearer.

### Common Mistakes to Avoid

```rust
use std::rc::Rc;
use std::cell::RefCell;
use std::thread;

// ❌ WRONG: Trying to send Rc across threads
let data: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(vec![1, 2, 3]));
thread::spawn(move || {
    data.borrow_mut().push(4);  // COMPILE ERROR: Rc is not Send!
});

// ✅ CORRECT: Use Arc<Mutex<T>> for thread-safe sharing
use std::sync::{Arc, Mutex};
let data: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(vec![1, 2, 3]));
let data_clone = Arc::clone(&data);
thread::spawn(move || {
    data_clone.lock().unwrap().push(4);  // OK!
});
```

```rust
// ❌ WRONG: Unnecessary Arc on single thread
use std::sync::Arc;
let data: Arc<String> = Arc::new(String::from("hello"));
// You're paying atomic operation costs for no benefit!

// ✅ CORRECT: Use Rc when staying on one thread
use std::rc::Rc;
let data: Rc<String> = Rc::new(String::from("hello"));
// No atomic overhead, same functionality
```

```rust
use std::cell::Cell;

// ❌ WRONG: Cell with non-Copy type
let data: Cell<String> = Cell::new(String::from("hello"));
// COMPILE ERROR: String does not implement Copy

// ✅ CORRECT: Use RefCell for non-Copy types
use std::cell::RefCell;
let data: RefCell<String> = RefCell::new(String::from("hello"));
data.replace(String::from("world"));  // OK!
```

### References

- [Rust Book: Shared-State Concurrency](https://doc.rust-lang.org/book/ch16-03-shared-state.html)
- [Rust Book: Send and Sync Traits](https://doc.rust-lang.org/book/ch16-04-extensible-concurrency-sync-and-send.html)
- [Rustonomicon: Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- [std::rc::Rc documentation](https://doc.rust-lang.org/std/rc/struct.Rc.html)
- [std::sync::Arc documentation](https://doc.rust-lang.org/std/sync/struct.Arc.html)

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

### Following Instructions

**CRITICAL:** When you encounter a situation where your opinion or interpretation conflicts with an instruction in this document, you MUST:

1. **ASK for permission** to deviate from the instruction, OR
2. **FOLLOW THE INSTRUCTION** instead of your opinion

**Never act on contradictory opinions without explicit permission.** If an instruction seems wrong, unclear, or suboptimal, ask the user before proceeding rather than making assumptions.

**Example:** If AGENTS.md says "write comprehensive tests" and you believe tests should be consolidated, you must either:
- Ask: "The instructions say to write comprehensive tests, but I think they should be consolidated. Is that okay?"
- Follow: Write comprehensive tests as instructed

**Never consolidate, skip, or modify instructions based on your own judgment without asking first.**

### Code Style
Adhere to "Clean Code" patterns.
- **Design:** Apply SOLID principles where applicable.

### Cross-Platform Compatibility (Lua)

**CRITICAL:** All Lua code must work across Windows, Linux, and macOS. Never use platform-specific shell commands.

## Library Research First

**BEFORE implementing any functionality, ALWAYS research if a library or built-in function already exists.** This applies to ALL contexts - not just cross-platform issues.

### General Rule: Research Before Implementing

Whether working with file operations, HTTP requests, data structures, parsing, or any other functionality:
1. **Check if Neovim provides a built-in function**
2. **Check if a standard library (vim.uv, vim.fs, etc.) provides the function**
3. **Check if an existing module in the codebase already handles this**
4. **Only implement your own solution as a last resort**

**Why this matters:**
- Offloads maintenance and correctness to library maintainers
- Ensures cross-platform compatibility automatically
- Benefits from community testing and bug fixes
- Reduces code we need to maintain
- Prevents "reinventing the wheel"

#### **Example: File Operations**

❌ **DON'T create your own copy function:**
```lua
-- ❌ BAD: Reinventing the wheel
local function copy_file(src, dest)
  if platform.get_os() == "windows" then
    vim.fn.system({ "cmd", "/c", "copy", src, dest })
  else
    vim.fn.system({ "cp", src, dest })
  end
end
```

✅ **DO use the library function:**
```lua
-- ✅ GOOD: Uses existing cross-platform library
local uv = vim.uv or vim.loop
uv.fs_copyfile(src, dest)
```

#### **Cross-Platform APIs**

**Available cross-platform APIs:**
- **File operations:** `vim.uv.fs_copyfile()`, `vim.uv.fs_rename()`, `vim.uv.fs_mkdir()`, `vim.uv.fs_unlink()`, `vim.uv.fs_rmdir()`, `vim.uv.fs_stat()`
- **Directory operations:** `vim.fn.mkdir()`, `vim.fn.delete()`
- **Path operations:** `vim.fs.joinpath()`, `vim.fs.normalize()`, `vim.fs.dirname()`
- **HTTP requests:** Use `download.lua` module (supports curl, wget, PowerShell)

#### **Forbidden Platform-Specific Commands**

❌ **Never use these directly:**
- `cp`, `mv`, `rm`, `mkdir` (Unix shell commands)
- `chmod` (Unix-only permissions)
- `curl` or `wget` without Windows fallback
- Hardcoded paths with `/` or `\` separators

✅ **Use these instead:**
```lua
-- Copy file (cross-platform)
local uv = vim.uv or vim.loop
uv.fs_copyfile(src, dest)

-- Create directory (cross-platform)
vim.fn.mkdir(path, "p")  -- 'p' creates parent directories

-- Delete file (cross-platform)
uv.fs_unlink(path)

-- Make executable (Unix only, check platform)
if platform.get_os() ~= "windows" then
  vim.fn.system({ "chmod", "+x", path })  -- Only on Unix
end
```

#### **Example: HTTP Downloads**

The download module provides cross-platform HTTP support:
```lua
local download = require("hermes.download")
local success, err = download.download(url, dest_path)
-- Works on: Windows (PowerShell), Linux (curl/wget), macOS (curl)
```

#### **Example: File Copy**

Before (Unix-only):
```lua
-- ❌ BAD: Only works on Unix
vim.fn.system({ "cp", src, dest })
```

After (Cross-platform):
```lua
-- ✅ GOOD: Works on all platforms
local uv = vim.uv or vim.loop
local result, err = uv.fs_copyfile(src, dest)
```

## Testing

Tests ensure code reliability and prevent regression.

### **CRITICAL: Test Placement Rules**

**DO NOT** create tests in the wrong location. Follow these strict rules:

1. **Unit Tests** - For pure Rust functions with no Neovim dependencies:
   - **Location:** Inside `#[cfg(test)]` module in the same source file as the code being tested
   - **Macro:** Use `#[test]`, NEVER use `#[nvim_oxi::test]`
   - **Example:** Testing `get_random_element()` or `get_permission_prompt()` goes in `src/utilities/prompt.rs`

2. **Integration Tests** - For code that interacts with Neovim APIs (buffers, files, windows):
   - **Location:** `tests/integration/src/` directory
   - **Macro:** Must use `#[nvim_oxi::test]` (runs inside Neovim)
   - **Example:** Testing buffer manipulation, file operations, autocommands

3. **E2E Tests** - For full system flow testing:
   - **Location:** `tests/e2e/` directory
   - **Macro:** Must use `#[nvim_oxi::test]`
   - **Example:** Testing complete request-response cycles

**WRONG (what NOT to do):**
```rust
// tests/integration/src/utilities/prompt.rs
#[nvim_oxi::test]
fn test_get_random_element() {  // Pure function, doesn't need Neovim!
    let result = get_random_element(vec!["a", "b"]);  // This is a unit test!
    assert!(...);
}
```

**CORRECT:**
```rust
// src/utilities/prompt.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]  // Regular test, not nvim_oxi::test
    fn test_get_random_element() {
        let result = get_random_element(vec!["a", "b"]);
        assert!(...);
    }
}
```

**Guideline:** If a test doesn't call `nvim_oxi::api::*`, it's a **unit test** and belongs in the source file.

### Guidelines

- **Coverage:** Cover all code paths, including edge cases and error handling.
- **Assertions:**
  - Use `assert_eq!` to verify exact values.
  - Avoid `assert!` with boolean checks (e.g., `is_some()`) when the value itself can be verified.
- **Scope:** Each test should verify a single behavior or unit. Use only **one assertion** per test unless absolutely necessary.
  - `.expect()` calls and setup code don't count as assertions
  - If a test needs multiple assertions, split it into multiple tests with descriptive names
  - Each test name should clearly indicate what specific behavior it's testing
- **Debugging:** Run tests locally to verify fixes.

### One Assertion Per Test Rule

**CRITICAL:** Every test must contain exactly ONE assertion (e.g., `assert!()`, `assert_eq!()`). This ensures:
- Clear test failures that immediately identify which behavior failed
- Easy debugging without guessing which assertion failed
- Tests serve as living documentation of specific behaviors

### No Conditional Assertions Rule

**CRITICAL:** Assertions must NEVER be wrapped in conditional statements (`if`, `elseif`, `else`). Each test must have exactly ONE success path that always executes the assertion. This ensures:
- Every test actually validates the expected behavior
- No silent skips when conditions aren't met
- Clear failure messages when assertions fail
- Tests serve as reliable documentation

**WRONG (conditional assertion that may not execute):**
```lua
-- ❌ BAD: Assertion might not run if native is nil
if native then
    assert.is_function(native.setup)
end
```

**CORRECT (always executes without conditionals):**
```

**For platform-specific tests:** Create separate test files or use descriptive test names instead of conditionals:
```lua
-- ❌ BAD: Conditional assertion based on OS
if os == "linux" then
    assert.equals("so", ext)
elseif os == "macos" then
    assert.equals("dylib", ext)
end

-- ✅ GOOD: Separate tests for each platform
it("returns so extension on Linux", function()
    stub(platform, "get_os").returns("linux")
    assert.equals("so", platform.get_ext())
end)

it("returns dylib extension on macOS", function()
    stub(platform, "get_os").returns("macos")
    assert.equals("dylib", platform.get_ext())
end)
```

**For search/validation patterns:** Assert on the result directly instead of using flags:
```lua
-- ❌ BAD: Conditional flag with late assertion
local found = false
for _, item in ipairs(list) do
    if item == "expected" then
        found = true
        break
    end
end
assert.is_true(found)

-- ✅ GOOD: Direct assertion using table functions
assert.is_not_nil(vim.tbl_find(list, "expected"), "Should find expected item in list")
```

**WRONG (multiple assertions in one test):**
```rust
#[nvim_oxi::test]
fn test_buffer_updated() -> nvim_oxi::Result<()> {
    let buffer = setup_buffer();
    update_content(&buffer, "new content");
    
    // ❌ BAD: Two assertions in one test
    assert!(buffer.is_modified(), "Should be modified");  // First assertion
    assert_eq!(buffer.content(), "new content");        // Second assertion
    Ok(())
}
```

**CORRECT (split into separate tests):**
```rust
#[nvim_oxi::test]
fn buffer_marked_modified_after_update() -> nvim_oxi::Result<()> {
    let buffer = setup_buffer();
    update_content(&buffer, "new content");
    
    // ✅ GOOD: Exactly one assertion
    assert!(buffer.is_modified(), "Buffer should be marked as modified");
    Ok(())
}

#[nvim_oxi::test]
fn buffer_content_matches_update() -> nvim_oxi::Result<()> {
    let buffer = setup_buffer();
    update_content(&buffer, "new content");
    
    // ✅ GOOD: Exactly one assertion
    assert_eq!(buffer.content(), "new content");
    Ok(())
}
```

**When multiple related behaviors need testing:**
- Create a separate test for each behavior
- Use descriptive test names that clearly state what is being verified
- Helper functions can reduce code duplication in setup

**Comparing multiple values in one assertion:**
When you need to verify multiple related values (e.g., checking all elements of a collection), prefer comparing slices/arrays rather than individual element assertions. This maintains the single-assertion rule while still verifying all data.

```rust
// ❌ BAD: Multiple assertions for individual elements
assert_eq!(actual_lines.len(), 3);
assert_eq!(actual_lines[0], "line2");
assert_eq!(actual_lines[1], "line3");
assert_eq!(actual_lines[2], "line4");

// ✅ GOOD: Single assertion comparing slices
assert_eq!(actual_lines.as_slice(), &["line2", "line3", "line4"]);
```

**When to use this pattern:**
- Verifying all elements in a collection match expected values
- Checking multiple fields of a struct that form a logical unit
- Comparing ordered sequences of data

**When NOT to use this pattern:**
- Each element represents a different behavior (should be separate tests)
- Different error conditions need separate verification
- The assertion would be too complex to understand at a glance
```rust
fn setup_write_request(path: &Path, content: &str) -> WriteTextFileRequest {
    WriteTextFileRequest::new(SessionId::from("test-session"), path, content)
}

#[nvim_oxi::test]
fn write_request_buffer_marked_modified() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test.txt").unwrap();
    let requests = create_requests();
    let (sender, mut receiver) = oneshot::channel();
    let responder = Responder::WriteFileResponse(
        sender,
        setup_write_request(temp_file.path(), "content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    requests.default_response(&request_id, serde_json::Value::Null)?;
    
    // Single assertion: verifies buffer modification
    assert!(is_buffer_modified(&temp_file), "Buffer should be marked as modified");
    Ok(())
}

#[nvim_oxi::test]
fn write_request_response_sent() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test.txt").unwrap();
    let requests = create_requests();
    let (sender, mut receiver) = oneshot::channel();
    let responder = Responder::WriteFileResponse(
        sender,
        setup_write_request(temp_file.path(), "content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    requests.default_response(&request_id, serde_json::Value::Null)?;
    
    // Single assertion: verifies response was sent
    assert!(receiver.try_recv().is_ok(), "Should receive success response");
    Ok(())
}
```

### Test Redundancy

Aim for the **minimum number of tests that cover all code paths**. Avoid testing the same logic multiple times.

**⚠️ CRITICAL: All tests must test application code**

Every test should verify behavior from the codebase being tested. Do NOT write tests that only exercise Rust language features, standard library functions, or external crate functionality without involving your application logic.

**Examples of redundancy to avoid:**

- **Language features:** Do not test Rust's built-in functionality (e.g., auto-derived traits like `PartialEq`, `Clone`, `Debug`, field access, Arc/Mutex usage). Assume the Rust compiler and standard library work correctly. Only test your own logic and custom trait implementations. Reading a field from a struct through an Arc/Mutex is standard Rust - don't test it.
  
  **Specifically DO NOT write tests like:**
  ```rust
  #[nvim_oxi::test]
  fn handler_is_cloneable() {  // ❌ BAD - testing #[derive(Clone)]
      let handler = create_handler();
      let _cloned = handler.clone();
      assert!(true);
  }
  
  // ❌ BAD - Creating test-only structs that reimplement production code
  struct TestHandler { state: Arc<Mutex<PluginState>> }
  impl TestHandler {
      fn can_write(&self) -> bool {
          // This reimplements the production code - NOT testing it!
          self.state.lock().await.permissions.fs_write_access
      }
  }
  #[test]
  fn test_can_write() {
      let handler = TestHandler::new();
      assert!(handler.can_write()); // Tests YOUR reimplementation, not the real Handler
  }
  ```
  
  **Why this is wrong:** When you create a `TestHandler` struct with methods that mirror the production `Handler` methods, you're testing YOUR reimplementation, not the actual production code. This gives false coverage numbers and doesn't verify the real behavior.
  
  **DO write tests like:**
  ```rust
  // ✅ GOOD - Test the actual Handler implementation
  #[test]
  fn test_handler_can_write() {
      let handler = create_real_handler(); // Use the actual Handler::new() or similar
      assert!(handler.can_write());
  }
  ```
  
  The `#[derive(Clone)]` macro is provided by Rust and guaranteed to work. Testing it wastes time and creates maintenance burden.
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

**What makes a proper integration test:**
Integration tests should verify that components actually **integrate** with Neovim, not just that API calls succeed. Examples:

- **Good**: Spawn a background thread that sends data via `NvimHandler`, then use `wait_for()` to verify the callback executed on Neovim's main thread
- **Bad**: Just verifying that `handler.blocking_send()` returns `Ok(())` - this doesn't test the integration

**Pattern for testing async/callback code:**
```rust
#[nvim_oxi::test]
fn cross_thread_communication_works() -> nvim_oxi::Result<()> {
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();
    
    let handler = NvimHandler::initialize(move |data| {
        received_clone.lock().unwrap().push(data);
        Ok(())
    })?;
    
    // Spawn thread that sends data
    std::thread::spawn(move || {
        handler.blocking_send("test".to_string()).unwrap();
    });
    
    // Wait for callback to execute on Neovim main thread
    let received_data = wait_for(
        || received.lock().unwrap().len() == 1,
        Duration::from_millis(500),
    );
    
    assert!(received_data, "Data should reach callback from spawned thread");
    Ok(())
}
```

**Guideline**: E2E tests verify that "the system works together", unit tests verify that "each component works correctly", integration tests verify that "Neovim-interacting components work correctly". Keep E2E tests minimal and focused on integration points.

### Integration Test Best Practices

**Do NOT test default values in integration tests.** Default values are hard-coded constants with no conditional logic. Testing them provides no value and creates maintenance burden.

**Examples of what NOT to test:**
```rust
// ❌ BAD - Testing a hard-coded default
#[nvim_oxi::test]
fn test_can_write_returns_true_by_default() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    assert!(handler.can_write().await); // Just testing that true == true
    Ok(())
}
```

**DO test:**
- Logic branches (what happens when value is changed from default)
- State mutations
- Integration with Neovim APIs
- Error conditions
- Side effects

**Example of good integration test:**
```rust
// ✅ GOOD - Tests actual behavior change
#[nvim_oxi::test]
fn test_write_file_creates_buffer() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let result = handler.write_file("test.txt", "content").await;
    assert!(nvim_oxi::api::get_current_buf().is_ok());
    Ok(())
}
```

### Coverage Requirements

**Lua code must maintain 80% test coverage.** This requirement applies to all Lua files in `lua/hermes/`.

- **Current coverage target:** 80%
- **Coverage tool:** luacov (as used in CI)
- **Coverage scope:** All files in `lua/hermes/` (configured in `tests/lua/.luacov`)

**Running Lua Coverage Locally:**

Use the same tooling as CI:

```bash
# Install dependencies (same as CI)
eval $(luarocks path --lua-version 5.1 --bin)
luarocks --lua-version=5.1 install vusted --local
luarocks --lua-version=5.1 install luacov --local
luarocks --lua-version=5.1 install luacov-reporter-lcov --local

# Run tests with coverage
eval $(luarocks path --lua-version 5.1 --bin)
vusted --coverage -e "package.path = package.path .. ';./tests/lua/?.lua'; package.path = package.path .. ';./tests/lua/spec/?.lua'; package.path = package.path .. ';./lua/?.lua'; package.path = package.path .. ';./lua/?/init.lua'" tests/lua/spec/

# Generate LCOV report
luacov -c tests/lua/.luacov -r lcov

# View human-readable report
luacov -c tests/lua/.luacov
```

**Understanding Coverage:**
- luacov generates `luacov.stats.out` (raw stats) and `luacov.report.out` (human-readable)
- Coverage inside `vim.schedule()` callbacks is not tracked (known limitation)
- Focus on testing sync code paths and state management

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

### Coverage Analysis

Use `cargo-llvm-cov` to generate test coverage reports:

```bash
# Generate HTML coverage report
cargo llvm-cov --bins --lib --all-features --workspace --html --ignore-filename-regex 'tests/.*'

# Generate summary only
cargo llvm-cov --summary-only

# Generate LCOV format for CI integration
cargo llvm-cov --lcov --output-path coverage.lcov
```

**Note:** Integration tests run in a separate Neovim process and do not contribute to `cargo llvm-cov` coverage metrics. Coverage reports only reflect unit test coverage.

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

## Agent Behavior

### Don't do things you are not asked to Do

**When asked to debug or investigate a problem:**
- **DO:** Run the code, reproduce the issue, analyze the logs/errors, and **report back what you found**
- **DO NOT:** Jump straight into fixing the code before reporting your findings
- **DO NOT:** Make changes unless explicitly asked to implement a solution

**Example:**
- **User:** "Debug the tests and figure out why they're failing"
- **WRONG:** Immediately modifying code to fix the issue
- **RIGHT:** Run the tests, identify the root cause, and report: "The tests are failing because X. The issue is at line Y. Do you want me to fix it?"

**Why this matters:**
- The user may want to understand the problem before deciding on a fix
- There may be multiple solutions and the user wants to choose
- Premature fixes may not align with the user's architectural vision
- Wastes time if the user rejects your unapproved solution

**Rule of thumb:** When the user says "debug", "investigate", "find out", or "look into" - they want INFORMATION, not CHANGES. Only implement when explicitly asked or given permission.
