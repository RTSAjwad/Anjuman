import { useEffect, useRef, useState } from "react";
import type { CurrentRoute, Event } from "shared_types/app";
import { currentHash, hashToEvent, routeToHash, setHash } from "./router";

// Web-only hash-router glue (Caveat A = A2): mirrors the core-owned
// `ViewModel.route` into `location.hash`, turns browser back/forward
// (`hashchange`) back into `Show*`/`OpenDeck*` events, and hydrates a deep link
// once authenticated + decks are loaded. The core stays the single source of
// truth; the URL is a projection, never read directly during normal operation.
export function useHashRouter(args: {
  route: CurrentRoute;
  authenticated: boolean;
  decksLoaded: boolean;
  selectedDeckId: bigint | null;
  onEvent: (event: Event) => void;
}) {
  const { route, authenticated, decksLoaded, selectedDeckId, onEvent } = args;

  // Guards the mirror -> hashchange round-trip (skip re-emitting the event we
  // just wrote ourselves). User back/forward leaves it false → forwarded.
  const suppress = useRef(false);
  // Deep-link hydration fires exactly once (after auth + decks).
  const [hydrated, setHydrated] = useState(false);

  // Mirror route -> hash.
  useEffect(() => {
    if (!hydrated) return;
    const target = routeToHash(route, selectedDeckId);
    const current = currentHash();
    if (current !== target) {
      suppress.current = true;
      setHash(target);
    }
  }, [hydrated, route, selectedDeckId]);

  // Register hashchange once.
  useEffect(() => {
    const handler = () => {
      if (suppress.current) {
        suppress.current = false;
        return;
      }
      onEvent(hashToEvent(currentHash()));
    };
    window.addEventListener("hashchange", handler);
    return () => window.removeEventListener("hashchange", handler);
  }, [onEvent]);

  // Deep-link / refresh hydration.
  useEffect(() => {
    if (hydrated) return;
    if (!authenticated || !decksLoaded) return;
    const hash = currentHash();
    setHydrated(true);
    if (hash === "" || hash === "#" || hash === "#/") {
      suppress.current = true;
      setHash("#/decks");
    } else {
      onEvent(hashToEvent(hash));
    }
  }, [hydrated, authenticated, decksLoaded, onEvent]);
}
