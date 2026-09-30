# AGENTS.md

Guidance and authoritative references for working on this repository. Read this
first; it exists to save you from re-discovering version-specific APIs and
gotchas — and from trusting "latest" docs where the exact version matters.

For the *why* behind the design, see [`ARCHITECTURE.md`](./ARCHITECTURE.md).
This file is the *how* and *where to look*. For building a shell (Leptos now,
SwiftUI/WinUI/Compose/Libadwaita later), see [`SHELLS.md`](./SHELLS.md) — the
shared capability-implementation and effect-resolve patterns every shell follows.

---

## 1. What this repo is

A Crux-based cross-platform application: a shared Rust **core** (`shared/`) plus
thin **shells** (`shells/`). One shell is implemented today (Leptos, WASM);
SwiftUI/WinUI/Compose/Libadwaita are planned.

- `shared/` — the Crux core. `Model`/`Event`/`ViewModel`/`Effect` in
  `src/app.rs`, FFI in `src/ffi.rs`, typegen binary in `src/bin/codegen.rs`.
- `shells/leptos/` — the web front-end (CSR-only, `trunk`).
- `SCREENS_SUPPORT.md` — the client feature matrix (screens + deferred
  client-side behaviours) that stage-4 stories derive from.
- `flake.nix` — Nix dev shell (in the **repo root**; provisions Rust 1.98 +
  `wasm32` target, `trunk`, `boltffi_cli`, `pnpm`, `gtk4`/`libadwaita`, plus the
  server's `sqlx-cli`/`sqlite`).
- `shared/boltffi.toml` — BoltFFI native-binding config (apple/android/wasm/csharp).

The Axum server and the `anjuman_contracts` crate live in **sibling folders of
this monorepo** (`../server`, `../contracts`) — path dependencies, not separate
repos.

## 2. Build / run / test

Everything goes through the Nix dev shell (there is no global Rust on PATH):

```sh
nix develop                    # enter the dev shell
cargo build                    # full workspace
cargo test                     # core tests
cargo build --features codegen -p shared          # typegen binary
cargo run --features codegen -p shared --bin codegen -- --language swift --output-dir shells/swiftui/generated

cd shells/leptos && trunk build   # web shell (wasm)
cd shells/leptos && trunk serve   # dev server (CSR)
cd shared && boltffi generate swift   # native bindings (also: kotlin/csharp/typescript)
```

## 3. Version-pinned authoritative references

> Always use the **pinned** docs for the versions below; "latest" on docs.rs has
> already bitten this project (e.g. thaw/leptos version skew).

### Crux core

| Crate | Pinned | Docs |
|---|---|---|
| `crux_core` | **0.20.0** | https://docs.rs/crux_core/0.20.0/crux_core/ |
| `crux_macros` | 0.10.1 | https://docs.rs/crux_macros/0.10.1/ |
| `crux_http` | 0.20.0 | https://docs.rs/crux_http/0.20.0/crux_http/ |
| `crux_kv` | 0.14.0 | https://docs.rs/crux_kv/0.14.0/ |
| `crux_time` | 0.18.0 | https://docs.rs/crux_time/0.18.0/ |

- Crux book: https://redbadger.github.io/crux/ (note: thin on detail; the
  **examples repo** is the real ground truth).
- **Examples repo (authoritative, mirrors our exact patterns):**
  - counter (our base): https://github.com/redbadger/crux/tree/master/examples/counter
  - counter-http (HTTP capability): https://github.com/redbadger/crux/tree/master/examples/counter-http
  - Key files: `shared/src/ffi.rs`, `shared/src/bin/codegen.rs`,
    `shared/Cargo.toml`, `shared/boltffi.toml`, and the `web-leptos/` shell.

### BoltFFI

| Crate | Pinned | Notes |
|---|---|---|
| `boltffi` (lib) | **=0.30.1** | `#[boltffi::export]` attribute |
| `boltffi_cli` | 0.30.1 | the `boltffi` binary |

- Docs: https://boltffi.dev (getting started: https://boltffi.dev/docs/getting-started)
- Types/mappings (incl. `Vec<u8>` → `Data`/`ByteArray`/`byte[]`/`Uint8Array`): https://boltffi.dev/docs/types
- `boltffi.toml` schema: authoritatively defined in `boltffi_cli`'s source,
  `src/config/mod.rs` and `src/config/targets/*.rs` (fetch from crates.io or GitHub).

### Leptos

| Crate | Pinned | Docs |
|---|---|---|
| `leptos` | **0.8** (lock: 0.8.21) | https://docs.rs/leptos/0.8.21/leptos/ |

- API moved in 0.7 → `reactive_graph`; import from `leptos::prelude::*`.
- **Authoritative shell example:** https://github.com/redbadger/crux/tree/master/examples/counter/web-leptos

### thaw (component library)

| Crate | Pinned | Notes |
|---|---|---|
| `thaw` | **0.5.0-beta** | betas can churn |

- **Compatibility matrix (critical):** 0.3→Leptos 0.6, 0.4→Leptos 0.7,
  0.5-beta→Leptos 0.8. https://github.com/thaw-ui/thaw#readme
- **Authoritative usage template:** https://github.com/thaw-ui/thaw-template
  (see `start-trunk/` for the CSR pattern).

## 4. Version-specific gotchas (learned the hard way)

These are facts the docs do **not** make obvious, and that cost real debugging:

### Crux
- `crux_core` has **no `macros` feature**. Its features are: `bindgen`,
  `crux_macros`, `default`, `facet_typegen`, `testing`, `typegen`,
  `uniffi_compat_bindgen`. Enable `crux_macros` for `#[effect]`.
- Use **`#[effect(facet_typegen)]`**, not `#[effect(typegen)]`. The Facet path is
  the current one; `typegen` uses the deprecated `serde-generate` line.
- `#[effect(...)]` **generates its own serialization**; do NOT combine with a
  manual `#[derive(Serialize, Deserialize)]` on the `Effect` enum.
- The `facet_typegen` feature is gated on the **consumer crate** — make it the
  crate's default feature so the `Export` impl is always compiled (avoids
  feature-unification breaks across the workspace).
- `Bridge` (FFI) **defaults to `bincode`** (`BincodeFfiFormat`). Only relevant to
  non-Rust shells; Rust shells use the typed `Core` API instead.
- `AppTester` is **deprecated** (0.17+). Test `update` directly and inspect
  `Command::effects()`.
- Event/ViewModel need `Clone` (and `Copy`/`PartialEq`/`Eq` are convenient) for
  use in Leptos reactive signals.
- `crate-type` must be `["cdylib", "lib", "staticlib"]` — `cdylib`+`staticlib`
  for BoltFFI, `lib` so other Rust crates (the Leptos shell) can depend on it.
- Typegen is a **`codegen` binary inside `shared`** (not a separate crate) —
  `shared/src/bin/codegen.rs`, gated by the `codegen` feature.

### HTTP
- `crux_http` is a **separate crate**, not part of `crux_core`.
- The **shell performs the request** and passes opaque bytes back via
  `core.resolve(...)`; the **core deserializes** the body. Wire format is **JSON**
  (`expect_json`/`body_json`); `rkyv`/protobuf deferred (see ARCHITECTURE §5).
- Async in the core is expressed via `Command` + `then_send(Event::X)`, not
  `async`/`await` in `update`.
- An HTTP round-trip is **two events**: `Event::Load<X>` (emits
  `Effect::Http(request)`) and `Event::<X>Received(HttpResponse)` (fed back in,
  mutates `model`). Tests assert the emitted `Http` capability and the
  post-response `Model`/`ViewModel` separately (see `PLANNING.md` §3).

### Testing
- Test `update` directly (`Command::effects()`), **never** `AppTester`
  (deprecated 0.17+). Inject **canned** `HttpResponse`s; no live server/network.
- The counter tests in `shared/src/app.rs` are the canonical pattern; new
  stories add `#[cfg(test)] mod tests` next to their feature in `shared/`.

### Leptos / shell
- CSR-only: the shell is a **single binary target** (`src/main.rs` declares
  `mod app; mod core_link;`). A separate `[lib]` with the same name causes a
  Cargo artifact-name collision that `trunk` rejects.
- Rust shells use **typed `Core` directly**, NOT the serialized FFI `Bridge`:
  `core.process_event(event)` → effect loop → `core.view()`.
- `#[component]` fns return `impl IntoView`; reactive signals need `Clone` data.

### thaw
- wrap the tree in **`<ConfigProvider>`** to apply the theme.
- Prefer plain HTML `<div>` for layout (nesting thaw components that return
  opaque `impl IntoView` inside `Space`/`Card` triggers an `IntoFragment` error).

### Nix
- `boltffi_cli` needs **edition 2024**, so it must be built with the rust-overlay
  toolchain (1.98), not nixpkgs' default cargo (1.82). See the `boltffi-cli`
  derivation in the root `flake.nix`.
- Type generation (TypeScript) shells out to **`pnpm`**; it must be in the shell.

## 5. Settled architectural decisions (do not re-litigate)

- **Rust shells** (Leptos, Libadwaita) call the typed `Core<A>` API in-process.
  **Non-Rust shells** (Swift/Kotlin/C#) use the FFI `Bridge` + BoltFFI + Facet.
- **JSON over the wire** for core↔server. Wire optimization is an explicit TODO.
- FFI binding tool is **BoltFFI** (not uniffi).
- The Axum server and `anjuman_contracts` live in this monorepo as sibling
  folders (`../server`, `../contracts`) — but each is a **separate Cargo
  workspace** (see ARCHITECTURE.md §7), linked by path dependencies.

See `ARCHITECTURE.md` §8 for the full settled-vs-open list.

## 6. Open questions / TODO

- Wire-format optimization (post-profiling).
- `anjuman_contracts` repo/registry location.
- First full capability set (HTTP + key-value + time vs. `Render` only).
- Streaming / SSE.

## 7. Working conventions

- Centralize crate versions in the root `Cargo.toml` `[workspace.dependencies]`.
- Generated code is gitignored (`shells/*/generated/`, `shells/leptos/dist/`).
- No global `cargo`/`rustc` — always `nix develop` first.

### Stories target the core; shells are tracked, not duplicated

This app will grow multiple native shells (Leptos now; SwiftUI/WinUI/Compose/
Libadwaita later). To avoid one story per shell (and the drift that would cause):

- **Write the story once, against the core.** Acceptance criteria live in and
  test `shared/` (`update` + `effects()`). The behaviour is defined exactly
  once; every shell inherits it.
- **Each story carries a `Shell contract` checklist** — the *minimum* a shell
  must do to run the feature (render `ViewModel` field X, forward `Event` Y,
  execute `Effect` Z). This is a checklist, **not** a set of per-shell stories.
- **Shells are columns in `SCREENS_SUPPORT.md`**, not stories. A screen/behaviour's
  coverage per shell (`Leptos | SwiftUI | WinUI | Compose | Libadwaita`) is one
  ✅/❌/⚪ cell. When a new shell lands, you check off its column against the
  existing shell contracts — you don't author parallel stories.
- **Genuinely shell-specific behaviour** (form-factor: answer-key bindings,
  "spacebar also answers", theme) is **not** a core story. It is classified ⚪ in
  the `SCREENS_SUPPORT.md` matrix and tracked per-shell there. (For mobile
  conventions, consult AnkiDroid later rather than inventing our own.)

This mirrors `PLANNING.md` §3 "Core vs. shell test split": the core is the
single source of truth for behaviour; the shell contract + matrix record the
thin per-platform glue without duplicating the story.

### Agent ownership (core agent vs. shell agent)

This workspace is worked by two agents with disjoint write scopes:

- **Core agent** owns `client/shared/` plus the **workspace manifests**
  (`client/Cargo.toml`, `client/shared/Cargo.toml`). It also owns `contracts/`
  and `server/` in this repo, and is the **only** agent that edits the shared
  wire types in `contracts/`.
- **Shell agent** owns `client/shells/*` only. It adds dependencies to
  `client/shells/*/Cargo.toml`, and **flags** any `[workspace.dependencies]`
  addition (a new shared crate version is the core agent's call).

Neither agent edits the other's files. A new `Event`/`Effect`/`ViewModel` field
or a `contracts/` change a shell needs is a **carve-out** the core agent makes on
request — the shell agent asks rather than editing `shared/` or `contracts/`
itself.
