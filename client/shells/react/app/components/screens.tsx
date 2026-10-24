// Screen components. The deck list is real (Phase 3); study, deck options, and
// the browser are still placeholders pending their port.

import { useCore } from "../lib/core-provider.client";
import { eventOpenDeck, eventOpenDeckOptions } from "shared_types/app";
import { DeckTree } from "./deck-tree";

export function DeckListScreen() {
  const { view, update } = useCore();

  return (
    <div className="h-full overflow-y-auto p-4">
      <DeckTree
        decks={view?.decks ?? []}
        error={view?.decks_error ?? null}
        onOpenDeck={(id) => update(eventOpenDeck(id))}
        onOpenOptions={(id) => update(eventOpenDeckOptions(id))}
      />
    </div>
  );
}

export function StudyScreen() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
      Study — coming in Phase 3.
    </div>
  );
}

export function DeckOptionsScreen() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
      Deck options — coming in Phase 3.
    </div>
  );
}

export function BrowserScreen() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
      Browser — coming in Phase 3.
    </div>
  );
}
