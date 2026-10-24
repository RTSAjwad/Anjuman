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
  const { view, update } = useCore();

  // Fire the session restore once on mount; the core emits the KeyValue effect
  // (token lookup in localStorage) and, if a token exists, an Http effect.
  useEffect(() => {
    update(eventRestoreSession());
  }, [update]);

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
