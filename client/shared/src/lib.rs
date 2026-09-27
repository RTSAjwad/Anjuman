//! The shared Crux core — all business logic, shared across every shell.

pub mod app;
pub mod ffi;

pub use app::{Anjuman, Effect, Event, ViewModel};
pub use crux_core::Core;
