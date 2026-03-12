default:
    echo "Just is running!"

e2e:
  cargo nextest run --manifest-path e2e/Cargo.toml

integration:
  cargo nextest run --test '*'
  
unit:
  cargo nextest run --lib
