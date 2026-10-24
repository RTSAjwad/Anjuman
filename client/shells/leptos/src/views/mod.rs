//! App-level view/screen components, one module per screen.
//!
//! These were mechanically extracted from `app.rs` (the composition root stays
//! there). Each module owns the view functions for one screen plus the private
//! helpers it needs; items that must cross module boundaries (used by
//! `RootComponent` or another view) are re-exported below.

pub mod browser;
pub mod deck;
pub mod deck_options;
pub mod study;

// Re-export the items `RootComponent` (and other views) reference. Leptos view
// names are derived by capitalizing the lower-case `fn` here, so `DeckList`,
// `Browser`, `StudyScreen`, `DeckOptionsPanel`, and `StudyControls` all resolve
// in `app.rs`'s `view!`.
pub(crate) use deck::{deck_ancestor_titles, DeckList};
pub(crate) use browser::Browser;
pub(crate) use study::{study_count_badges, StudyControls, StudyScreen};
pub(crate) use deck_options::DeckOptionsPanel;
