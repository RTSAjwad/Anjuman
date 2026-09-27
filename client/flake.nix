{
  description = "Anjuman — Crux-based cross-platform application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";

    # Rust toolchain: stable, with the wasm32 target and rust-analyzer.
    rust-overlay.url = "github:oxalica/rust-overlay";

    # Composable per-system helpers (devShells, etc.).
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];

      pkgs = import nixpkgs {
        inherit system overlays;
      };

      # Stable Rust toolchain with the WebAssembly target installed, so the
      # Leptos shell (and future WASM work) builds out of the box.
      rust-toolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = ["wasm32-unknown-unknown"];
        extensions = ["rust-src" "rust-analyzer" "clippy" "rustfmt"];
      };

      # Native build inputs for the Linux (libadwaita) shell. Present now so
      # the shell can be added without re-editing the flake.
      linux-native-inputs = with pkgs; [
        pkg-config
        gtk4
        libadwaita
        glib
      ];

      # BoltFFI CLI, built from crates.io (not yet in nixpkgs). Used to
      # generate Swift/Kotlin/C#/TypeScript bindings from `shared/boltffi.toml`.
      # Built with our rust-overlay toolchain because boltffi_cli needs
      # edition 2024, which nixpkgs' default cargo (1.82) doesn't support.
      boltffi-cli = let
        rust-platform = pkgs.makeRustPlatform {
          rustc = rust-toolchain;
          cargo = rust-toolchain;
        };
      in
        rust-platform.buildRustPackage rec {
          pname = "boltffi_cli";
          version = "0.30.1";

          src = pkgs.fetchCrate {
            inherit pname version;
            hash = "sha256-FtD5ZPq0tY+c30VE8H0Qx+VpB/RbdfVUPBUWwTHfgIQ=";
          };

          cargoHash = "sha256-cUPvAOMDA3RQDAus1W0SjR8i/btva5NhG5NRcnxxwbc=";

          # boltffi_cli's own test suite has environment-specific failures;
          # we only need the binary for binding generation.
          doCheck = false;
        };
    in {
      formatter = pkgs.alejandra;

      packages = {
        inherit boltffi-cli;
        default = boltffi-cli;
      };

      devShells = {
        default = pkgs.mkShell {
          name = "anjuman";

          packages = with pkgs; [
            rust-toolchain

            # Web (Leptos) shell tooling.
            trunk
            wasm-bindgen-cli
            wasm-pack
            binaryen

            # TypeScript type generation (facet) shells out to pnpm.
            pnpm
            nodejs

            # FFI bindings + type generation for native shells.
            boltffi-cli

            # General Rust workflow.
            cargo-watch
            cargo-edit
          ];

          nativeBuildInputs = linux-native-inputs;

          RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/library";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath linux-native-inputs;

          shellHook = ''
            echo "Anjuman dev shell (rust $(rustc --version | awk '{print $2}'))"
            echo "  wasm target:  $(rustup target list --installed 2>/dev/null | grep wasm32 || echo 'via toolchain')"
          '';
        };
      };
    });
}
