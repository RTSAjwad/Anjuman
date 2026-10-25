# Anjuman — Crux cross-platform application

A cross-platform application with a shared Rust **core** and first-class native
shells on five platforms (SwiftUI, WinUI, Jetpack Compose, Libadwaita, and a
React web shell).

See [`ARCHITECTURE.md`](../docs/client/ARCHITECTURE.md) for the full architecture.

## Layout

```
Cargo.toml          # [workspace] + [workspace.dependencies] (central version pinning)
shared/             # The Crux core (Model / Event / ViewModel / Effect + FFI + typegen)
shells/react/       # React (TypeScript) web shell
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
- `wasm-bindgen-cli` (used by `boltffi pack wasm` for the React shell's WASM bindings)
- `pnpm` + `node` + `typescript` (React shell + TypeScript generation)
- `gtk4` / `libadwaita` (future Libadwaita shell)

## Building

```sh
# Core + tests
cargo test

# Full workspace (native targets)
cargo build

# WASM core (for the React shell's boltffi pack wasm step)
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
cd shells/react
pnpm install   # first time only
pnpm run dev
```

Then open the URL Vite prints (default `http://127.0.0.1:5173`). Or from the
repo root, `nix run .#client` does the same.

## How it works

The **core** (`shared/`) is a pure-Rust, Elm-style app (`Model`, `Event`,
`ViewModel`, `Effect`). It exposes an FFI surface (`shared/src/ffi.rs`) via
`crux_core::Bridge`, annotated with `#[boltffi::export]`. Shells drive it by
sending serialized events and reading the serialized view model. BoltFFI
(`shared/boltffi.toml`) generates the Swift/Kotlin/C#/TypeScript bindings that
call this surface, and the `codegen` binary (`shared/src/bin/codegen.rs`)
generates the `Event`/`ViewModel`/effect payload types each shell needs.

For the React shell (the only implemented shell today), this is done in
`shells/react`, which drives the **FFI `Bridge`** (`CoreFfi`) via generated
BoltFFI bindings — `update`/`resolve`/`view` with `bincode` serialization — and
reads the resulting `ViewModel` into React state. This is the non-Rust-shell
path (same one SwiftUI/Kotlin/C# share). A future Rust shell (Libadwaita) would
instead drive the **typed `Core<Anjuman>` API** directly (`process_event` /
`view`) — no serialization — which is simpler and faster.
