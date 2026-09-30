//! Core application logic (Elm-style `Model` / `Event` / `ViewModel` / `Effect`).
//!
//! This module has **no knowledge of any UI**. It only declares what changes and
//! what side effects to perform; each shell decides *how* to render and execute.

use crux_core::{
    Command,
    macros::effect,
    render::{self, RenderOperation},
};
use crux_http::{command::Http, protocol::HttpRequest};
use crux_kv::protocol::KeyValueOperation;
use facet::Facet;
use serde::{Deserialize, Serialize};

use anjuman_contracts::auth::{LoginRequest, LoginResponse, UserResponse};

/// The base URL of the Anjuman server.
const API_URL: &str = "http://localhost:3000";

/// The key under which the JWT is persisted (by the shell's KV capability).
const TOKEN_KEY: &str = "auth_token";

/// The entire application state.
#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    /// The authenticated session, if any.
    pub auth: Auth,
    /// True while a login/restore round-trip is in flight (to show a spinner).
    pub busy: bool,
    /// A human-readable error from the last auth attempt, if any.
    pub error: Option<String>,
    /// Transient token carried from `RestoreSession` → `/me` → `MeResult` (the
    /// token is known at restore time but the user comes back from `/me`).
    pub pending_token: Option<String>,
}

/// Authentication state.
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub enum Auth {
    #[default]
    Unauthenticated,
    Authenticated {
        token: String,
        user: UserResponse,
    },
}

/// Actions the user (or a running effect) can trigger.
///
/// The `#[serde(skip)]`/`#[facet(skip)]` events are *core-local*: they carry
/// deserialized capability responses and never cross the FFI boundary (the shell
/// forwards raw bytes via `resolve`, and the core constructs these itself).
#[derive(Serialize, Deserialize, Facet, Debug, Clone)]
#[repr(C)]
pub enum Event {
    /// Submit the login form.
    LoginSubmit {
        email: String,
        password: String,
    },
    /// Attempt to restore a persisted session on startup.
    RestoreSession,
    /// Sign out (clear the stored token + drop session state).
    Logout,

    // --- Core-local completion events (never cross the FFI boundary) ---

    #[serde(skip)]
    #[facet(skip)]
    LoginResult(#[facet(opaque)] crux_http::Result<crux_http::Response<LoginResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    MeResult(#[facet(opaque)] crux_http::Result<crux_http::Response<UserResponse>>),

    #[serde(skip)]
    #[facet(skip)]
    TokenRead(#[facet(opaque)] crux_kv::DataResult),

    #[serde(skip)]
    #[facet(skip)]
    TokenStored(#[facet(opaque)] crux_kv::DataResult),

    #[serde(skip)]
    #[facet(skip)]
    TokenDeleted(#[facet(opaque)] crux_kv::DataResult),
}

/// Side effects the core requests from the shell.
///
/// `Http` carries an `HttpRequest` (performed by the shell via `crux_http`);
/// `KeyValue` carries a `KeyValueOperation` (persisted by the shell via
/// `crux_kv`); `Render` asks for a UI refresh.
///
/// NOTE: `#[effect(facet_typegen)]` generates the necessary traits (including
/// serialization and `Facet`) for the FFI enum itself — it must NOT be combined
/// with a manual `#[derive(Serialize, Deserialize)]`.
#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    KeyValue(KeyValueOperation),
}

/// The precise state the UI needs to render.
#[derive(Serialize, Deserialize, Facet, Default, Clone, PartialEq, Eq, Debug)]
pub struct ViewModel {
    pub email: String,
    pub authenticated: bool,
    /// The signed-in user's display name, when authenticated.
    pub display_name: String,
    pub busy: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Anjuman;

impl crux_core::App for Anjuman {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::LoginSubmit { email, password } => {
                model.busy = true;
                model.error = None;

                let body = LoginRequest { email, password };
                let call_api = Http::post(format!("{API_URL}/auth/login"))
                    .body_json(&body)
                    .expect("serialize login request")
                    .expect_json()
                    .build()
                    .then_send(Event::LoginResult);

                render::render().and(call_api)
            }

            Event::LoginResult(Ok(mut response)) => {
                let login = response.take_body().expect("login response has a body");
                model.auth = Auth::Authenticated {
                    token: login.token.clone(),
                    user: login.user.clone(),
                };
                model.busy = false;
                model.error = None;

                // Persist the token so the session survives a reload.
                let store = crux_kv::KeyValue::set(TOKEN_KEY, login.token.into_bytes())
                    .then_send(Event::TokenStored);

                render::render().and(store)
            }

            Event::LoginResult(Err(e)) => {
                model.busy = false;
                model.error = Some(e.to_string());
                render::render()
            }

            Event::RestoreSession => {
                // Read the persisted token; `TokenRead` decides whether to call
                // `/me` with it.
                crux_kv::KeyValue::get(TOKEN_KEY).then_send(Event::TokenRead)
            }

            Event::TokenRead(result) => {
                let token = match result {
                    Ok(Some(bytes)) => String::from_utf8(bytes).ok(),
                    _ => None,
                };

                match token {
                    Some(token) => {
                        // Carry the token across the `/me` round-trip, then
                        // validate it against /me.
                        model.pending_token = Some(token.clone());
                        model.busy = true;
                        Http::get(format!("{API_URL}/me"))
                            .header("authorization", format!("Bearer {token}"))
                            .expect_json()
                            .build()
                            .then_send(Event::MeResult)
                    }
                    None => render::render(), // not signed in; nothing to do
                }
            }

            Event::MeResult(Ok(mut response)) => {
                let user = response.take_body().expect("me response has a body");
                let token = model.pending_token.take().unwrap_or_default();
                model.auth = Auth::Authenticated { token, user };
                model.busy = false;
                render::render()
            }

            Event::MeResult(Err(_)) => {
                // Token invalid/expired: clear it and return to signed-out.
                model.pending_token = None;
                model.auth = Auth::Unauthenticated;
                model.busy = false;
                let clear = crux_kv::KeyValue::delete(TOKEN_KEY).then_send(Event::TokenDeleted);
                render::render().and(clear)
            }

            Event::Logout => {
                model.auth = Auth::Unauthenticated;
                model.busy = false;
                model.error = None;
                let clear = crux_kv::KeyValue::delete(TOKEN_KEY).then_send(Event::TokenDeleted);
                render::render().and(clear)
            }

            // Completion events whose only purpose was to run after the KV write
            // (no further state change needed — the render already happened).
            Event::TokenStored(_) | Event::TokenDeleted(_) => Command::done(),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        match &model.auth {
            Auth::Authenticated { user, .. } => ViewModel {
                email: user.email.clone(),
                authenticated: true,
                display_name: format!("{} {}", user.first_name, user.last_name),
                busy: model.busy,
                error: model.error.clone(),
            },
            Auth::Unauthenticated => ViewModel {
                email: String::new(),
                authenticated: false,
                display_name: String::new(),
                busy: model.busy,
                error: model.error.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anjuman_contracts::UserRole;
    use crux_core::App as _;
    use crux_http::testing::ResponseBuilder;

    fn update(event: Event, model: &mut Model) -> Vec<Effect> {
        let app = Anjuman::default();
        let mut command = app.update(event, model);
        command.effects().collect()
    }

    // --------------------------------------------------------------------
    // US-4.2 — auth flow
    // --------------------------------------------------------------------

    /// `LoginSubmit` emits a POST to `/auth/login` with a JSON `LoginRequest`
    /// body (plus a render).
    #[test]
    fn login_submit_emits_post_with_json_body() {
        let mut model = Model::default();
        let effects = update(
            Event::LoginSubmit {
                email: "a@b.c".to_string(),
                password: "pw".to_string(),
            },
            &mut model,
        );

        assert!(model.busy);
        let http = effects
            .iter()
            .find_map(|e| match e {
                Effect::Http(req) => Some(&req.operation),
                _ => None,
            })
            .expect("expected an Http effect");

        assert_eq!(http.method.as_str(), "POST");
        assert_eq!(http.url.as_str(), "http://localhost:3000/auth/login");
        // The JSON body carries the credentials.
        let body: LoginRequest =
            serde_json::from_slice(&http.body).expect("login body is JSON");
        assert_eq!(body.email, "a@b.c");
        assert_eq!(body.password, "pw");
    }

    /// A successful login stores `token` + `user` and emits a KV `set` for the
    /// token.
    #[test]
    fn login_success_stores_token_and_emits_kv_set() {
        let mut model = Model::default();
        let response = ResponseBuilder::ok()
            .body(LoginResponse {
                token: "jwt-token".to_string(),
                user: UserResponse {
                    id: 1,
                    email: "a@b.c".to_string(),
                    first_name: "Ada".to_string(),
                    last_name: "Lovelace".to_string(),
                    role: UserRole::Student,
                    school_id: 1,
                },
            })
            .build();
        let effects = update(Event::LoginResult(Ok(response)), &mut model);

        match &model.auth {
            Auth::Authenticated { token, user } => {
                assert_eq!(token, "jwt-token");
                assert_eq!(user.email, "a@b.c");
            }
            other => panic!("expected authenticated, got {other:?}"),
        }
        // KV set effect for the token (alongside a render).
        assert!(
            effects.iter().any(|e| matches!(e,
                Effect::KeyValue(op)
                    if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Set { ref key, .. } if key == TOKEN_KEY))),
            "expected a KeyValue Set effect for the token"
        );
    }

    /// A login rejection leaves the model unauthenticated and surfaces an error.
    #[test]
    fn login_rejection_leaves_unauthenticated() {
        let mut model = Model::default();
        let err = crux_http::testing::rejection::<LoginResponse>(401, "invalid")
            .unwrap_err();
        let effects = update(Event::LoginResult(Err(err)), &mut model);

        assert!(matches!(model.auth, Auth::Unauthenticated));
        assert!(!model.busy);
        assert!(model.error.is_some());
        assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
    }

    /// `RestoreSession` reads the token from KV.
    #[test]
    fn restore_session_reads_token_from_kv() {
        let mut model = Model::default();
        let effects = update(Event::RestoreSession, &mut model);

        assert!(matches!(
            effects.as_slice(),
            [Effect::KeyValue(op)]
                if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Get { ref key } if key == TOKEN_KEY)
        ));
    }

    /// A stored token triggers a `GET /me` with the `Authorization: Bearer`
    /// header.
    #[test]
    fn stored_token_triggers_me_with_bearer_header() {
        let mut model = Model::default();
        // KV `get` resolves to `DataResult` = `Result<Option<Vec<u8>>, _>`.
        let effects = update(Event::TokenRead(Ok(Some(b"jwt-token".to_vec()))), &mut model);

        match effects.as_slice() {
            [Effect::Http(req)] => {
                assert_eq!(req.operation.method.as_str(), "GET");
                assert_eq!(req.operation.url.as_str(), "http://localhost:3000/me");
                let auth = req
                    .operation
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("authorization"))
                    .expect("authorization header present");
                assert_eq!(auth.value, "Bearer jwt-token");
            }
            other => panic!("expected one Http effect, got {other:?}"),
        }
    }

    /// `/me` success stores the `UserResponse` (with the pending token); `/me`
    /// 401 clears the token and returns to signed-out.
    #[test]
    fn me_success_and_401() {
        // Success path: restore set `pending_token`; `/me` succeeds and combines
        // it with the returned user.
        let mut model = Model {
            pending_token: Some("jwt-token".to_string()),
            ..Model::default()
        };
        let ok = ResponseBuilder::ok()
            .body(UserResponse {
                id: 7,
                email: "me@b.c".to_string(),
                first_name: "Grace".to_string(),
                last_name: "Hopper".to_string(),
                role: UserRole::Teacher,
                school_id: 1,
            })
            .build();
        let _ = update(Event::MeResult(Ok(ok)), &mut model);
        match &model.auth {
            Auth::Authenticated { token, user } => {
                assert_eq!(token, "jwt-token");
                assert_eq!(user.email, "me@b.c");
            }
            other => panic!("expected authenticated, got {other:?}"),
        }

        // 401 path: clears token + emits KV delete.
        let err = crux_http::testing::rejection::<UserResponse>(401, "expired").unwrap_err();
        let effects = update(Event::MeResult(Err(err)), &mut model);
        assert!(matches!(model.auth, Auth::Unauthenticated));
        assert!(
            effects.iter().any(|e| matches!(e,
                Effect::KeyValue(op)
                    if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Delete { ref key } if key == TOKEN_KEY))),
            "expected a KeyValue Delete effect for the token"
        );
    }

    /// `Logout` clears the session and emits a KV delete for the token.
    #[test]
    fn logout_clears_session_and_deletes_token() {
        let mut model = Model {
            auth: Auth::Authenticated {
                token: "jwt-token".to_string(),
                user: UserResponse {
                    id: 1,
                    email: "a@b.c".to_string(),
                    first_name: "Ada".to_string(),
                    last_name: "Lovelace".to_string(),
                    role: UserRole::Student,
                    school_id: 1,
                },
            },
            ..Model::default()
        };
        let effects = update(Event::Logout, &mut model);

        assert!(matches!(model.auth, Auth::Unauthenticated));
        assert!(
            effects.iter().any(|e| matches!(e,
                Effect::KeyValue(op)
                    if matches!(op.operation, crux_kv::protocol::KeyValueOperation::Delete { ref key } if key == TOKEN_KEY))),
            "expected a KeyValue Delete effect for the token"
        );
    }
}
