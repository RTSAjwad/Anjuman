//! Reactive bridge between Leptos signals and the shared core.
//!
//! This is a **native Rust shell**: instead of serializing through the FFI
//! `Bridge`, it drives `shared::Core<Anjuman>` directly with the typed `Event`
//! and `ViewModel` types. (The serialized `CoreFfi`/`Bridge` path is for
//! non-Rust shells — Swift/Kotlin/C#.) This mirrors the upstream Crux
//! `web-leptos` example.

use std::rc::Rc;

use leptos::prelude::{Update as _, WriteSignal};

use shared::{Core, Anjuman, Effect, Event, ViewModel};

pub type AppCore = Rc<Core<Anjuman>>;

/// Create the core, shared behind `Rc` (the WASM runtime is single-threaded).
pub fn new() -> AppCore {
    Rc::new(Core::new())
}

/// Process an event and run its effects against the given render signal.
pub fn update(core: &AppCore, event: Event, render: WriteSignal<ViewModel>) {
    for effect in &core.process_event(event) {
        process_effect(core, effect, render);
    }
}

fn process_effect(core: &AppCore, effect: &Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }
    }
}
