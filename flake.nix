{
  description = "Anjuman — Anki-inspired spaced repetition platform (monorepo)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    # Rust toolchain (stable, with wasm target) via rust-overlay, shared across
    # both the client and the server so there is exactly one Rust in the shell.
    rust-overlay.url = "github:oxalica/rust-overlay";

    # Composable per-system helpers (devShells, packages, formatter, …).
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
      # Leptos shell (and future WASM work) builds out of the box. Used by both
      # the client and the server (the server just doesn't need the wasm
      # target, which is harmless to have present).
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

          cargoHash = "sha256-DdfG8Z1joV1ftyIw9UIF0GVYalxMC1gYZtxIWeF8O/E=";

          # boltffi_cli's own test suite has environment-specific failures;
          # we only need the binary for binding generation.
          doCheck = false;
        };

      # --- Client (Crux cross-platform app) dependencies ---
      client-packages = with pkgs; [
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

      # --- Server (Axum backend) dependencies ---
      server-packages = with pkgs; [
        rust-toolchain

        rust-analyzer
        sqlx-cli
        sqlite

        pkg-config
        openssl

        just
      ];

      # Environment & shell hook shared by the full dev shell.
      shell-env = {
        RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/library";
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath linux-native-inputs;
      };
    in {
      formatter = pkgs.alejandra;

      packages = {
        inherit boltffi-cli;
        default = boltffi-cli;
      };

      devShells = {
        # The full workspace shell: everything needed for both the client and
        # the server in one environment.
        default = pkgs.mkShell {
          name = "anjuman";

          packages = client-packages ++ server-packages;

          nativeBuildInputs = linux-native-inputs;

          RUST_SRC_PATH = shell-env.RUST_SRC_PATH;
          LD_LIBRARY_PATH = shell-env.LD_LIBRARY_PATH;

          shellHook = ''
            export DATABASE_URL=''${DATABASE_URL:-sqlite://platform.db}

            echo "Anjuman dev shell (rust $(rustc --version | awk '{print $2}'))"
            echo "  wasm target:  $(rustup target list --installed 2>/dev/null | grep wasm32 || echo 'via toolchain')"
          '';
        };

        # Client-only shell.
        client = pkgs.mkShell {
          name = "anjuman-client";

          packages = client-packages;

          nativeBuildInputs = linux-native-inputs;

          RUST_SRC_PATH = shell-env.RUST_SRC_PATH;
          LD_LIBRARY_PATH = shell-env.LD_LIBRARY_PATH;

          shellHook = ''
            echo "Anjuman client dev shell (rust $(rustc --version | awk '{print $2}'))"
          '';
        };

        # Server-only shell.
        server = pkgs.mkShell {
          name = "anjuman-server";

          packages = server-packages;

          shellHook = ''
            export DATABASE_URL=''${DATABASE_URL:-sqlite://platform.db}

            # Initialize a local SQLite database in WAL mode if one doesn't
            # exist yet (mirrors the previous server flake's convenience).
            if [ ! -f platform.db ]; then
              sqlite3 platform.db "PRAGMA journal_mode=WAL;"
            fi

            echo "Anjuman server dev shell (rust $(rustc --version | awk '{print $2}'))"
          '';
        };
      };
    });
}
