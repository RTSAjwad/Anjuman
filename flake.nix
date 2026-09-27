{
  description = "Anjuman — Anki-inspired spaced repetition platform (monorepo)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    # Rust toolchain (stable, with wasm target) via rust-overlay, shared across
    # both the client and the server so there is exactly one Rust in the shell.
    rust-overlay.url = "github:oxalica/rust-overlay";

    # flake-parts is the structural framework (used by services-flake /
    # process-compose-flake).
    flake-parts.url = "github:hercules-ci/flake-parts";

    # Development services (Postgres) via flake-parts.
    process-compose-flake.url = "github:Platonic-Systems/process-compose-flake";
    services-flake.url = "github:juspay/services-flake";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      imports = [
        inputs.process-compose-flake.flakeModule
      ];

      perSystem = {
        system,
        ...
      }: let
        # Apply the rust-overlay to our pkgs so `pkgs.rust-bin` is available.
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        rust-toolchain =
          pkgs.rust-bin.stable.latest.default.override {
            targets = ["wasm32-unknown-unknown"];
            extensions = ["rust-src" "rust-analyzer" "clippy" "rustfmt"];
          };

        # Native build inputs for the Linux (libadwaita) shell.
        linux-native-inputs = with pkgs; [
          pkg-config
          gtk4
          libadwaita
          glib
        ];

        # BoltFFI CLI, built from crates.io (not yet in nixpkgs). Built with our
        # rust-overlay toolchain because boltffi_cli needs edition 2024.
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
          postgresql

          pkg-config
          openssl

          just
        ];
      in {
        formatter = pkgs.alejandra;

        packages = {
          inherit boltffi-cli;
          default = boltffi-cli;
        };

        devShells = {
          default = pkgs.mkShell {
            name = "anjuman";

            packages = client-packages ++ server-packages;

            nativeBuildInputs = linux-native-inputs;

            RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/library";
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath linux-native-inputs;

            shellHook = ''
              export DATABASE_URL=''${DATABASE_URL:-postgres://127.0.0.1:5432/anjuman}

              echo "Anjuman dev shell (rust $(rustc --version | awk '{print $2}'))"
              echo "  wasm target:  $(rustup target list --installed 2>/dev/null | grep wasm32 || echo 'via toolchain')"
              echo "  Postgres:     run 'nix run .#anjuman' in another terminal"
            '';
          };

          client = pkgs.mkShell {
            name = "anjuman-client";

            packages = client-packages;

            nativeBuildInputs = linux-native-inputs;

            RUST_SRC_PATH = "${rust-toolchain}/lib/rustlib/src/rust/library";
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath linux-native-inputs;

            shellHook = ''
              echo "Anjuman client dev shell (rust $(rustc --version | awk '{print $2}'))"
            '';
          };

          server = pkgs.mkShell {
            name = "anjuman-server";

            packages = server-packages;

            shellHook = ''
              export DATABASE_URL=''${DATABASE_URL:-postgres://127.0.0.1:5432/anjuman}

              echo "Anjuman server dev shell (rust $(rustc --version | awk '{print $2}'))"
              echo "  Postgres: run 'nix run .#anjuman' in another terminal"
            '';
          };
        };

        # The "anjuman" process-compose service group — brings up local
        # development services (Postgres). Run it with `nix run .#anjuman`.
        process-compose."anjuman" = {
          imports = [
            inputs.services-flake.processComposeModules.default
          ];

          services.postgres."pg" = {
            enable = true;
            initialDatabases = [
              {name = "anjuman";}
            ];
          };
        };
      };
    };
}
