//! The shared Crux core — all business logic, shared across every shell.

pub mod app;
pub mod ffi;

pub use app::{Anjuman, Effect, Event, ViewModel};
pub use crux_core::Core;

// Re-export the capability crates so shells can reference their protocol types
// (e.g. `shared::crux_http::protocol::HttpRequest`) without adding the crates
// to every shell's dependency list.
pub use crux_http;
pub use crux_kv;
