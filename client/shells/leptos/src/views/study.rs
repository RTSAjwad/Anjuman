use leptos::prelude::*;
use shared::app::{StudyCardView, StudyCountsView};
use shared::{Event, ViewModel};

use crate::components::{
    UiBadge, UiBadgeColor, UiButton, UiButtonAppearance, UiDialog, UiInput,
};
use lepticons::{Icon, LucideGlyph};

/// The study screen: one card at a time, centred in the available space, with
/// the reveal/answer controls pinned to the bottom. Due counts live in the
/// header (see `study_count_badges`); the deck title is in the breadcrumb. Pure
/// rendering of the `ViewModel` snapshot — study state (`StartStudy` gating,
/// reveal reset) lives in `RootComponent`.
#[component]
pub(crate) fn study_screen(
    vm: ViewModel,
    revealed: RwSignal<bool>,
    set_event: WriteSignal<Event>,
) -> impl IntoView {
    let current = vm.current_card;
    let study_error = vm.study_error;
    let card_action_error = vm.card_action_error;
    let busy = vm.busy;

    view! {
        <div class="study-screen">
            {study_error.map(|err| view! {
                <p style="color: #c00;">{err}</p>
            })}
            {card_action_error.map(|err| view! {
                <p style="color: #c00;">{err}</p>
            })}

            <div class="study-screen__body">
                {match current {
                    Some(card) => study_card_view(card, revealed, set_event).into_any(),
                    None => {
                        if busy {
                            view! {
                                <p style="color: #888;">"Loading…"</p>
                            }
                            .into_any()
                        } else {
                            study_done_view(set_event).into_any()
                        }
                    }
                }}
            </div>
        </div>
    }
}

/// The due-count badges shown in the header, next to the breadcrumb.
pub(crate) fn study_count_badges(counts: StudyCountsView) -> impl IntoView {
    view! {
        <div class="thaw-study-counts">
            <UiBadge color=UiBadgeColor::Brand>
                {counts.new_count.to_string()}
            </UiBadge>
            <UiBadge color=UiBadgeColor::Danger>
                {(counts.learning_count + counts.relearning_count).to_string()}
            </UiBadge>
            <UiBadge color=UiBadgeColor::Success>
                {counts.review_count.to_string()}
            </UiBadge>
        </div>
    }
}

/// Study-card actions shown in the top bar while a card is in hand: Bury,
/// Suspend (card + note), and Reschedule (set due date), collected into a single
/// "⋮" dropdown menu (the flag picker is a separate `FlagDropdown` next to it).
/// Each action forwards a core `Event` using the current card; the core owns the
/// HTTP + state. Reschedule is a normal menu item that opens a `UiDialog` for the
/// day offset. The menu uses the same top-layer `position: fixed` + anchor
/// mechanism as `FlagDropdown` so it isn't clipped by the header's layout.
#[component]
pub(crate) fn StudyControls(card: StudyCardView, set_event: WriteSignal<Event>) -> impl IntoView {
    // Reschedule: a day offset the user types, then confirms.
    let days = RwSignal::new(String::new());
    let reschedule_open = RwSignal::new(false);
    let reschedule = move |_| {
        if let Ok(d) = days.get().trim().parse::<i64>() {
            set_event.set(Event::RescheduleCard { days: d });
            days.set(String::new());
            reschedule_open.set(false);
        }
    };

    let buried = card.buried;
    let suspended = card.suspended;

    let open = RwSignal::new(false);
    let toggle = move |_| open.update(|o| *o = !*o);
    let close = move || open.set(false);

    let menu_style = "position: fixed; position-anchor: --study-actions-anchor; position-area: block-end span-inline-start;";

    view! {
        <div>
            <div tabindex="0" role="button" class="inline-flex size-9 items-center justify-center rounded-md hover:bg-accent" aria-label="Card actions" style="anchor-name:--study-actions-anchor" on:click=toggle>
                <Icon glyph=LucideGlyph::EllipsisVertical size="20" />
            </div>
            <ul
                class="z-50 flex w-52 flex-col gap-0 rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md transition-all duration-200 data-[state=closed]:pointer-events-none data-[state=closed]:scale-95 data-[state=closed]:opacity-0 data-[state=open]:scale-100 data-[state=open]:opacity-100"
                style=menu_style
                data-state=move || if open.get() { "open" } else { "closed" }
            >
                {if buried {
                    view! {
                        <li><button class="flex w-full rounded-sm px-3 py-1.5 text-sm hover:bg-accent" on:click={ let close = close.clone(); move |_| { close(); set_event.set(Event::UnburyCard); } }>"Unbury"</button></li>
                    }.into_any()
                } else {
                    view! {
                        <li><button class="flex w-full rounded-sm px-3 py-1.5 text-sm hover:bg-accent" on:click={ let close = close.clone(); move |_| { close(); set_event.set(Event::BuryCard); } }>"Bury"</button></li>
                    }.into_any()
                }}
                <li><button class="flex w-full rounded-sm px-3 py-1.5 text-sm hover:bg-accent" on:click={ let close = close.clone(); move |_| { close(); set_event.set(Event::BuryNote); } }>"Bury note"</button></li>
                {if suspended {
                    view! {
                        <li><button class="flex w-full rounded-sm px-3 py-1.5 text-sm hover:bg-accent" on:click={ let close = close.clone(); move |_| { close(); set_event.set(Event::UnsuspendCard); } }>"Unsuspend"</button></li>
                    }.into_any()
                } else {
                    view! {
                        <li><button class="flex w-full rounded-sm px-3 py-1.5 text-sm hover:bg-accent" on:click={ let close = close.clone(); move |_| { close(); set_event.set(Event::SuspendCard); } }>"Suspend"</button></li>
                    }.into_any()
                }}
                <li><button class="flex w-full rounded-sm px-3 py-1.5 text-sm hover:bg-accent" on:click={ let close = close.clone(); move |_| { close(); set_event.set(Event::SuspendNote); } }>"Suspend note"</button></li>

                <li>
                    <button class="flex w-full rounded-sm px-3 py-1.5 text-sm hover:bg-accent" on:click={ let close = close.clone(); move |_| { close(); reschedule_open.set(true); } }>"Reschedule"</button>
                </li>
            </ul>
        </div>

        <UiDialog open=reschedule_open.into()>
            <h3 class="text-lg font-bold">"Reschedule"</h3>
            <p class="py-4">"How many days from now?"</p>
            <UiInput value=days input_type="number" placeholder="days" />
            <div class="mt-6 flex justify-end gap-2">
                <UiButton appearance=UiButtonAppearance::Subtle on_click=move |_| reschedule_open.set(false)>"Cancel"</UiButton>
                <UiButton appearance=UiButtonAppearance::Primary on_click=reschedule>"Reschedule"</UiButton>
            </div>
        </UiDialog>
    }
}

/// A single card: the front (then the revealed back) rendered as HTML in a
/// sandboxed iframe, with the reveal/answer controls pinned to the bottom.
fn study_card_view(
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
        <div class="study-card-view">
            <iframe
                class="study-card-view__iframe"
                sandbox=""
                srcdoc=move || {
                    if revealed.get() {
                        card_html(&back, &styling)
                    } else {
                        card_html(&front, &styling)
                    }
                }
            ></iframe>

            <div class="study-card-view__controls">
                {move || {
                    if revealed.get() {
                        view! {
                            <div class="thaw-study-answers">
                                {answer_button(rating.clone(), interval_for.clone(), 1, "Again")}
                                {answer_button(rating.clone(), interval_for.clone(), 2, "Hard")}
                                {answer_button(rating.clone(), interval_for.clone(), 3, "Good")}
                                {answer_button(rating.clone(), interval_for.clone(), 4, "Easy")}
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {
                            <UiButton appearance=UiButtonAppearance::Neutral on_click=move |_| revealed.set(true)>
                                "Show answer"
                            </UiButton>
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
/// answer (rating 1) renders as a soft danger/error button; Hard/Good/Easy are
/// soft default buttons.
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
    // "Again" (rating 1) is a soft danger/error button; Hard/Good/Easy are soft
    // neutral buttons.
    let (appearance, extra_class) = if n == 1 {
        (UiButtonAppearance::Danger, "btn-soft")
    } else {
        (UiButtonAppearance::NeutralSoft, "")
    };

    view! {
        <div class="thaw-study-answer">
            <UiButton appearance=appearance class=extra_class on_click=move |_| rating(n)>
                {label}
            </UiButton>
            <span class="thaw-study-answer__interval">{interval}</span>
        </div>
    }
}

/// Format an interval in seconds using Anki's interval-unit notation:
/// `<1m` → `60s` style, minutes, hours, days, months, years.
pub(crate) fn format_interval(seconds: i64) -> String {
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

/// The "nothing due" completion state (and a manual re-`StartStudy` affordance).
fn study_done_view(set_event: WriteSignal<Event>) -> impl IntoView {
    view! {
        <div style="display: flex; flex-direction: column; gap: 0.75rem; align-items: flex-start;">
            <p style="color: #888;">"Nothing due right now. Nice work!"</p>
            <UiButton appearance=UiButtonAppearance::Primary on_click=move |_| set_event.set(Event::StartStudy)>
                "Check again"
            </UiButton>
        </div>
    }
}
