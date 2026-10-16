fn main() -> Result<(), nvim_oxi::tests::BuildError> {
    nvim_oxi::tests::build()?;

    // The nested `cargo build --target-dir <target>/nvim_oxi_tests/hermes-e2e`
    // run by `nvim_oxi::tests::build()` compiles every target of this package,
    // including the `mock-agent-stdio` bin. Mirror nvim_oxi's target-directory
    // computation so tests can locate that bin at compile time via
    // `HERMES_MOCK_AGENT_BIN`.
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            // `tests/e2e` sits two levels below the workspace root; fall back
            // to the default `<workspace>/target` layout.
            let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            manifest_dir
                .ancestors()
                .nth(2)
                .expect("CARGO_MANIFEST_DIR should be nested inside workspace")
                .join("target")
        });
    let profile = std::env::var("PROFILE").expect("cargo always sets PROFILE");
    let bin_path = target_dir
        .join("nvim_oxi_tests")
        .join("hermes-e2e")
        .join(&profile)
        .join(if cfg!(windows) {
            "mock-agent-stdio.exe"
        } else {
            "mock-agent-stdio"
        });
    println!(
        "cargo:rustc-env=HERMES_MOCK_AGENT_BIN={}",
        bin_path.display()
    );
    Ok(())
}
