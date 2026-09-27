# Anjuman — Crux cross-platform application

A cross-platform application with a shared Rust **core** and first-class native
shells on five platforms (SwiftUI, WinUI, Jetpack Compose, Libadwaita, and a
Leptos web shell).

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full architecture.

## Layout

```
Cargo.toml          # [workspace] + [workspace.dependencies] (central version pinning)
shared/             # The Crux core (Model / Event / ViewModel / Effect + FFI + typegen)
shells/leptos/      # Leptos (WASM) web shell
```

> Note: the Nix dev shell now lives in the **repository root** (`../flake.nix`),
> shared with the server and `anjuman_contracts` (single monorepo). Enter it from
> the repo root with `nix develop`.

The Axum server and the shared `anjuman_contracts` crate live in sibling
folders in the same monorepo (`../server`, `../contracts`).

## Prerequisites

Everything is provided by the Nix flake at the repository root. Install Nix
with flakes enabled, then from the repo root:

```sh
# Enter the full dev shell (automates if you use direnv)
nix develop

# Or the client-only shell
nix develop .#client
```

The dev shell provides:

- Rust toolchain (`rustc` 1.98) with the `wasm32-unknown-unknown` target
- `trunk`, `wasm-bindgen-cli` (Leptos web shell)
- `pnpm` + `node` (TypeScript type generation)
- `gtk4` / `libadwaita` (future Libadwaita shell)

## Building

```sh
# Core + tests
cargo test

# Full workspace (native targets)
cargo build

# Leptos shell (WASM)
cargo build --target wasm32-unknown-unknown

# Type generation — emits Swift/Kotlin/C#/TS types for each shell's
# generated/ directory, via the `codegen` binary in `shared`.
cargo run --features codegen -p shared --bin codegen -- \
  --language swift --output-dir shells/swiftui/generated
# ...repeat with --language kotlin|typescript and the matching shell dir

# Native bindings (BoltFFI) — generates from shared/boltffi.toml
cd shared
boltffi generate swift     # or: kotlin / csharp / typescript
boltffi pack all           # build native artifacts + bindings for all targets
```

## Running the web shell

```sh
cd shells/leptos
trunk serve
```

Then open the URL trunk prints (default `http://localhost:8080`).

## How it works

The **core** (`shared/`) is a pure-Rust, Elm-style app (`Model`, `Event`,
`ViewModel`, `Effect`). It exposes an FFI surface (`shared/src/ffi.rs`) via
`crux_core::Bridge`, annotated with `#[boltffi::export]`. Shells drive it by
sending serialized events and reading the serialized view model. BoltFFI
(`shared/boltffi.toml`) generates the Swift/Kotlin/C#/TypeScript bindings that
call this surface, and the `codegen` binary (`shared/src/bin/codegen.rs`)
generates the `Event`/`ViewModel`/effect payload types each shell needs.

For the Leptos shell, this is done in `shells/leptos/src/core_link.rs`, which
serializes events into bytes, calls `CoreFfi::update`, and reads the resulting
`ViewModel` back into a Leptos signal. This is the **same** FFI interface the
native shells (SwiftUI/Compose/WinUI) will use, so every shell exercises one
code path.
