default:
    echo "Just is running!"

silent_e2e:
  OPENCODE_PERMISSION='{"*": "ask"}' cargo nextest run --manifest-path tests/e2e/Cargo.toml

e2e:
  OPENCODE_PERMISSION='{"*": "ask"}' cargo nextest run --manifest-path tests/e2e/Cargo.toml --no-capture

e2e_check:
  OPENCODE_PERMISSION='{"*": "ask"}' cargo nextest run can_chose_a_response_to_a_permission_request --manifest-path tests/e2e/Cargo.toml --no-capture

silent_integration:
  cargo nextest run --manifest-path tests/integration/Cargo.toml

integration:
  cargo nextest run --manifest-path tests/integration/Cargo.toml --no-capture
  
unit:
  cargo nextest run --lib
