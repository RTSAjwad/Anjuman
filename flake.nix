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

      perSystem = {system, ...}: let
        # Apply the rust-overlay to our pkgs so `pkgs.rust-bin` is available.
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        rust-toolchain = pkgs.rust-bin.stable.latest.default.override {
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

          # `cargo rustc` is invoked by `boltffi pack wasm` to emit binding
          # metadata; `shared` compiles to a `cdylib`/`staticlib` and needs a C
          # linker. The dev shell inherits one from `stdenv`, but the `nix run
          # .#client` app and the process-compose `frontend` process run pnpm
          # outside `mkShell`, so it must be listed explicitly.
          stdenv.cc

          # Web (React) shell tooling. `wasm-bindgen-cli` is required by
          # `boltffi pack wasm` (it generates the JS bindings glue for the
          # wasm target); the other three former Leptos-only tools are gone.
          wasm-bindgen-cli

          # TypeScript type generation (facet) shells out to pnpm.
          pnpm
          nodejs
          # `tsc` typechecks BoltFFI's generated TS bindings at the end of
          # `boltffi pack wasm` (React shell). Not otherwise on $PATH.
          typescript

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

          # `cc` is a build-time dependency of `ring` (via `jsonwebtoken`). The
          # dev shell inherits a C compiler from `stdenv`, but the bare `nix run
          # .#server` app and the process-compose `server` process run `cargo`
          # outside `mkShell`, so it must be listed explicitly.
          stdenv.cc

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

        # One-command runners for the individual dev processes (complementary to
        # the process-compose group below, which runs the whole stack at once).
        apps = {
          # `nix run .#server` — run the Axum backend (Postgres must already be
          # up; see `nix run .#anjuman`).
          server = {
            type = "app";
            program = let
              run = pkgs.writeShellApplication {
                name = "anjuman-server";
                runtimeInputs = server-packages;
                text = ''
                  export DATABASE_URL="''${DATABASE_URL:-postgres://127.0.0.1:5432/anjuman}"
                  exec cargo run --manifest-path server/Cargo.toml "$@"
                '';
              };
            in "${run}/bin/anjuman-server";
            meta.description = "Run the Anjuman Axum backend on 127.0.0.1:3000";
          };

          # `nix run .#client` — serve the React shell. The WASM core + generated
          # types are built separately (they only change when the core changes);
          # run `pnpm run build` (or `pnpm run wasm:build`/`types:build`) first,
          # then this dev server. Links node_modules if needed and starts the
          # React Router dev server on http://127.0.0.1:5173.
          client = {
            type = "app";
            program = let
              serve = pkgs.writeShellApplication {
                name = "anjuman-client";
                runtimeInputs = client-packages;
                text = ''
                  cd client/shells/react
                  if [ ! -d node_modules ]; then
                    pnpm install
                  fi
                  exec pnpm run dev "$@"
                '';
              };
            in "${serve}/bin/anjuman-client";
            meta.description = "Serve the Anjuman React shell (127.0.0.1:5173)";
          };
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

        # The "anjuman" process-compose service group — brings up the full
        # dev stack (Postgres + server + frontend). Run it with
        # `nix run .#anjuman`; process-compose shows a TUI with all three logs.
        process-compose."anjuman" = {
          imports = [
            inputs.services-flake.processComposeModules.default
          ];

          services.postgres."pg" = {
            enable = true;
            initialDatabases = [
              {name = "anjuman";}
              # Separate DB for `server/tests/` integration tests.
              {name = "anjuman_test";}
            ];
          };

          settings.processes = {
            # Run the Axum backend. Postgres (above) is started by this group, so
            # `DATABASE_URL` points at the group's own Postgres. `depends_on` the
            # Postgres readiness probe so the server's startup migrations don't
            # race an unready database.
            server = {
              command = pkgs.writeShellApplication {
                name = "anjuman-server";
                runtimeInputs = server-packages;
                text = ''
                  export DATABASE_URL="''${DATABASE_URL:-postgres://127.0.0.1:5432/anjuman}"
                  exec cargo run --manifest-path server/Cargo.toml
                '';
              };
              depends_on.pg.condition = "process_healthy";
            };

            # Serve the React shell (links node_modules if needed, then React
            # Router dev server on 127.0.0.1:5173). WASM + types are built
            # separately via `pnpm run build`.
            frontend = {
              command = pkgs.writeShellApplication {
                name = "anjuman-react";
                runtimeInputs = client-packages;
                text = ''
                  if [ ! -d node_modules ]; then
                    pnpm install
                  fi
                  exec pnpm run dev
                '';
              };
              working_dir = "client/shells/react";
            };
          };
        };
      };
    };
}
