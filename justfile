default:
    echo "Just is running!"

silent_e2e:
  OPENCODE_PERMISSION='{"*": "ask"}' cargo nextest run --manifest-path tests/e2e/Cargo.toml

e2e:
  cargo llvm-cov nextest -E 'package(hermes-e2e)' --no-capture --manifest-path tests/e2e/Cargo.toml --all-features --workspace --lcov --output-path e2e.coverage.info --ignore-filename-regex 'tests/.*' --no-fail-fast

integration:
  cargo llvm-cov nextest -E 'package(hermes-integration)' --manifest-path tests/integration/Cargo.toml --all-features --workspace --lcov --output-path integration.coverage.info --ignore-filename-regex 'tests/.*' --no-fail-fast --no-capture
  
unit:
  cargo nextest run --lib --no-fail-fast

# Run Lua tests using vusted
lua:
  vusted -e "package.path = package.path .. ';./tests/lua/?.lua'; package.path = package.path .. ';./tests/lua/spec/?.lua'; package.path = package.path .. ';./lua/?.lua'; package.path = package.path .. ';./lua/?/init.lua'" tests/lua/spec/

# Run specific Lua test file (e.g., just test-lua-file tests/lua/spec/platform_spec.lua)
lua-file FILE:
  vusted -e "package.path = package.path .. ';./tests/lua/?.lua'; package.path = package.path .. ';./tests/lua/spec/?.lua'; package.path = package.path .. ';./lua/?.lua'; package.path = package.path .. ';./lua/?/init.lua'" {{FILE}}
