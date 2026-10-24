// Screen components. The deck list, study, and deck options are real (Phase 3);
// the browser is still a placeholder pending its port.

import { useCore } from "../lib/core-provider.client";
import { eventOpenDeck, eventOpenDeckOptions } from "shared_types/app";
import { DeckTree } from "./deck-tree";
import { StudyScreen as StudyScreenView } from "./study";
import { DeckOptionsScreen as DeckOptionsView } from "./deck-options";

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
  const { view, update } = useCore();
  if (view === null) return null;

  return (
    <StudyScreenView
      card={view.current_card}
      counts={view.counts}
      busy={view.busy}
      studyError={view.study_error}
      actionError={view.card_action_error}
      update={update}
    />
  );
}

export function DeckOptionsScreen() {
  const { view, update } = useCore();
  if (view === null) return null;

  return (
    <DeckOptionsView
      deckId={view.selected_deck?.id ?? null}
      options={view.deck_options}
      error={view.deck_options_error}
      presets={view.presets}
      presetsError={view.presets_error}
      canManagePresets={view.permissions.can_manage_presets}
      update={update}
    />
  );
}

export function BrowserScreen() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
      Browser — coming in Phase 3.
    </div>
  );
}
