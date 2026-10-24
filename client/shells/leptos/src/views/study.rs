use leptos::prelude::*;
use shared::app::{StudyCardView, StudyCountsView};
use shared::{Event, ViewModel};

use crate::components::{
    Badge, BadgeVariant, Button, ButtonVariant, Dialog, FloatingLayer, Input, Select, StudyCard,
};
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
                    Some(card) => view! { <StudyCard card=card revealed=revealed set_event=set_event /> }.into_any(),
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
        <div class="study-counts">
            <Badge variant=BadgeVariant::Default>
                {counts.new_count.to_string()}
            </Badge>
            <Badge variant=BadgeVariant::Destructive>
                {(counts.learning_count + counts.relearning_count).to_string()}
            </Badge>
            <Badge variant=BadgeVariant::Success>
                {counts.review_count.to_string()}
            </Badge>
        </div>
    }
}

/// Study-card actions shown in the top bar while a card is in hand: Bury,
/// Suspend (card + note), and Reschedule (set due date), collected into a single
/// "⋮" dropdown menu, plus a flag `<select>` next to it. Each action forwards a
/// core `Event` using the current card; the core owns the HTTP + state.
/// Reschedule is a normal menu item that opens a `Dialog` for the day offset.
/// The dropdown follows the rust-ui/leptos-ui `DropdownMenu` styling (content +
/// item classes) but is positioned via CSS anchor so it isn't clipped by the
/// header's layout.
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
    let flag = card.flag;

    // The "⋮" study-actions dropdown. Built on `FloatingLayer` (click-outside/Esc/
    // scroll-lock + unique CSS anchor), styled with rust-ui's `DropdownMenuItem`
    // class. Each action closes the menu then emits the core `Event`.
    let item_class = "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm transition-colors duration-200 text-popover-foreground hover:bg-accent hover:text-accent-foreground";

    let trigger = Box::new(move |_open: Signal<bool>, _toggle: Callback<leptos::ev::MouseEvent>| {
        view! {
            <div
                tabindex="0"
                role="button"
                class="inline-flex size-9 items-center justify-center rounded-md hover:bg-accent"
                aria-label="Card actions"
            >
                <Icon glyph=LucideGlyph::EllipsisVertical size="20" />
            </div>
        }
        .into_any()
    });

    let content = {
        Box::new(move |_open: Signal<bool>, close: Callback<()>| {
            let item = item_class;
            view! {
                <ul class="m-0 w-44 list-none rounded-md border bg-card p-1 shadow-md">
                    <li><button class=item on:click={ let close = close.clone(); move |_| { close.run(()); set_event.set(if buried { Event::UnburyCard } else { Event::BuryCard }); } }>{if buried { "Unbury" } else { "Bury" }}</button></li>
                    <li><button class=item on:click={ let close = close.clone(); move |_| { close.run(()); set_event.set(Event::BuryNote); } }>"Bury note"</button></li>
                    <li><button class=item on:click={ let close = close.clone(); move |_| { close.run(()); set_event.set(if suspended { Event::UnsuspendCard } else { Event::SuspendCard }); } }>{if suspended { "Unsuspend" } else { "Suspend" }}</button></li>
                    <li><button class=item on:click={ let close = close.clone(); move |_| { close.run(()); set_event.set(Event::SuspendNote); } }>"Suspend note"</button></li>
                    <li><button class=item on:click={ let close = close.clone(); move |_| { close.run(()); reschedule_open.set(true); } }>"Reschedule"</button></li>
                </ul>
            }
            .into_any()
        })
    };

    view! {
        <div class="flex items-center gap-1">
            <FloatingLayer
                content_class="rounded-md border bg-card shadow-md"
                trigger=trigger
                content=content
            />

            <Select
                value=Signal::derive(move || flag.to_string())
                options=FLAG_OPTIONS
                on_change=Some(Callback::new(move |f: String| {
                    let flag = f.parse::<i64>().unwrap_or(0);
                    set_event.set(Event::FlagCard { flag: flag as i32 });
                }))
            />
        </div>

        <Dialog open=reschedule_open.into()>
            <h3 class="text-lg font-bold">"Reschedule"</h3>
            <p class="py-4">"How many days from now?"</p>
            <Input value=days input_type="number" placeholder="days" />
            <div class="mt-6 flex justify-end gap-2">
                <Button variant=ButtonVariant::Ghost on_click=move |_| reschedule_open.set(false)>"Cancel"</Button>
                <Button variant=ButtonVariant::Default on_click=reschedule>"Reschedule"</Button>
            </div>
        </Dialog>
    }
}

/// The "nothing due" completion state (and a manual re-`StartStudy` affordance).
fn study_done_view(set_event: WriteSignal<Event>) -> impl IntoView {
    view! {
        <div style="display: flex; flex-direction: column; gap: 0.75rem; align-items: flex-start;">
            <p style="color: #888;">"Nothing due right now. Nice work!"</p>
            <Button variant=ButtonVariant::Default on_click=move |_| set_event.set(Event::StartStudy)>
                "Check again"
            </Button>
        </div>
    }
}
