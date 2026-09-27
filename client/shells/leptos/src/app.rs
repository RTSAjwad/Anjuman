//! Leptos shell for the Anjuman core.
//!
//! This is the web front-end. It renders the core's `ViewModel` and forwards UI
//! events into `shared::Core<Anjuman>` via a reactive `Effect`, so every
//! button click re-runs the core and re-renders from the latest view model.
//!
//! UI is built from the `thaw` component library (a Fluent-inspired kit), which
//! is purely a shell concern — it consumes `ViewModel` and emits `Event`, and
//! never touches the core. `ConfigProvider` wraps the tree to apply the theme.

use leptos::prelude::*;
use shared::Event;
use thaw::{Button, ButtonAppearance, ConfigProvider};

use crate::core_link;

#[component]
pub fn RootComponent() -> impl IntoView {
    let core = core_link::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::Reset);

    Effect::new(move |_| {
        core_link::update(&core, event.get(), render);
    });

    view! {
        <ConfigProvider>
            <main style="max-width: 24rem; margin: 4rem auto; text-align: center;">
                <h1>"Anjuman"</h1>
                <p style="font-size: 3rem; margin: 1rem 0;">{move || view.get().count}</p>
                <div style="display: flex; gap: 0.75rem; justify-content: center;">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| set_event.set(Event::Increment)
                    >
                        "Increment"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| set_event.set(Event::Decrement)
                    >
                        "Decrement"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| set_event.set(Event::Reset)
                    >
                        "Reset"
                    </Button>
                </div>
            </main>
        </ConfigProvider>
    }
}
