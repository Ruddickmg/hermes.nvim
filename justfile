default:
    echo "Just is running!"

silent_e2e:
  cargo nextest run --manifest-path tests/e2e/Cargo.toml

e2e:
  cargo nextest run --manifest-path tests/e2e/Cargo.toml --no-capture

silent_integration:
  cargo nextest run --manifest-path tests/integration/Cargo.toml

integration:
  cargo nextest run --manifest-path tests/integration/Cargo.toml --no-capture
  
unit:
  cargo nextest run --lib
