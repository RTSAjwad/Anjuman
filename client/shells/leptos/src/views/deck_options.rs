use leptos::prelude::*;
use shared::app::{DeckOptionsEdit, DeckOptionsView, PresetSummary};
use shared::Event;

use crate::components::{
    Button, ButtonAppearance, Input, UiSelect,
};
use crate::views::study::format_interval;

/// Editable deck-options form (US-4.10): renders the selected deck's preset as
/// controls (number/text inputs, bool switches, enum selectors), applies local
/// validation, and forwards `Event::DeckOptionsSave` with the edited values.
///
/// Nothing is shown until the preset has been fetched (via the Options route's
/// `DeckOptionsRequested`). The form state is local (this component) and reset
/// on each entry to Options.
#[component]
pub(crate) fn deck_options_panel(
    deck_id: i64,
    options: Option<DeckOptionsView>,
    error: Option<String>,
    presets: Vec<PresetSummary>,
    presets_error: Option<String>,
    can_manage_presets: bool,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let options = match options {
        Some(o) => o,
        None => {
            // Error and empty states for an unresolved preset.
            return view! {
                <section class="deck-options">
                    {if let Some(err) = error {
                        view! { <p class="deck-options__error">"Could not load deck options: " {err}</p> }
                            .into_any()
                    } else {
                        view! { <p class="deck-options__empty">"No deck options to show."</p> }
                            .into_any()
                    }}
                </section>
            }
            .into_any();
        }
    };

    let name = options.name.clone();
    // The system-owned global default (id 0) is read-only — see US-4.10 and the
    // server report.
    let read_only = options.id == 0;
    // Editing/saving is gated on the management permission (US-9.5); without it
    // the preset is shown read-only (view stays, controls are locked).
    let can_edit = can_manage_presets && !read_only;
    // Editable draft state, initialized from the loaded preset. Rebuilt on each
    // entry to Options (see `RootComponent`, which mounts this only on the
    // Options route).
    let state = DeckOptionsEditState::from_view(&options);
    // Local (shell-side) save error: parse errors that never reach the core.
    // `error` above carries the core-set fetch/save error; this is separate.
    let local_error = RwSignal::new(None::<String>);

    // US-4.11: request the school's preset list once, on entry to the screen, so
    // the assign picker can render it (the server's `GET /deck-options` does not
    // include the system default id 0, so the shell synthesizes that entry).
    Effect::new(move |_| {
        set_event.set(Event::DeckOptionsListRequested);
    });

    // The assign picker: a native `<select>` showing the deck's current preset
    // id. On a user change, forward `Event::DeckOptionsAssign` directly (no thaw
    // `Select`/`Effect` workaround needed — the `on:change` only fires on a real
    // user change). The core re-fetches after assignment, so `options.id` then
    // matches the newly-assigned id.
    let assign_selection = RwSignal::new(options.id.to_string());

    let on_save = move |_| {
        local_error.set(None);
        match state.to_edit() {
            Ok(edit) => set_event.set(Event::DeckOptionsSave(edit)),
            Err(msg) => local_error.set(Some(msg)),
        }
    };

    view! {
        <section class="deck-options" class:deck-options--readonly=move || !can_edit>
            <header class="deck-options__header">
                <h2 class="deck-options__title">"Deck options"</h2>
                <span class="deck-options__preset">
                    {name}
                    {if read_only { " — read-only" } else { "" }}
                </span>
            </header>

            {if read_only {
                view! {
                    <p class="deck-options__readonly-notice">
                        "This is the shared default preset. It can't be edited — create your own preset to customise scheduling."
                    </p>
                }
                .into_any()
            } else if !can_manage_presets {
                view! {
                    <p class="deck-options__readonly-notice">
                        "You don't have permission to change deck options."
                    </p>
                }
                .into_any()
            } else {
                ().into_any()
            }}

            {if can_manage_presets {
                view! {
                    <div class="deck-options-assign">
                        <span class="deck-options-assign__label">"Preset"</span>
                        <select
                            class="h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm"
                            prop:value=move || assign_selection.get()
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                assign_selection.set(value.clone());
                                if let Ok(id) = value.parse::<i64>() {
                                    set_event.set(Event::DeckOptionsAssign {
                                        deck_id,
                                        options_id: id,
                                    });
                                }
                            }
                        >
                            <option value="0">"Default"</option>
                            {presets.iter().map(|p| {
                                let id = p.id.to_string();
                                let name = p.name.clone();
                                view! { <option value=id>{name}</option> }
                            }).collect_view()}
                        </select>
                        {presets_error.map(|err| view! {
                            <span class="deck-options__error">{err}</span>
                        })}
                    </div>
                }
                .into_any()
            } else {
                ().into_any()
            }}

            <OptionsGroup title="Daily limits">
                <NumberField label="New cards/day" value=state.new_per_day placeholder="20" />
                <NumberField label="Maximum reviews/day" value=state.review_per_day placeholder="200" />
            </OptionsGroup>

            <OptionsGroup title="New cards">
                <TextField label="Learning steps" value=state.learning_steps placeholder="1m 10m" />
                <SelectField
                    label="Insertion order"
                    value=state.insertion_order
                    options=INSERTION_ORDER_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="Lapses">
                <TextField label="Relearning steps" value=state.relearning_steps placeholder="10m" />
                <NumberField label="Leech threshold" value=state.leech_threshold placeholder="8" />
                <SelectField label="Leech action" value=state.leech_action options=LEECH_ACTION_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="Display order">
                <SelectField label="New card gather order" value=state.new_gather_order options=NEW_GATHER_OPTIONS />
                <SelectField label="New card sort order" value=state.new_sort_order options=NEW_SORT_OPTIONS />
                <SelectField label="New/review order" value=state.new_review_order options=NEW_REVIEW_OPTIONS />
                <SelectField label="Interday learning/review order" value=state.interday_order options=NEW_REVIEW_OPTIONS />
                <SelectField label="Review sort order" value=state.review_sort_order options=REVIEW_SORT_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="Burying">
                <ToggleField label="Bury new siblings" checked=state.bury_new />
                <ToggleField label="Bury review siblings" checked=state.bury_review />
                <ToggleField label="Bury interday learning siblings" checked=state.bury_interday />
            </OptionsGroup>

            <OptionsGroup title="Audio">
                <ToggleField label="Don't play audio automatically" checked=state.dont_play_audio_automatically />
                <ToggleField label="Skip question when replaying answer" checked=state.skip_question_when_replaying_answer />
            </OptionsGroup>

            <OptionsGroup title="Timers">
                <NumberField label="Maximum answer seconds" value=state.maximum_answer_seconds placeholder="60" />
                <ToggleField label="Show on-screen timer" checked=state.show_on_screen_timer />
                <ToggleField label="Stop on-screen timer on answer" checked=state.stop_timer_on_answer />
            </OptionsGroup>

            <OptionsGroup title="Auto advance">
                <NumberField label="Seconds to show question" value=state.auto_advance_show_question placeholder="0" />
                <NumberField label="Seconds to show answer" value=state.auto_advance_show_answer placeholder="0" />
                <ToggleField label="Wait for audio" checked=state.auto_advance_wait_for_audio />
                <SelectField label="Question action" value=state.auto_advance_question_action options=QUESTION_ACTION_OPTIONS />
                <SelectField label="Answer action" value=state.auto_advance_answer_action options=ANSWER_ACTION_OPTIONS />
            </OptionsGroup>

            <OptionsGroup title="FSRS">
                <NumberField label="Desired retention (%) " value=state.desired_retention placeholder="90" />
                <EasyDaysField days=state.easy_days />
            </OptionsGroup>

            <OptionsGroup title="Advanced">
                <NumberField label="Maximum interval (days)" value=state.maximum_interval placeholder="36500" />
                <FieldRow label="FSRS parameters">
                    <span class="deck-options__readonly">
                        {if options.fsrs_parameters.is_empty() {
                            "Default".to_string()
                        } else {
                            format_parameters(&options.fsrs_parameters)
                        }}
                    </span>
                </FieldRow>
            </OptionsGroup>

            <footer class="deck-options__footer">
                {move || {
                    if let Some(msg) = local_error.get() {
                        view! { <p class="deck-options__error">{msg}</p> }.into_any()
                    } else if let Some(msg) = error.clone() {
                        view! { <p class="deck-options__error">{msg}</p> }.into_any()
                    } else {
                        ().into_any()
                    }
                }}
                {if can_edit {
                    view! {
                        <Button appearance=ButtonAppearance::Primary on_click=on_save>
                            "Save"
                        </Button>
                    }
                    .into_any()
                } else {
                    ().into_any()
                }}
            </footer>
        </section>
    }
    .into_any()
}

/// The editable draft state for a deck-options preset. Each control binds to a
/// `RwSignal`; `to_edit` parses the raw strings into a typed `DeckOptionsEdit`
/// (or returns a human-readable parse error). Enums stay as their `snake_case`
/// wire form; numbers/steps are parsed on save.
#[derive(Clone, Copy)]
struct DeckOptionsEditState {
    new_per_day: RwSignal<String>,
    review_per_day: RwSignal<String>,
    leech_threshold: RwSignal<String>,
    maximum_answer_seconds: RwSignal<String>,
    maximum_interval: RwSignal<String>,
    desired_retention: RwSignal<String>,
    auto_advance_show_question: RwSignal<String>,
    auto_advance_show_answer: RwSignal<String>,
    learning_steps: RwSignal<String>,
    relearning_steps: RwSignal<String>,
    bury_new: RwSignal<bool>,
    bury_review: RwSignal<bool>,
    bury_interday: RwSignal<bool>,
    dont_play_audio_automatically: RwSignal<bool>,
    skip_question_when_replaying_answer: RwSignal<bool>,
    show_on_screen_timer: RwSignal<bool>,
    stop_timer_on_answer: RwSignal<bool>,
    auto_advance_wait_for_audio: RwSignal<bool>,
    new_gather_order: RwSignal<String>,
    new_sort_order: RwSignal<String>,
    new_review_order: RwSignal<String>,
    interday_order: RwSignal<String>,
    review_sort_order: RwSignal<String>,
    insertion_order: RwSignal<String>,
    leech_action: RwSignal<String>,
    auto_advance_question_action: RwSignal<String>,
    auto_advance_answer_action: RwSignal<String>,
    easy_days: [RwSignal<String>; 7],
}

impl DeckOptionsEditState {
    /// Initialise the draft from a loaded preset's current values.
    fn from_view(o: &DeckOptionsView) -> Self {
        DeckOptionsEditState {
            new_per_day: RwSignal::new(o.new_per_day.to_string()),
            review_per_day: RwSignal::new(o.review_per_day.to_string()),
            leech_threshold: RwSignal::new(o.leech_threshold.to_string()),
            maximum_answer_seconds: RwSignal::new(o.maximum_answer_seconds.to_string()),
            maximum_interval: RwSignal::new(o.maximum_interval.to_string()),
            desired_retention: RwSignal::new(format_desired_retention(o.desired_retention)),
            auto_advance_show_question: RwSignal::new(format_float_input(o.auto_advance_seconds_show_question)),
            auto_advance_show_answer: RwSignal::new(format_float_input(o.auto_advance_seconds_show_answer)),
            learning_steps: RwSignal::new(format_steps(&o.learning_steps)),
            relearning_steps: RwSignal::new(format_steps(&o.relearning_steps)),
            bury_new: RwSignal::new(o.bury_new),
            bury_review: RwSignal::new(o.bury_review),
            bury_interday: RwSignal::new(o.bury_interday),
            dont_play_audio_automatically: RwSignal::new(o.dont_play_audio_automatically),
            skip_question_when_replaying_answer: RwSignal::new(o.skip_question_when_replaying_answer),
            show_on_screen_timer: RwSignal::new(o.show_on_screen_timer),
            stop_timer_on_answer: RwSignal::new(o.stop_timer_on_answer),
            auto_advance_wait_for_audio: RwSignal::new(o.auto_advance_wait_for_audio),
            new_gather_order: RwSignal::new(o.new_gather_order.clone()),
            new_sort_order: RwSignal::new(o.new_sort_order.clone()),
            new_review_order: RwSignal::new(o.new_review_order.clone()),
            interday_order: RwSignal::new(o.interday_order.clone()),
            review_sort_order: RwSignal::new(o.review_sort_order.clone()),
            insertion_order: RwSignal::new(o.insertion_order.clone()),
            leech_action: RwSignal::new(o.leech_action.clone()),
            auto_advance_question_action: RwSignal::new(o.auto_advance_question_action.clone()),
            auto_advance_answer_action: RwSignal::new(o.auto_advance_answer_action.clone()),
            easy_days: {
                // Always 7 entries (the wire array is Monday-first); defensively
                // default any missing entry to "normal".
                let arr: [RwSignal<String>; 7] =
                    std::array::from_fn(|_| RwSignal::new("normal".to_string()));
                for (i, d) in o.easy_days.iter().enumerate().take(7) {
                    arr[i].set(d.clone());
                }
                arr
            },
        }
    }

    /// Parse the raw draft into a typed `DeckOptionsEdit`. All fields are `Some`
    /// (the whole preset is sent on save); enums stay snake_case; numbers and
    /// steps are parsed, returning `Err` with a readable message on failure.
    fn to_edit(&self) -> Result<DeckOptionsEdit, String> {
        let int = |s: &RwSignal<String>, what: &str| -> Result<i64, String> {
            let t = s.get().trim().to_string();
            t.parse::<i64>().map_err(|_| format!("{what} must be a whole number"))
        };
        let float = |s: &RwSignal<String>, what: &str| -> Result<f64, String> {
            let t = s.get().trim().to_string();
            t.parse::<f64>().map_err(|_| format!("{what} must be a number"))
        };

        Ok(DeckOptionsEdit {
            new_per_day: Some(int(&self.new_per_day, "New cards/day")?),
            review_per_day: Some(int(&self.review_per_day, "Maximum reviews/day")?),
            leech_threshold: Some(int(&self.leech_threshold, "Leech threshold")?),
            maximum_answer_seconds: Some(int(&self.maximum_answer_seconds, "Maximum answer seconds")?),
            maximum_interval: Some(int(&self.maximum_interval, "Maximum interval")?),
            // Desired retention is entered as a percent (e.g. 90); convert to 0..1.
            desired_retention: Some(float(&self.desired_retention, "Desired retention")? / 100.0),
            auto_advance_seconds_show_question: Some(float(&self.auto_advance_show_question, "Auto-advance seconds (question)")?),
            auto_advance_seconds_show_answer: Some(float(&self.auto_advance_show_answer, "Auto-advance seconds (answer)")?),
            learning_steps: Some(self.learning_steps.get().trim().to_string()),
            relearning_steps: Some(self.relearning_steps.get().trim().to_string()),
            bury_new: Some(self.bury_new.get()),
            bury_review: Some(self.bury_review.get()),
            bury_interday: Some(self.bury_interday.get()),
            dont_play_audio_automatically: Some(self.dont_play_audio_automatically.get()),
            skip_question_when_replaying_answer: Some(self.skip_question_when_replaying_answer.get()),
            show_on_screen_timer: Some(self.show_on_screen_timer.get()),
            stop_timer_on_answer: Some(self.stop_timer_on_answer.get()),
            auto_advance_wait_for_audio: Some(self.auto_advance_wait_for_audio.get()),
            new_gather_order: Some(self.new_gather_order.get()),
            new_sort_order: Some(self.new_sort_order.get()),
            new_review_order: Some(self.new_review_order.get()),
            interday_order: Some(self.interday_order.get()),
            review_sort_order: Some(self.review_sort_order.get()),
            insertion_order: Some(self.insertion_order.get()),
            leech_action: Some(self.leech_action.get()),
            auto_advance_question_action: Some(self.auto_advance_question_action.get()),
            auto_advance_answer_action: Some(self.auto_advance_answer_action.get()),
            easy_days: Some(self.easy_days.iter().map(|d| d.get()).collect()),
            // Name and parameters are not edited here (read-only); leave absent.
            name: None,
            fsrs_parameters: None,
        })
    }
}

/// A titled group of option rows within the settings form.
#[component]
fn options_group(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="deck-options-group">
            <h3 class="deck-options-group__title">{title}</h3>
            <div class="deck-options-group__rows">{children()}</div>
        </div>
    }
}

/// A label + control row. The control is provided as children.
#[component]
fn field_row(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="deck-options-row">
            <span class="deck-options-row__label">{label}</span>
            <div class="deck-options-row__control">{children()}</div>
        </div>
    }
}

/// A numeric text input bound to a string signal (parsed on save).
#[component]
fn number_field(label: &'static str, value: RwSignal<String>, placeholder: &'static str) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <Input value=value input_type="number" placeholder=placeholder />
        </FieldRow>
    }
}

/// A free-text input (Anki-style step strings).
#[component]
fn text_field(label: &'static str, value: RwSignal<String>, placeholder: &'static str) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <Input value=value placeholder=placeholder />
        </FieldRow>
    }
}

/// A boolean toggle row.
#[component]
fn toggle_field(label: &'static str, checked: RwSignal<bool>) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <input type="checkbox" class="h-4 w-4 rounded border-border accent-primary" bind:checked=checked />
        </FieldRow>
    }
}

/// An enum selector row, driven by a snake-case string signal.
#[component]
fn select_field(
    label: &'static str,
    value: RwSignal<String>,
    options: &'static [(&'static str, &'static str)],
) -> impl IntoView {
    view! {
        <FieldRow label=label>
            <UiSelect
                value=Signal::derive(move || value.get())
                options=options
                on_change=Some(Callback::new(move |v: String| value.set(v)))
            />
        </FieldRow>
    }
}

/// Seven per-weekday Easy Days selectors (Monday-first).
#[component]
fn easy_days_field(days: [RwSignal<String>; 7]) -> impl IntoView {
    const NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    view! {
        <div class="deck-options-easy-days">
            {NAMES.iter().enumerate().map(|(i, name)| {
                let d = days[i];
                view! {
                    <div class="deck-options-easy-day">
                        <span class="deck-options-easy-day__label">{*name}</span>
                        <UiSelect
                            value=Signal::derive(move || d.get())
                            options=EASY_DAY_OPTIONS
                            on_change=Some(Callback::new(move |v: String| d.set(v)))
                        />
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

// --- Enum option lists (wire snake_case value, human label) ---

const NEW_GATHER_OPTIONS: &[(&str, &str)] = &[
    ("deck", "Deck"),
    ("deck_then_random_notes", "Deck, then random notes"),
    ("ascending", "Ascending position"),
    ("descending", "Descending position"),
    ("random_notes", "Random notes"),
    ("random_cards", "Random cards"),
];
const NEW_SORT_OPTIONS: &[(&str, &str)] = &[
    ("card_type_then_gathered", "Card type, then order gathered"),
    ("gathered", "Order gathered"),
    ("card_type_then_random", "Card type, then random"),
    ("random_note_then_card_type", "Random note, then card type"),
    ("random", "Random"),
];
const NEW_REVIEW_OPTIONS: &[(&str, &str)] = &[
    ("mix", "Mix with reviews"),
    ("after", "Show after reviews"),
    ("before", "Show before reviews"),
];
const REVIEW_SORT_OPTIONS: &[(&str, &str)] = &[
    ("due_then_random", "Due date, then random"),
    ("due_then_deck", "Due date, then deck"),
    ("deck_then_due", "Deck, then due date"),
    ("ascending_interval", "Ascending intervals"),
    ("descending_interval", "Descending intervals"),
    ("easy_first", "Easy first"),
    ("difficult_first", "Difficult first"),
    ("ascending_retrievability", "Ascending retrievability"),
    ("descending_retrievability", "Descending retrievability"),
    ("relative_overdueness", "Relative overdueness"),
    ("random", "Random"),
    ("order_added", "Order added"),
    ("latest_added_first", "Latest added first"),
];
const INSERTION_ORDER_OPTIONS: &[(&str, &str)] = &[
    ("sequential", "Sequential"),
    ("random", "Random"),
];
const LEECH_ACTION_OPTIONS: &[(&str, &str)] = &[
    ("tag_only", "Tag only"),
    ("suspend_card", "Suspend card"),
];
const QUESTION_ACTION_OPTIONS: &[(&str, &str)] = &[
    ("show_answer", "Show answer"),
    ("show_card", "Show card"),
];
const ANSWER_ACTION_OPTIONS: &[(&str, &str)] = &[
    ("bury_card", "Bury card"),
    ("answer_again", "Answer again"),
    ("answer_good", "Answer good"),
    ("answer_hard", "Answer hard"),
    ("show_reminder", "Show reminder"),
];
const EASY_DAY_OPTIONS: &[(&str, &str)] = &[
    ("minimum", "Minimum"),
    ("reduced", "Reduced"),
    ("normal", "Normal"),
];

/// Format desired retention (0..1) into a whole-percent string (e.g. 0.9 → "90").
fn format_desired_retention(value: f64) -> String {
    format!("{:.0}", value * 100.0)
}

/// Format a float for an editable input (drop a trailing `.0`, keep 1 dp else).
fn format_float_input(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// Format learning/relearning steps (seconds) back to Anki's compact `1m 10m`
/// notation, so the values read the way the user expects them on the wire.
fn format_steps(steps: &[i64]) -> String {
    if steps.is_empty() {
        return "(none)".to_string();
    }
    steps.iter().map(|&s| format_interval(s)).collect::<Vec<_>>().join(" ")
}

/// Format the raw FSRS weight vector (kept read-only; the optimizer is deferred).
fn format_parameters(params: &[f32]) -> String {
    params
        .iter()
        .map(|w| format!("{w:.4}"))
        .collect::<Vec<_>>()
        .join(", ")
}
