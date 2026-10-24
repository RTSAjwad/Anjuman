//! StudyCard — the sandboxed-iframe card surface + reveal/answer controls
//! (§10.4 #1). Anjuman-specific: rust-ui has no iframe-sandboxed card renderer.
//! Shaped to rust-ui conventions (`data-name`, `tw_merge!` utility styling,
//! one component per file).

use leptos::prelude::*;
use shared::app::StudyCardView;
use shared::Event;

use super::button::{Button, ButtonVariant};

/// A single study card: the front (then the revealed back) rendered as HTML in a
/// sandboxed iframe, with the reveal/answer controls pinned to the bottom.
#[component]
pub fn StudyCard(
    card: StudyCardView,
    revealed: RwSignal<bool>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let rating = move |r: i32| {
        set_event.set(Event::Answer { rating: r });
    };
    // Predicted intervals (rating -> seconds), if the server returned them.
    let intervals = card.predicted_interval.unwrap_or_default();
    let interval_for = move |r: i32| intervals.get(&r).copied();

    let front = card.front;
    let back = card.back;
    let styling = card.styling;

    view! {
        <div data-name="StudyCard" class="flex min-h-0 flex-1 flex-col">
            <iframe
                class="w-full min-h-0 flex-1 border-none bg-white"
                sandbox=""
                srcdoc=move || {
                    if revealed.get() {
                        card_html(&back, &styling)
                    } else {
                        card_html(&front, &styling)
                    }
                }
            ></iframe>

            <div class="flex shrink-0 justify-center gap-2 px-6 py-4">
                {move || {
                    if revealed.get() {
                        view! {
                            <div class="flex flex-wrap gap-2">
                                {answer_button(rating.clone(), interval_for.clone(), 1, "Again")}
                                {answer_button(rating.clone(), interval_for.clone(), 2, "Hard")}
                                {answer_button(rating.clone(), interval_for.clone(), 3, "Good")}
                                {answer_button(rating.clone(), interval_for.clone(), 4, "Easy")}
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {
                            <Button variant=ButtonVariant::Secondary on_click=move |_| revealed.set(true)>
                                "Show answer"
                            </Button>
                        }
                        .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Wrap a card-side HTML fragment (the template-rendered front/back) in a
/// minimal HTML document so it renders standalone in the sandboxed iframe,
/// injecting the note type's shared styling CSS so the card is self-contained.
fn card_html(body: &str, styling: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\" /><style>{styling}</style></head><body>{body}</body></html>"
    )
}

/// A single answer button (label + predicted interval underneath). The "Again"
/// answer (rating 1) renders as a destructive button; Hard/Good/Easy are ghost
/// (quiet) buttons.
fn answer_button<R, I>(
    rating: R,
    interval_for: I,
    n: i32,
    label: &'static str,
) -> impl IntoView
where
    R: Fn(i32) + Copy + Send + Sync + 'static,
    I: Fn(i32) -> Option<i64> + Clone,
{
    let interval = interval_for(n).map(format_interval).unwrap_or_default();
    let variant = if n == 1 {
        ButtonVariant::Destructive
    } else {
        ButtonVariant::Ghost
    };

    view! {
        <div data-name="StudyAnswerButton" class="flex flex-col items-center gap-0.5">
            <Button variant=variant on_click=move |_| rating(n)>
                {label}
            </Button>
            <span class="text-xs text-muted-foreground">{interval}</span>
        </div>
    }
}

/// Format an interval in seconds using Anki's interval-unit notation:
/// `<1m` → `60s` style, minutes, hours, days, months, years.
pub fn format_interval(seconds: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    const MONTH: i64 = 30 * DAY;
    const YEAR: i64 = 365 * DAY;

    if seconds < MINUTE {
        format!("{seconds}s")
    } else if seconds < HOUR {
        format!("{}m", seconds / MINUTE)
    } else if seconds < DAY {
        format!("{}h", seconds / HOUR)
    } else if seconds < MONTH {
        format!("{}d", seconds / DAY)
    } else if seconds < YEAR {
        format!("{}mo", seconds / MONTH)
    } else {
        format!("{}y", seconds / YEAR)
    }
}
