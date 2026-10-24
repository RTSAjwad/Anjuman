import { useEffect } from "react";
import type { Route } from "./+types/home";
import { useCore } from "../lib/core-provider.client";
import { eventRestoreSession } from "shared_types/app";

export function meta({}: Route.MetaArgs) {
  return [
    { title: "Anjuman" },
    { name: "description", content: "Anjuman — spaced repetition" },
  ];
}

export default function Home() {
  const { view, ready, update } = useCore();

  // Fire the session restore once the WASM core is ready (the core emits the
  // KeyValue effect — token lookup in localStorage — and, if a token exists, an
  // Http effect). Gated on `ready` because `update` throws before `initialize()`
  // resolves, and `CoreFfi` is only constructed once `shared_bg.wasm` is loaded.
  useEffect(() => {
    if (ready) {
      update(eventRestoreSession());
    }
  }, [ready, update]);

  if (view === null) {
    return <p>Loading…</p>;
  }

  return (
    <main className="container mx-auto p-4">
      <h1 className="text-xl font-semibold">Anjuman</h1>
      {view.authenticated ? (
        <p>
          Signed in as <strong>{view.display_name}</strong> ({view.email})
        </p>
      ) : (
        <p>Not signed in.</p>
      )}
      <p>
        Decks: <strong>{view.decks.length}</strong>
      </p>
      <p>
        Route: <code>{view.route.kind}</code>
      </p>
    </main>
  );
}
