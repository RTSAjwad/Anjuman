// Web-only URL routing for the React shell.
//
// Mirrors the Leptos `router.rs`: the business route is core-owned
// (`ViewModel.route` + `selected_deck`), and the URL is a *projection* of it —
// never a second source of truth. Hash routing (`#/decks`, `#/cards`,
// `#/decks/42/study`) is used deliberately so a hard refresh always serves
// `index.html` under any static host (no history-rewrite rule needed).
//
// This is a presentation concern, so it stays shell-local (the `Event`/
// `ViewModel` shapes are unchanged — the shell forwards core events).

import type { CurrentRoute, Event } from "shared_types/app";
import {
  eventShowDeckList,
  eventShowCards,
  eventShowNotes,
  eventOpenDeck,
  eventOpenDeckOptions,
} from "shared_types/app";

// The `location.hash` value for the given core route. `selectedDeckId` is only
// needed for the two deck panels; other routes ignore it.
export function routeToHash(
  route: CurrentRoute,
  selectedDeckId: bigint | null,
): string {
  switch (route.kind) {
    case "DeckList":
      return "#/decks";
    case "Cards":
      return "#/cards";
    case "Notes":
      return "#/notes";
    case "Study":
      return selectedDeckId === null
        ? "#/decks"
        : `#/decks/${selectedDeckId}/study`;
    case "DeckOptions":
      return selectedDeckId === null
        ? "#/decks"
        : `#/decks/${selectedDeckId}/options`;
  }
}

// Parse a `location.hash` (leading `#` stripped) back into a core `Event`, for
// deep-link hydration and `hashchange` (back/forward) handling. Unknown/malformed
// hashes map to the deck list.
export function hashToEvent(hash: string): Event {
  const path = hash.replace(/^#/, "").replace(/^\//, "");
  const segments = path.split("/").filter((s) => s.length > 0);

  if (segments.length === 0 || segments[0] === "decks" && segments.length === 1) {
    return eventShowDeckList();
  }
  if (segments.length === 1 && segments[0] === "cards") {
    return eventShowCards();
  }
  if (segments.length === 1 && segments[0] === "notes") {
    return eventShowNotes();
  }
  if (
    segments.length === 3 &&
    segments[0] === "decks" &&
    segments[2] === "study"
  ) {
    const id = BigInt(segments[1]);
    return eventOpenDeck(id);
  }
  if (
    segments.length === 3 &&
    segments[0] === "decks" &&
    segments[2] === "options"
  ) {
    const id = BigInt(segments[1]);
    return eventOpenDeckOptions(id);
  }
  return eventShowDeckList();
}

export function currentHash(): string {
  return typeof window === "undefined" ? "" : window.location.hash;
}

export function setHash(hash: string): void {
  if (typeof window !== "undefined") {
    window.location.hash = hash;
  }
}
