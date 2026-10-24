# Carve-out (shell agent → core agent): React web shell — WASM + toolchain wiring

**From:** shell agent
**To:** core agent
**Status:** pending
**Traceability:** `docs/client/REACT_SHELL_PLAN.md` §5 Phase 0, Caveat B.

## What I need (all core-agent scope — I do not touch `shared/`, `flake.nix`, or `boltffi.toml`)

I am standing up the new React web shell (`client/shells/react/`). It is a
**non-Rust** shell, so it drives the core over the FFI `Bridge` as WASM +
`bincode`. The core's `CoreFfi` (`shared/src/ffi.rs`) and its `[targets.wasm]`
BoltFFI target already exist — I need three small wiring changes to make them
consumable by the React shell.

### 1. Re-point the `[targets.wasm]` output to the React shell

In `client/shared/boltffi.toml`, the existing `[targets.wasm]` block outputs into
the Leptos shell (`shells/leptos/generated`). For the React shell I need it to
output to `shells/react/generated`, and the `typescript` sub-target's `pkg` to
resolve against the same base. Concretely:

```toml
[targets.wasm]
output = "../shells/react/generated"        # was "../shells/leptos/generated"
profile = "release"
artifact_path = "../target/wasm32-unknown-unknown/release/shared.wasm"

[targets.wasm.typescript]
output = "../shells/react/generated/pkg"     # was "../shells/leptos/generated/pkg"

[targets.wasm.npm]
package_name = "shared"
```

(If you'd rather keep a Leptos target and *add* a React target, that's fine too —
I just need React to get its own `generated/`. But there is no reason to keep
generating the wasm package into the Leptos shell, which runs in-process and
never consumes it — so re-pointing is the cleaner option. Your call.)

### 2. Add `typescript` to `client-packages` in `flake.nix`

`boltffi pack wasm` runs a final `tsc` typecheck of its generated bindings, but
`tsc` (the TypeScript compiler) is not on `$PATH` in the dev shell —
`typescript` is only present as a pnpm-vendored copy inside `codegen`'s output.
This makes `boltffi pack wasm` fail at the last step (`error: command failed:
tsc`) even though the WASM + bindings are generated correctly.

Add the `typescript` package (nixpkgs) to `client-packages` so `tsc` is
available. (Or, if there's a reason to avoid a global `tsc`, confirm the
alternative — but the upstream walk-through just expects `tsc` resolvable.)

### 3. (Nice-to-have) an npm-side dev entry

For the npm side I'll manage `pnpm`/`vite`/`vite-plugin-wasm` inside
`client/shells/react/` myself (shell scope). Nothing needed from you yet beyond
the two items above, unless you'd like to also add a `nix run .#react-client`
app/`devShell` convenience — which I'd welcome but don't strictly require for the
spike.

## Why this is a carve-out (not something I can self-serve)

Per `AGENTS.md` §2 write boundaries: `boltffi.toml` lives in `client/shared/`
and `flake.nix` is the repo-root manifest — both core-agent scope. The shell
agent owns `client/shells/*` only.

## Verification I've already done (so you don't have to re-derive)

- `cargo build --features codegen -p shared` ✅ builds the typegen binary.
- `codegen --language typescript` ✅ emits `shared_types` (typed events +
  `bincode`/`serde` serialization).
- `boltffi pack wasm` ✅ produces `shared_bg.wasm` + the `shared` npm package,
  then ✗ fails only at the `tsc` typecheck because `tsc` is absent from `$PATH`.

So items 1 and 2 are the complete, minimal unblock for the Phase 0 round-trip.
