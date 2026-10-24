import { useEffect } from "react";
import type { Route } from "./+types/home";
import { useCore } from "../lib/core-provider.client";
import { eventRestoreSession } from "shared_types/app";
import { AppShell } from "../components/app-shell";

export function meta({}: Route.MetaArgs) {
  return [{ title: "Anjuman" }];
}

export default function Home() {
  const { ready, update } = useCore();

  // Fire the session restore once the WASM core is ready (the core emits the
  // KeyValue effect — token lookup in localStorage — and, if a token exists, an
  // Http effect). Gated on `ready` because `update` throws before `initialize()`.
  useEffect(() => {
    if (ready) {
      update(eventRestoreSession());
    }
  }, [ready, update]);

  return <AppShell />;
}
