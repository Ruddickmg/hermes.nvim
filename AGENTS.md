## Testing

The goal of testing is to give us confidence that our code is working as expected, and to catch any bugs or issues before they can be accidentally released. Keep that in mind when looking through code and writing tests. We want to be sure that if the tests pass, the application works as expected. We do not want undefined behavior.

### Do:
 - Read documentation for nxim-oxi testing [here](https://github.com/noib3/nvim-oxi#testing)
 - cover all code paths, including edge cases and error paths
 - compare actual values, use "asser_eq" or similar assertions to make sure the values in the output are as expected based on the input
 - run tests to debug and verify they are working
 - make each test for one thing, do not have multiple assertions in a single test case unless absolutely necessary. This makes it easier to identify what is failing when a test does fail, and also makes it easier to understand the purpose of the test.

Example of a good test
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
   assert_eq!(addition(a, b), Some(3));
}
```

This is a good test because we test agains the expected output for given input


### Don't:
 - write tests that check the existance of things like "is_some" when you can compare the actual value
 - delete tests in order to fix failures

Example of a bad version of the test above
```rust
#[test]
fn test_addition_function() {
   let a = 1;
   let b = 2;
   assert!(addition(a, b).is_some());
}
```

 - write tests that use equals operators instead of `assert_eq`. 

Example:
```rust 
// bad test
assert!("something" == "something")

// good test
assert_eq!("something", "something")
```


This is a bad test because though it may check that the result exists (is not None), it doesn't verify that the value is correct. For example if we change variable "b" to 5, the test will pass, even though the result is different. Be as specific as possible while testing.

## Code

 - read documentation on nvim-oxi here: https://docs.rs/nvim-oxi/latest/nvim_oxi/
 - read documentaion for the agent_client_protocol rust sdk here: https://docs.rs/agent-client-protocol-schema/latest/agent_client_protocol_schema/index.html
 - read documentation on the agent client protocol here: https://agentclientprotocol.com/get-started/introduction
 - Use clean code and SOLID principles
