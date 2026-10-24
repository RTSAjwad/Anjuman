import { startTransition, StrictMode } from "react";
import { hydrateRoot } from "react-dom/client";
import { HydratedRouter } from "react-router/dom";
import * as sharedWasm from "shared";

// Gate hydration on the WASM core being loaded, so the first event that reaches
// the core already has a ready `shared_bg.wasm` instance (mirrors the official
// Crux "React Router" web walk-through).
const wasmInitialized = (
  sharedWasm as unknown as { initialized: Promise<void> }
).initialized;

wasmInitialized.then(() => {
  startTransition(() => {
    hydrateRoot(
      document,
      <StrictMode>
        <HydratedRouter />
      </StrictMode>,
    );
  });
});
