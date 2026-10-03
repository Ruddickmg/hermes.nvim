{
  description = "Rust dev environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    shell-services = {
      url = "github:Rayzeq/shell-services/9912882b86aba66b4d0b95d6b2af1d4fbb3cc6e7";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-parts,
      shell-services,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      perSystem =
        { system, ... }:
        let
          pkgs = import nixpkgs {
            inherit system;
            config.allowUnfree = true;
            overlays = [
              (import rust-overlay)
              (final: prev: {
                lib = prev.lib // {
                  toJSON = builtins.toJSON;
                };
              })
            ];
          };
          shellServices = import shell-services { inherit pkgs; };
          toolChain = (
            pkgs.rust-bin.stable.latest.default.override {
              extensions = [
                "rust-src"
                "cargo"
                "rustc"
                "rust-analyzer"
                "llvm-tools"
                "clippy"
              ];
            }
          );

          sources = builtins.fromJSON (builtins.readFile ./nix/sources.json);

          binaryNameForSystem = {
            "x86_64-linux" = "libhermes-linux-x86_64.so";
            "aarch64-linux" = "libhermes-linux-aarch64.so";
            "x86_64-darwin" = "libhermes-macos-x86_64.dylib";
            "aarch64-darwin" = "libhermes-macos-aarch64.dylib";
            "x86_64-windows" = "libhermes-windows-x86_64.dll";
          };

          hermes-binary = pkgs.fetchurl {
            url = "https://github.com/Ruddickmg/hermes.nvim/releases/download/v${sources.version}/${binaryNameForSystem.${system}}";
            hash = (sources.${system}).hash;
          };
        in
        {
          packages.default = pkgs.vimUtils.buildVimPlugin {
            pname = "hermes-nvim";
            version = sources.version;
            src = inputs.self;
            nvimSkipModules = [ "hermes" ];
            postInstall = ''
              mkdir -p $out/lib
              cp ${hermes-binary} $out/lib/${binaryNameForSystem.${system}}
            '';
            passthru = {
              inherit hermes-binary;
            };
          };

          devShells.default = shellServices {
            nativeBuildInputs = with pkgs; [
              gcc
            ];

            RUST_SRC_PATH = "${toolChain}/lib/rustlib/src/rust/library";

            buildInputs = with pkgs; [
              toolChain
              glib
              clang
              sqlx-cli
              just
              lspmux
              opencode
              github-copilot-cli
              cargo-nextest
              cargo-llvm-cov
              cargo-deny
              cargo-sort
              lua51Packages.vusted
              nodejs
              pnpm
            ];

            shellHook = ''
              XDG_RUNTIME_DIR="''${XDG_RUNTIME_DIR:-''${TMPDIR:-/tmp}/hermes-runtime-$(id -u)}"
              mkdir -p "$XDG_RUNTIME_DIR/lspmux"
              XDG_CONFIG_HOME="''${XDG_CONFIG_HOME:-$HOME/.config}"
              mkdir -p "$XDG_CONFIG_HOME/lspmux"
              cfg="$XDG_CONFIG_HOME/lspmux/config.toml"
              if [ ! -f "$cfg" ]; then
                cat > "$cfg" <<EOF
            listen = "$XDG_RUNTIME_DIR/lspmux/lspmux.sock"
            connect = "$XDG_RUNTIME_DIR/lspmux/lspmux.sock"
            log_filters = "debug"
            EOF
              fi
            '';

            services = {
              lspmux = {
                start = [ "${pkgs.lspmux}/bin/lspmux" "server" ];
                env = {
                  PATH = pkgs.lib.makeBinPath [ toolChain pkgs.gcc ];
                };
              };
            };
          };
        };
    };
}
