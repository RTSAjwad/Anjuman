//! Leptos shell for the Anjuman core.
//!
//! This is the web front-end. It renders the core's `ViewModel` and forwards UI
//! events into `shared::Core<Anjuman>` via a reactive `Effect`. The shell is
//! pure glue: it consumes `ViewModel` and emits `Event`, never holding logic.
//!
//! UI is built from the `thaw` component library. `ConfigProvider` wraps the
//! tree to apply the theme; plain `<div>` is preferred for layout (nesting thaw
//! components that return `impl IntoView` inside `Space`/`Card` trips an
//! `IntoFragment` error).

use leptos::prelude::*;
use shared::{Event, ViewModel};
use thaw::{Button, ButtonAppearance, ConfigProvider, Input};

use crate::core_link;

#[component]
pub fn RootComponent() -> impl IntoView {
    let core = core_link::new();
    let (view, render) = signal(core.view());
    let (event, set_event) = signal(Event::RestoreSession);

    // A place to hold the in-progress email/password while the user types
    // (`thaw`'s `Input` drives these signals directly via its `value` prop).
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());

    Effect::new(move |_| {
        core_link::update(&core, event.get(), render);
    });

    let submit = move |_| {
        set_event.set(Event::LoginSubmit {
            email: email.get(),
            password: password.get(),
        });
    };
    let logout = move |_| set_event.set(Event::Logout);

    view! {
        <ConfigProvider>
            <main style="max-width: 24rem; margin: 4rem auto; padding: 0 1rem;">
                <h1>"Anjuman"</h1>

                {move || {
                    let vm: ViewModel = view.get();
                    if vm.authenticated {
                        view! {
                            <p>"Signed in as " <strong>{vm.display_name}</strong></p>
                            <p style="color: #888;">{vm.email}</p>
                            <Button appearance=ButtonAppearance::Subtle on_click=logout>
                                "Log out"
                            </Button>
                        }.into_any()
                    } else {
                        view! {
                            <div style="display: flex; gap: 0.5rem; flex-direction: column;">
                                <Input value=email placeholder="Email" />
                                <Input value=password placeholder="Password" />
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=submit
                                    disabled=vm.busy
                                >
                                    "Log in"
                                </Button>
                                {vm.error.clone().map(|err| view! {
                                    <p style="color: #c00;">{err}</p>
                                })}
                            </div>
                        }.into_any()
                    }
                }}
            </main>
        </ConfigProvider>
    }
}
