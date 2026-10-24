use leptos::prelude::*;
use leptos::task;
use shared::app::{
    CardBrowserFilter, CardBrowserView, CurrentRoute, DeckSummary, NoteBrowserFilter, NoteView,
};
use shared::{Event, ViewModel};

use crate::app::BrowserState;
use crate::components::{
    Button, ButtonVariant, Input, Switch, Pagination, Select,
    Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
};
use crate::views::flag_dropdown::{flag_color, flag_label, flag_swatch};
use lepticons::{Icon, LucideGlyph};

/// The eight Anki flag values (0 = none, 1–7 the colours) for a flag `<select>`.
const FLAG_OPTIONS: &[(&str, &str)] = &[
    ("0", "No flag"),
    ("1", "Red"),
    ("2", "Orange"),
    ("3", "Green"),
    ("4", "Blue"),
    ("5", "Pink"),
    ("6", "Turquoise"),
    ("7", "Purple"),
];

/// The card browser (US-4.12): a management-oriented card table with
/// filter/sort controls (a hand-rolled collapsible filter tree in the sidebar),
/// pagination, and per-card Suspend/Bury/Reschedule/Flag actions. Consumes the
/// core's `browser_*` `ViewModel` fields and forwards the
/// `Browser*`/`CardsRequested` events.
#[component]
pub(crate) fn browser(
    vm: Signal<ViewModel>,
    state: BrowserState,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let vm_sig = vm;

    // Request the first page on entry (no filter, default sort/page), plus the
    // note-type list for the filter sidebar. Guarded by `initialized` so it only
    // fires the first time the browser is shown — not on every ViewModel-driven
    // re-render, which would clobber the user's active filter with an unfiltered
    // refetch.
    let initialized = state.initialized;
    Effect::new(move |_| {
        if initialized.get() {
            return;
        }
        initialized.set(true);
        set_event.set(Event::NoteTypesRequested);
        // The event bridge carries one event at a time; a second `set_event.set`
        // in the same tick would clobber the note-types request above. Defer the
        // cards fetch so both reach the core.
        let set_event = set_event;
        task::spawn_local(async move {
            set_event.set(Event::CardsRequested {
                filter: CardBrowserFilter {
                    deck_ids: Vec::new(),
                    note_type_ids: Vec::new(),
                    q: None,
                    states: Vec::new(),
                    flags: Vec::new(),
                    sort: "created_at".to_string(),
                    page: 1,
                    per_page: 50,
                },
            });
        });
    });

    // Filter/sort state lives in the hoisted `BrowserState` (stable across
    // re-renders); these are the individual signals backing the sidebar.
    let deck_value = state.deck;
    let note_type_value = state.note_type;
    let state_value = state.state;
    let flag_value = state.flag;
    let sort_value = state.sort;
    let q_value = state.query;
    let page = state.page;

    // The deck tree (hierarchy preserved for the sidebar) and the note-type list
    // are read reactively inside the render closure below (never snapshotted here).

    // Build a filter from the current signal state and fire `CardsRequested`.
    // `page` defaults to 1 (a filter change resets to the first page).
    let with_filter = move |page: i64| -> CardBrowserFilter {
        let mut deck_ids = Vec::new();
        if let Ok(d) = deck_value.get().parse::<i64>() {
            if d != 0 {
                deck_ids.push(d);
            }
        }
        let mut note_type_ids = Vec::new();
        if let Ok(n) = note_type_value.get().parse::<i64>() {
            if n != 0 {
                note_type_ids.push(n);
            }
        }
        let mut states = Vec::new();
        if !state_value.get().is_empty() {
            states.push(state_value.get());
        }
        let mut flags = Vec::new();
        if let Ok(f) = flag_value.get().parse::<i64>() {
            flags.push(f);
        }
        CardBrowserFilter {
            deck_ids,
            note_type_ids,
            q: if q_value.get().trim().is_empty() {
                None
            } else {
                Some(q_value.get().trim().to_string())
            },
            states,
            flags,
            sort: sort_value.get(),
            page,
            per_page: 50,
        }
    };

    // A no-arg variant for the tree leaves (which don't receive a useful event).
    let fire_filter = move || {
        page.set(1);
        let filter = with_filter(1);
        set_event.set(Event::CardsRequested { filter });
    };

    // Watch the pagination signal (thaw's `Pagination` mutates it directly) and
    // fire a fetch when the page changes away from what the core last returned.
    // Self-syncing (no explicit guard): after the fetch, `browser_page` catches up
    // to `page`, so the effect stops firing.
    Effect::new(move |_| {
        let p = page.get();
        let current = vm_sig.get().browser_page.max(1) as usize;
        if p != current {
            let filter = with_filter(p as i64);
            set_event.set(Event::CardsRequested { filter });
        }
    });

    // Suspend/bury/flag are read-only derived signals over the live
    // `browser_selected`. The custom `CardSwitch` mirrors state from these (never
    // writes locally) and emits the directional event on click.
    let selected_suspend = Signal::derive(move || {
        vm_sig.get().browser_selected.as_ref().map(|c| c.suspended).unwrap_or(false)
    });
    let selected_bury = Signal::derive(move || {
        vm_sig.get().browser_selected.as_ref().map(|c| c.buried).unwrap_or(false)
    });
    let selected_flag = Signal::derive(move || {
        vm_sig.get().browser_selected.as_ref().map(|c| c.flag.to_string()).unwrap_or_default()
    });

    // Selecting a leaf sets its category's filter value and re-applies; clicking
    // the already-selected leaf de-selects it (resets that category's filter).
    let select_deck = {
        let fire_filter = fire_filter.clone();
        move |id: String| {
            if deck_value.get() == id {
                deck_value.set(String::new());
            } else {
                deck_value.set(id);
            }
            fire_filter();
        }
    };
    let select_note_type = {
        let fire_filter = fire_filter.clone();
        move |id: String| {
            if note_type_value.get() == id {
                note_type_value.set(String::new());
            } else {
                note_type_value.set(id);
            }
            fire_filter();
        }
    };
    let select_state = {
        let fire_filter = fire_filter.clone();
        move |s: String| {
            if state_value.get() == s {
                state_value.set(String::new());
            } else {
                state_value.set(s);
            }
            fire_filter();
        }
    };
    let select_flag = {
        let fire_filter = fire_filter.clone();
        move |f: String| {
            if flag_value.get() == f {
                flag_value.set(String::new());
            } else {
                flag_value.set(f);
            }
            fire_filter();
        }
    };
    let select_sort = {
        let fire_filter = fire_filter.clone();
        move |s: String| {
            // Sort always defaults to a value; clicking the active one resets it.
            if sort_value.get() == s {
                sort_value.set("created_at".to_string());
            } else {
                sort_value.set(s);
            }
            fire_filter();
        }
    };

    // Clone the select closures for use inside the `view!` moves.
    let deck_sel = select_deck;
    let note_sel = select_note_type;
    let state_sel = select_state;
    let flag_sel = select_flag;
    let sort_sel = select_sort;

    // Active selections (reactive — `RwSignal`, so `--active` updates live).
    let deck_active = deck_value.clone();
    let note_active = note_type_value.clone();
    let state_active = state_value.clone();
    let flag_active = flag_value.clone();
    let sort_active = sort_value.clone();

    // Search fires on Enter (the form's submit). `fire_filter` resets the page
    // and applies the current filter (including the query).
    let search_apply = fire_filter.clone();

    // -----------------------------------------------------------------
    // Note view. Mirrors the card view: a filter sidebar (deck/note-type/sort),
    // a table, a right-hand inspector pane, and bottom pagination. Drives the
    // notes core surface (`NotesRequested { filter }`, `notes_page/total/per_page`,
    // `NoteCreate`/`NoteSave`/`NoteDelete`).

    let note_deck_filter = state.note_deck_filter;
    let note_note_type = state.note_note_type;
    let note_sort = state.note_sort;
    let note_query = state.note_query;
    let note_page = state.note_page;
    let notes_initialized = state.notes_initialized;

    // Lazy first fetch of the note list when the Notes view is toggled on.
    Effect::new(move |_| {
        if vm_sig.get().route != CurrentRoute::Notes {
            return;
        }
        if notes_initialized.get() {
            return;
        }
        notes_initialized.set(true);
        set_event.set(Event::NotesRequested {
            filter: NoteBrowserFilter::default(),
        });
    });

    let with_note_filter = move |page: i64| -> NoteBrowserFilter {
        let mut deck_ids = Vec::new();
        if let Ok(d) = note_deck_filter.get().parse::<i64>() {
            if d != 0 {
                deck_ids.push(d);
            }
        }
        let mut note_type_ids = Vec::new();
        if let Ok(n) = note_note_type.get().parse::<i64>() {
            if n != 0 {
                note_type_ids.push(n);
            }
        }
        NoteBrowserFilter {
            deck_ids,
            note_type_ids,
            q: if note_query.get().trim().is_empty() {
                None
            } else {
                Some(note_query.get().trim().to_string())
            },
            sort: note_sort.get(),
            page,
            per_page: 50,
        }
    };

    let fire_note_filter = move |page: i64| {
        note_page.set(page.max(1) as usize);
        set_event.set(Event::NotesRequested { filter: with_note_filter(page) });
    };

    // Selecting a note-view filter leaf sets the value and re-applies; clicking
    // the active leaf de-selects it.
    let select_note_deck = {
        let fire_note_filter = fire_note_filter.clone();
        move |id: String| {
            if note_deck_filter.get() == id {
                note_deck_filter.set("0".to_string());
            } else {
                note_deck_filter.set(id);
            }
            fire_note_filter(1);
        }
    };
    let select_note_type_filter = {
        let fire_note_filter = fire_note_filter.clone();
        move |id: String| {
            if note_note_type.get() == id {
                note_note_type.set(String::new());
            } else {
                note_note_type.set(id);
            }
            fire_note_filter(1);
        }
    };
    let select_note_sort = {
        let fire_note_filter = fire_note_filter.clone();
        move |s: String| {
            if note_sort.get() == s {
                note_sort.set("created_at".to_string());
            } else {
                note_sort.set(s);
            }
            fire_note_filter(1);
        }
    };

    // Watch the note pagination signal and fetch a new page when it changes.
    Effect::new(move |_| {
        if vm_sig.get().route != CurrentRoute::Notes {
            return;
        }
        let p = note_page.get();
        let current = vm_sig.get().notes_page.max(1) as usize;
        if p != current {
            set_event.set(Event::NotesRequested { filter: with_note_filter(p as i64) });
        }
    });

    // Stable derived signals read by the render (created once in the component
    // body — creating a `Signal::derive` inside the per-render closure would leak
    // a new memo every render).
    let notes_selected_signal = Signal::derive(move || vm_sig.get().notes_selected.clone());
    let browser_selected_signal = Signal::derive(move || vm_sig.get().browser_selected.clone());

    // Pagination page-count derives (created once in the component body).
    let browser_page_count = Signal::derive(move || {
        let total = vm_sig.get().browser_total.max(0) as usize;
        let per = vm_sig.get().browser_per_page.max(1) as usize;
        ((total + per - 1) / per).max(1)
    });
    let notes_page_count = Signal::derive(move || {
        let total = vm_sig.get().notes_total.max(0) as usize;
        let per = vm_sig.get().notes_per_page.max(1) as usize;
        ((total + per - 1) / per).max(1)
    });

    // Owned snapshot of the note list is re-read reactively inside the render
    // closure below (never captured once here).

    view! {
        <div class="card-browser">
            <div class="card-browser__body">
            <aside class="card-browser__sidebar">
                {match vm_sig.get().route {
                    CurrentRoute::Cards => view! {
                        <form
                            class="card-browser__sidebar-search"
                            on:submit=move |e: leptos::ev::SubmitEvent| {
                                e.prevent_default();
                                search_apply();
                            }
                        >
                            <div class="flex w-full">
                                <Input value=q_value placeholder="Search…" class="flex-1 rounded-r-none" />
                                <Button button_type="submit" aria_label="Search" square=true on_click=|_| {}>
                                    <Icon glyph=LucideGlyph::Search size="16" />
                                </Button>
                            </div>
                        </form>

                        {move || vm_sig.get().browser_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load cards: " {err}</p>
                        })}
                        {move || vm_sig.get().note_types_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load note types: " {err}</p>
                        })}

                        <ul class="card-browser__menu">
                            {filter_branch(
                                "Decks",
                                deck_filter_tree(
                                    vm_sig.get().decks.clone(),
                                    deck_active,
                                    deck_sel,
                                ),
                            )}

                            {filter_branch(
                                "Note types",
                                {move || vm_sig.get().note_types.iter().map(|nt| {
                                    let id_str = nt.id.to_string();
                                    let active = note_active.get() == id_str;
                                    let name = nt.name.clone();
                                    let sel = note_sel.clone();
                                    filter_leaf(active, id_str, name, move |id| sel(id))
                                }).collect_view()},
                            )}

                            {filter_branch(
                                "State",
                                ["new", "learning", "review", "relearning"].into_iter().map(|s| {
                                    let s_val = s.to_string();
                                    let active = state_active.get() == s_val;
                                    let sel = state_sel.clone();
                                    let label: &'static str = match s {
                                        "new" => "New",
                                        "learning" => "Learning",
                                        "review" => "Review",
                                        "relearning" => "Relearning",
                                        _ => unreachable!(),
                                    };
                                    filter_leaf(active, s_val, label.to_string(), move |id| sel(id))
                                }).collect_view(),
                            )}

                            {filter_branch(
                                "Flag",
                                ["1", "2", "3", "4", "5", "6", "7"].into_iter().map(|f| {
                                    let f_val = f.to_string();
                                    let active = flag_active.get() == f_val;
                                    let sel = flag_sel.clone();
                                    let color = flag_color(f.parse::<i64>().unwrap());
                                    let label = flag_label(f.parse::<i64>().unwrap());
                                    view! {
                                        <li>
                                            <button class="flex w-full items-center rounded-sm px-3 py-1.5 text-sm" class:bg-accent=active on:click=move |_| sel(f_val.clone())>
                                                <span class="card-browser__flag-sw">
                                                    <span class="card-browser__flag-swatch" style=format!("background-color: {color}")></span>
                                                    <span>{label}</span>
                                                </span>
                                            </button>
                                        </li>
                                    }
                                }).collect_view(),
                            )}

                            {filter_branch(
                                "Sort",
                                ["created_at", "due_at", "deck", "question"].into_iter().map(|s| {
                                    let s_val = s.to_string();
                                    let active = sort_active.get() == s_val;
                                    let sel = sort_sel.clone();
                                    let label: &'static str = match s {
                                        "created_at" => "Created",
                                        "due_at" => "Due",
                                        "deck" => "Deck",
                                        "question" => "Question",
                                        _ => unreachable!(),
                                    };
                                    filter_leaf(active, s_val, label.to_string(), move |id| sel(id))
                                }).collect_view(),
                            )}
                        </ul>
                    }
                    .into_any(),
                    CurrentRoute::Notes => view! {
                        <form
                            class="card-browser__sidebar-search"
                            on:submit=move |e: leptos::ev::SubmitEvent| {
                                e.prevent_default();
                                fire_note_filter(1);
                            }
                        >
                            <div class="flex w-full">
                                <Input value=note_query placeholder="Search…" class="flex-1 rounded-r-none" />
                                <Button button_type="submit" aria_label="Search" square=true on_click=|_| {}>
                                    <Icon glyph=LucideGlyph::Search size="16" />
                                </Button>
                            </div>
                        </form>

                        {move || vm_sig.get().notes_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load notes: " {err}</p>
                        })}
                        {move || vm_sig.get().note_types_error.clone().map(|err| view! {
                            <p class="deck-options__error">"Could not load note types: " {err}</p>
                        })}

                        <ul class="card-browser__menu">
                            {filter_branch(
                                "Decks",
                                deck_filter_tree(
                                    vm_sig.get().decks.clone(),
                                    note_deck_filter,
                                    select_note_deck,
                                ),
                            )}

                            {filter_branch(
                                "Note types",
                                {move || vm_sig.get().note_types.iter().map(|nt| {
                                    let id_str = nt.id.to_string();
                                    let active = note_note_type.get() == id_str;
                                    let name = nt.name.clone();
                                    let sel = select_note_type_filter.clone();
                                    filter_leaf(active, id_str, name, move |id| sel(id))
                                }).collect_view()},
                            )}

                            {filter_branch(
                                "Sort",
                                ["created_at", "note_type", "deck", "question"].into_iter().map(|s| {
                                    let s_val = s.to_string();
                                    let active = note_sort.get() == s_val;
                                    let sel = select_note_sort.clone();
                                    let label: &'static str = match s {
                                        "created_at" => "Created",
                                        "note_type" => "Note type",
                                        "deck" => "Deck",
                                        "question" => "Question",
                                        _ => unreachable!(),
                                    };
                                    filter_leaf(active, s_val, label.to_string(), move |id| sel(id))
                                }).collect_view(),
                            )}
                        </ul>
                    }
                    .into_any(),
                    _ => ().into_any(),
                }}
            </aside>

            <div class="card-browser__main">
                {match vm_sig.get().route {
                    CurrentRoute::Cards => view! {
                        <div class="card-browser__content">
                            <div class="card-browser__table">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHead>"Card"</TableHead>
                                        <TableHead>"Deck"</TableHead>
                                        <TableHead>"State"</TableHead>
                                        <TableHead>"Flag"</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {move || vm_sig.get().browser_cards.iter().map(|c| {
                                        let card_id = c.card_id;
                                        let is_selected = vm_sig.get().browser_selected.as_ref().map(|c| c.card_id) == Some(card_id);
                                        let front = c.front.clone();
                                        let deck_title = c.deck_title.clone();
                                        let state = c.state.clone();
                                        let flag = c.flag;
                                        let suspended = c.suspended;
                                        let buried = c.buried;
                                        view! {
                                            <TableRow
                                                selected=Signal::derive(move || is_selected)
                                                on_click=Callback::new(move |_| set_event.set(Event::BrowserSelectCard { card_id }))
                                            >
                                                <TableCell>
                                                    <span class="card-browser__front">
                                                        {front}
                                                        {if suspended { " (suspended)" } else { "" }}
                                                        {if buried { " (buried)" } else { "" }}
                                                    </span>
                                                </TableCell>
                                                <TableCell>{deck_title}</TableCell>
                                                <TableCell>{state}</TableCell>
                                                <TableCell>{flag_swatch(flag)}</TableCell>
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                            </div>

                            <div class="card-browser__pagination">
                                <span>{move || vm_sig.get().browser_total.to_string()}{" cards"}</span>
                                <Pagination page=page page_count=browser_page_count />
                            </div>
                        </div>

                        <CardInspector
                            card=browser_selected_signal
                            suspend=selected_suspend
                            bury=selected_bury
                            flag=selected_flag
                            set_event=set_event
                        />
                    }
                    .into_any(),
                    CurrentRoute::Notes => view! {
                        <div class="card-browser__content">
                            <div class="card-browser__table">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHead>"Sort field"</TableHead>
                                        <TableHead>"Note type"</TableHead>
                                        <TableHead>"Tags"</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {move || vm_sig.get().notes.iter().map(|n| {
                                        let note_id = n.id;
                                        let note_type_name = n.note_type_name.clone();
                                        let sort_field = n.sort_field.clone().unwrap_or_default();
                                        let tags = n.tags.join(", ");
                                        view! {
                                            <TableRow
                                                selected=Signal::derive(move || vm_sig.get().notes_selected.as_ref().map(|n| n.id) == Some(note_id))
                                                on_click=Callback::new(move |_| set_event.set(Event::NoteBrowserSelectNote { note_id }))
                                            >
                                                <TableCell>
                                                    <span class="card-browser__front">{sort_field}</span>
                                                </TableCell>
                                                <TableCell>{note_type_name}</TableCell>
                                                <TableCell>{tags}</TableCell>
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                            </div>

                            <div class="card-browser__pagination">
                                <span>{move || vm_sig.get().notes_total.to_string()}{" notes"}</span>
                                <Pagination page=note_page page_count=notes_page_count />
                            </div>
                        </div>

                        <NoteInspector
                            note=notes_selected_signal
                        />
                    }
                    .into_any(),
                    _ => ().into_any(),
                }}
            </div>
        </div>
        </div>
    }
}

/// The card-view information/action pane (right-hand side of the browser).
/// Always rendered; shows the selected card's fields plus the per-card actions
/// (Suspend / Bury / Reschedule / Flag), or a hint when nothing is selected.
#[component]
fn CardInspector(
    card: Signal<Option<CardBrowserView>>,
    suspend: Signal<bool>,
    bury: Signal<bool>,
    flag: Signal<String>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let reschedule_days = RwSignal::new(String::new());

    view! {
        <aside class="card-browser__inspector">
            <h3 class="card-browser__inspector-title">"Selected card"</h3>

            {if let Some(card) = card.get() {
                let card_id = card.card_id;
                view! {
                    <dl class="card-browser__inspector-fields">
                        <div class="card-browser__inspector-row">
                            <dt>"Card"</dt>
                            <dd>{card.card_id.to_string()}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Note"</dt>
                            <dd>{card.note_id.to_string()}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Deck"</dt>
                            <dd>{card.deck_title}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Note type"</dt>
                            <dd>{card.note_type_name}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"State"</dt>
                            <dd>{card.state}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Suspended"</dt>
                            <dd>
                                <CardSwitch
                                    checked=suspend
                                    label=""
                                    on_toggle=move || {
                                        if suspend.get() {
                                            set_event.set(Event::BrowserUnsuspendCard { card_id });
                                        } else {
                                            set_event.set(Event::BrowserSuspendCard { card_id });
                                        }
                                    }
                                />
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Buried"</dt>
                            <dd>
                                <CardSwitch
                                    checked=bury
                                    label=""
                                    disabled=suspend
                                    on_toggle=move || {
                                        if bury.get() {
                                            set_event.set(Event::BrowserUnburyCard { card_id });
                                        } else {
                                            set_event.set(Event::BrowserBuryCard { card_id });
                                        }
                                    }
                                />
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Flag"</dt>
                            <dd>
                                <Select
                                    value=Signal::derive(move || flag.get())
                                    options=FLAG_OPTIONS
                                    on_change=Some(Callback::new(move |f: String| {
                                        set_event.set(Event::BrowserFlagCard {
                                            card_id,
                                            flag: f.parse::<i64>().unwrap_or(0) as i32,
                                        });
                                    }))
                                />
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Reschedule"</dt>
                            <dd>
                                <div class="card-browser__inspector-reschedule">
                                    <div class="flex">
                                        <Input value=reschedule_days input_type="number" placeholder="days" class="rounded-r-none" />
                                        <Button
                                            variant=ButtonVariant::Ghost
                                            square=true
                                            join_item=true
                                            aria_label="Reschedule"
                                            on_click=move |_| {
                                                if let Ok(d) = reschedule_days.get().trim().parse::<i64>() {
                                                    set_event.set(Event::BrowserRescheduleCard { card_id, days: d });
                                                    reschedule_days.set(String::new());
                                                }
                                            }
                                        >
                                            <Icon glyph=LucideGlyph::Check size="16" />
                                        </Button>
                                    </div>
                                </div>
                            </dd>
                        </div>
                        <div class="card-browser__inspector-row card-browser__inspector-row--stacked">
                            <dt>"Front"</dt>
                            <dd class="card-browser__inspector-text">{card.front}</dd>
                        </div>
                        <div class="card-browser__inspector-row card-browser__inspector-row--stacked">
                            <dt>"Back"</dt>
                            <dd class="card-browser__inspector-text">{card.back}</dd>
                        </div>
                    </dl>
                }
                .into_any()
            } else {
                view! {
                    <span class="card-browser__hint">"Select a card to view and manage it."</span>
                }
                .into_any()
            }}
        </aside>
    }
}

/// The note-view information pane (mirrors `CardInspector`). Read-only: shows
/// the selected note's fields, note type, and tags. Note editing/creation/delete
/// are out of scope for now, so there are no inputs or actions here.
#[component]
fn NoteInspector(
    note: Signal<Option<NoteView>>,
) -> impl IntoView {
    view! {
        <aside class="card-browser__inspector">
            <h3 class="card-browser__inspector-title">"Note"</h3>

            {if let Some(note) = note.get() {
                let note_type_name = note.note_type_name.clone();
                let tags = note.tags.clone();
                let mut fields: Vec<(String, String)> = note.fields.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                view! {
                    <dl class="card-browser__inspector-fields">
                        <div class="card-browser__inspector-row">
                            <dt>"ID"</dt>
                            <dd>{note.id.to_string()}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Note type"</dt>
                            <dd>{note_type_name}</dd>
                        </div>
                        <div class="card-browser__inspector-row">
                            <dt>"Tags"</dt>
                            <dd>{tags.join(", ")}</dd>
                        </div>
                        {fields.iter().map(|(name, value)| {
                            let name = name.clone();
                            let value = value.clone();
                            view! {
                                <div class="card-browser__inspector-row card-browser__inspector-row--stacked">
                                    <dt>{name}</dt>
                                    <dd class="card-browser__inspector-text">{value}</dd>
                                </div>
                            }
                        }).collect_view()}
                    </dl>
                }
                .into_any()
            } else {
                view! {
                    <span class="card-browser__hint">"Select a note to view it."</span>
                }
                .into_any()
            }}
        </aside>
    }
}

/// An unidirectional suspend/bury toggle. This is **purely reflective**: it has
/// no writable state of its own — the checked state comes only from the `checked`
/// signal (derived from the core's `ViewModel`), and a click emits a *directional*
/// event via `on_toggle` (suspend vs unsuspend, bury vs unbury). The core's
/// response re-syncs `checked`, which drives the visible toggle back to truth.
///
/// The toggle itself is the owned `Switch` primitive (signal-driven
/// `checked`, no DOM mirroring).
#[component]
fn CardSwitch(
    checked: Signal<bool>,
    label: &'static str,
    on_toggle: impl Fn() + 'static,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <label class="card-browser__switch">
            <Switch checked=checked on_toggle=on_toggle disabled=disabled />
            <span class="card-browser__switch-label">{label}</span>
        </label>
    }
}

/// A collapsible filter category in the browser sidebar, rendered with a native
/// `<details>`/`<summary>`. Expansion is native (no signal needed — the browser
/// tracks the `<details>` `open` state). A plain `fn` (not `#[component]`) so it
/// sits inline in the reactive `view!` and re-renders when its data (e.g. the
/// async-loaded note types) arrives.
fn filter_branch(label: &'static str, leaves: impl IntoView) -> impl IntoView {
    view! {
        <li>
            <details>
                <summary>{label}</summary>
                <ul>{leaves}</ul>
            </details>
        </li>
    }
}

/// A single selectable filter leaf (native `<button>`), highlighted with the
/// accent background when selected.
fn filter_leaf(
    active: bool,
    value: String,
    label: String,
    on_select: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <li>
            <button class="flex w-full rounded-sm px-3 py-1.5 text-sm" class:bg-accent=active on:click=move |_| on_select(value.clone())>
                {label}
            </button>
        </li>
    }
}

/// Renders the deck hierarchy as nested collapsible groups. A deck with children
/// is an expandable `<details>`/`<summary>`. Because the summary toggles the
/// submenu (it can't also be a filter), the parent deck's own selectable entry
/// is rendered as the **first item inside** its submenu (labelled "this deck
/// only"), followed by its children. A leaf deck is a plain selectable
/// `<li><button>`.
fn deck_filter_tree(
    decks: Vec<DeckSummary>,
    active: RwSignal<String>,
    on_select: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    decks
        .into_iter()
        .map(|deck| deck_filter_node(deck, active.clone(), on_select.clone()))
        .collect_view()
}

/// A single deck node in the filter menu.
fn deck_filter_node(
    deck: DeckSummary,
    active: RwSignal<String>,
    on_select: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    let id_str = deck.id.to_string();
    let title = deck.title;
    let has_children = !deck.children.is_empty();
    let children = deck.children;

    // Active-state reader for this deck (used by the selectable entries).
    let is_active = {
        let id_str = id_str.clone();
        move || active.get() == id_str
    };

    if has_children {
        let select_self = on_select.clone();
        let self_id = id_str.clone();
        // The parent's own "filter to this deck only" entry, shown first inside
        // the expanded submenu (the summary itself only toggles the submenu).
        let self_item = view! {
            <li>
                <button class="flex w-full items-center gap-2 rounded-sm px-3 py-1.5 text-sm" class:bg-accent=is_active.clone() on:click=move |_| select_self(self_id.clone())>
                    {title.clone()}
                    <span class="card-browser__deck-self">{"this deck only"}</span>
                </button>
            </li>
        };
        view! {
            <li>
                <details>
                    <summary>{title}</summary>
                    <ul>
                        {self_item}
                        {deck_filter_tree(children, active.clone(), on_select)}
                    </ul>
                </details>
            </li>
        }
        .into_any()
    } else {
        view! {
            <li>
                <button class="flex w-full rounded-sm px-3 py-1.5 text-sm" class:bg-accent=is_active on:click=move |_| on_select(id_str.clone())>
                    {title}
                </button>
            </li>
        }
        .into_any()
    }
}
