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
use crux_http::{command::Http, protocol::HttpRequest};
use facet::Facet;
use serde::{Deserialize, Serialize};

use anjuman_contracts::shared::MessageResponse;

/// The entire application state.
#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub count: isize,
    /// The last server health response (from `GET /health`), if any.
    pub health_status: Option<String>,
}

/// Actions the user (or a running effect) can trigger.
///
/// `Facet` enables foreign-language type generation; `Serialize`/`Deserialize`
/// allow the event to cross the FFI boundary as bytes. `#[repr(C)]` gives the
/// generated FFI enum a stable layout.
#[derive(Serialize, Deserialize, Facet, Debug, Clone)]
#[repr(C)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
    /// Ask the server whether it is up (exercises the HTTP capability + the
    /// `anjuman_contracts` shared-type path).
    CheckHealth,

    /// The result of the health check — local to the core, never crosses the
    /// FFI boundary (the shell forwards the raw response bytes via `resolve`).
    #[serde(skip)]
    #[facet(skip)]
    SetHealth(#[facet(opaque)] crux_http::Result<crux_http::Response<MessageResponse>>),
}

/// Side effects the core requests from the shell.
///
/// `Http` carries an `HttpRequest` the shell must perform (fetch, then feed the
/// response bytes back via `core.resolve`); `Render` asks for a UI refresh.
/// Key-value/time capabilities are added as later features need them.
///
/// NOTE: `#[effect(facet_typegen)]` generates the necessary traits (including
/// serialization and `Facet`) for the FFI enum itself — it must NOT be combined
/// with a manual `#[derive(Serialize, Deserialize)]`.
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
}

/// The precise state the UI needs to render.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq)]
pub struct ViewModel {
    pub count: isize,
    pub health_status: Option<String>,
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
            Event::CheckHealth => {
                return Http::get("http://localhost:3000/health")
                    .expect_json()
                    .build()
                    .then_send(Event::SetHealth);
            }
            Event::SetHealth(Ok(mut response)) => {
                let body = response.take_body().expect("health response has a body");
                model.health_status = Some(body.message);
            }
            Event::SetHealth(Err(_)) => {
                model.health_status = Some("server unreachable".to_string());
            }
        }

        // Every update requests a UI refresh.
        render::render()
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            count: model.count,
            health_status: model.health_status.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_http::testing::ResponseBuilder;

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
        let mut model = Model { count: 42, health_status: None };
        let effects = update(Event::Reset, &mut model);

        assert_eq!(model.count, 0);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Render(..)]
        ));
    }

    // --------------------------------------------------------------------
    // US-4.1 — client plumbing (contracts + HTTP capability)
    // --------------------------------------------------------------------

    /// A canned JSON body deserializes into an `anjuman_contracts` DTO,
    /// proving the shared-type path the core will use for every API call.
    #[test]
    fn contract_dto_deserializes_from_json() {
        let json = r#"{"message":"ok"}"#;
        let msg: anjuman_contracts::shared::MessageResponse =
            serde_json::from_str(json).expect("deserialize contract DTO");
        assert_eq!(msg.message, "ok");
    }

    /// `CheckHealth` emits exactly one `Http` effect carrying a GET to the
    /// server's health endpoint — the two-event HTTP idiom's first half.
    #[test]
    fn check_health_emits_http_request() {
        let mut model = Model::default();
        let effects = update(Event::CheckHealth, &mut model);

        match effects.as_slice() {
            [Effect::Http(request)] => {
                assert_eq!(request.operation.method.as_str(), "GET");
                assert_eq!(
                    request.operation.url.as_str(),
                    "http://localhost:3000/health"
                );
            }
            other => panic!("expected a single Http effect, got {} effect(s)", other.len()),
        }
    }

    /// A successful (canned) response updates `model.health_status` from the
    /// deserialized contract DTO and requests a render — the idiom's second half.
    #[test]
    fn health_response_updates_model_and_renders() {
        let mut model = Model::default();
        let response = ResponseBuilder::ok()
            .body(MessageResponse { message: "ok".to_string() })
            .build();
        let effects = update(Event::SetHealth(Ok(response)), &mut model);

        assert_eq!(model.health_status.as_deref(), Some("ok"));
        assert!(matches!(
            effects.as_slice(),
            [Effect::Render(..)]
        ));

        // The view reflects it too.
        let view = Anjuman.view(&model);
        assert_eq!(view.health_status.as_deref(), Some("ok"));
    }

    /// A rejected health check is surfaced as "server unreachable" rather than
    /// panicking.
    #[test]
    fn health_rejection_marks_unreachable() {
        let mut model = Model::default();
        // Build an HTTP rejection the same way `crux_http` would deliver one.
        let err = crux_http::testing::rejection::<MessageResponse>(
            503,
            r#"{"message":"down"}"#,
        )
        .unwrap_err();
        let effects = update(Event::SetHealth(Err(err)), &mut model);

        assert_eq!(model.health_status.as_deref(), Some("server unreachable"));
        assert!(matches!(effects.as_slice(), [Effect::Render(..)]));
    }
}
