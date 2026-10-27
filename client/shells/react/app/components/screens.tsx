// Screen components. The deck list, study, and deck options are real (Phase 3);
// the browser is still a placeholder pending its port.

import { useCore } from "../lib/core-provider.client";
import { eventOpenDeck, eventOpenDeckOptions } from "shared_types/app";
import { DeckTree } from "./deck-tree";
import { StudyScreen as StudyScreenView } from "./study";
import { DeckOptionsScreen as DeckOptionsView } from "./deck-options";
import { BrowserScreen as BrowserView } from "./browser";

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
  const { view, update } = useCore();
  if (view === null) return null;

  const route = view.route.kind === "Notes" ? "Notes" : "Cards";

  return (
    <BrowserView
      route={route}
      decks={view.decks}
      cards={view.browser_cards}
      cardPage={view.browser_page}
      cardPerPage={view.browser_per_page}
      cardTotal={view.browser_total}
      cardSelected={view.browser_selected}
      cardError={view.browser_error}
      notes={view.notes}
      notePage={view.notes_page}
      notePerPage={view.notes_per_page}
      noteTotal={view.notes_total}
      noteSelected={view.notes_selected}
      noteError={view.notes_error}
      noteTypes={view.note_types}
      canManageNotes={view.permissions.can_manage_notes}
      canManageTags={view.permissions.can_manage_tags}
      tags={view.tags}
      tagTree={view.tag_tree}
      update={update}
    />
  );
}
