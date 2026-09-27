//! FFI surface for native shells.
//!
//! Wraps `Anjuman` in a `Bridge` so events, effect requests and view models
//! cross the FFI boundary as serialized bytes. This is the same interface
//! SwiftUI, WinUI, Jetpack Compose and Libadwaita will use.
//!
//! The `#[boltffi::export]` attribute makes this class callable from the
//! generated Swift / Kotlin / C# / TypeScript bindings (see `boltffi.toml`).
//! BoltFFI maps `&[u8]` and `Vec<u8>` to native byte-buffer types
//! (`Data`, `ByteArray`, `byte[]`, `Uint8Array`) respectively.

use crux_core::{Core, bridge::{Bridge, EffectId}};

use crate::app::Anjuman;

/// The main interface used by the shell.
pub struct CoreFfi {
    core: Bridge<Anjuman>,
}

impl Default for CoreFfi {
    fn default() -> Self {
        Self::new()
    }
}

#[boltffi::export]
impl CoreFfi {
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Bridge::new(Core::new()),
        }
    }

    /// Send an event to the app and return the effects.
    ///
    /// # Panics
    /// If the event cannot be deserialized. In production you should handle
    /// the error properly.
    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.update(data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Resolve an effect and return the effects.
    ///
    /// # Panics
    /// If the `data` cannot be deserialized into an effect or the `id` is
    /// invalid. In production you should handle the error properly.
    #[must_use]
    pub fn resolve(&self, id: u32, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.resolve(EffectId(id), data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Get the current `ViewModel`.
    ///
    /// # Panics
    /// If the view cannot be serialized. In production you should handle the
    /// error properly.
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let mut view_model = Vec::new();
        match self.core.view(&mut view_model) {
            Ok(()) => view_model,
            Err(e) => panic!("{e}"),
        }
    }
}
