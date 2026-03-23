default:
    echo "Just is running!"

silent_e2e:
  OPENCODE_PERMISSION='{"*": "ask"}' cargo nextest run --manifest-path tests/e2e/Cargo.toml

e2e:
  cargo llvm-cov nextest --no-capture --manifest-path tests/e2e/Cargo.toml --all-features --workspace --lcov --output-path e2e.coverage.info --ignore-filename-regex 'tests/.*' --no-fail-fast

integration:
  cargo llvm-cov nextest --manifest-path tests/integration/Cargo.toml --all-features --workspace --lcov --output-path integration.coverage.info --ignore-filename-regex 'tests/.*' --no-fail-fast --no-capture
  
unit:
  cargo nextest run --lib --no-fail-fast
