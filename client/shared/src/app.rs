//! Core application logic (Elm-style `Model` / `Event` / `ViewModel` / `Effect`).
//!
//! This module has **no knowledge of any UI**. It only declares what changes and
//! what side effects to perform; each shell decides *how* to render and execute.

use crux_core::{
    render::{self, RenderOperation},
    App,
    Command,
    macros::effect,
};
use facet::Facet;
use serde::{Deserialize, Serialize};

/// The entire application state.
#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub count: isize,
}

/// Actions the user (or a running effect) can trigger.
///
/// `Facet` enables foreign-language type generation; `Serialize`/`Deserialize`
/// allow the event to cross the FFI boundary as bytes. `#[repr(C)]` gives the
/// generated FFI enum a stable layout.
#[derive(Serialize, Deserialize, Facet, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
}

/// Side effects the core requests from the shell.
///
/// For now this is only `Render`. Additional capabilities (HTTP, key-value,
/// time) are added here as the app grows — each shell then implements them in
/// its own idiom.
///
/// NOTE: `#[effect(facet_typegen)]` generates the necessary traits (including
/// serialization and `Facet`) for the FFI enum itself — it must NOT be combined
/// with a manual `#[derive(Serialize, Deserialize)]`.
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
}

/// The precise state the UI needs to render.
#[derive(Serialize, Deserialize, Facet, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewModel {
    pub count: isize,
}

#[derive(Default)]
pub struct Anjuman;

impl App for Anjuman {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Increment => model.count += 1,
            Event::Decrement => model.count -= 1,
            Event::Reset => model.count = 0,
        }

        // Every update requests a UI refresh.
        render::render()
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            count: model.count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(event: Event, model: &mut Model) -> Vec<Effect> {
        let app = Anjuman::default();
        let mut command = app.update(event, model);
        command.effects().collect()
    }

    #[test]
    fn increment() {
        let mut model = Model::default();
        let effects = update(Event::Increment, &mut model);

        assert_eq!(model.count, 1);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Render(..)]
        ));
    }

    #[test]
    fn decrement() {
        let mut model = Model::default();
        let effects = update(Event::Decrement, &mut model);

        assert_eq!(model.count, -1);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Render(..)]
        ));
    }

    #[test]
    fn reset() {
        let mut model = Model { count: 42 };
        let effects = update(Event::Reset, &mut model);

        assert_eq!(model.count, 0);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Render(..)]
        ));
    }
}
