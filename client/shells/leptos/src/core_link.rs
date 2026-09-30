//! Reactive bridge between Leptos signals and the shared core.
//!
//! This is a **native Rust shell**: instead of serializing through the FFI
//! `Bridge`, it drives `shared::Core<Anjuman>` directly with the typed `Event`
//! and `ViewModel` types. (The serialized `CoreFfi`/`Bridge` path is for
//! non-Rust shells — Swift/Kotlin/C#.) This mirrors the upstream Crux
//! `web-leptos` example.

use std::rc::Rc;

use leptos::{prelude::*, task};

use shared::{Anjuman, Core, Effect, Event, ViewModel};

use crate::{http, kv};

pub type AppCore = Rc<Core<Anjuman>>;

/// Create the core, shared behind `Rc` (the WASM runtime is single-threaded).
pub fn new() -> AppCore {
    Rc::new(Core::new())
}

/// Process an event and run its effects against the given render signal.
pub fn update(core: &AppCore, event: Event, render: WriteSignal<ViewModel>) {
    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

/// Execute a single effect. `Http` and `KeyValue` capabilities are performed by
/// this shell (WASM: `gloo-net` fetch + `web-sys` localStorage) and their
/// results fed back into the core via `resolve`.
fn process_effect(core: &AppCore, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }

        Effect::Http(mut request) => {
            task::spawn_local({
                let core = core.clone();
                async move {
                    let response = http::request(&request.operation).await;
                    for effect in core
                        .resolve(&mut request, response.into())
                        .expect("should resolve http")
                    {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }

        Effect::KeyValue(mut request) => {
            task::spawn_local({
                let core = core.clone();
                async move {
                    let result = kv::execute(&request.operation).await;
                    for effect in core
                        .resolve(&mut request, result)
                        .expect("should resolve kv")
                    {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }
    }
}
