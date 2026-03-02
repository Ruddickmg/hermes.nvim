## Testing

### Do:
 - cover all code paths, including edge cases and error paths
 - compare actual values, use "asser_eq" or similar assertions to make sure the values in the output are as expected based on the input
 - Write integration tests for nvim specific code since it is a requirement of nvim-oxi. See docs for nxim-oxi testing [here](https://github.com/noib3/nvim-oxi#testing)
 - run tests to debug and verify they are working

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


This is a bad test because though it may check that the result exists (is not None), it doesn't verify that the value is correct. For example if we change variable "b" to 5, the test will pass, even though the result is different. Be as specific as possible while testing.

## Code

 - Use clean code and SOLID principles
