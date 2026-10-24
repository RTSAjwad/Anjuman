//! Owned Leptos design-system primitives, one module per component family.
//!
//! These own their **behavior**, **state**, and **structural** markup, styled
//! with **Tailwind utility classes + the semantic theme tokens** in `input.css`
//! (shadcn convention: `bg-primary`, `text-primary-foreground`, `border-input`,
//! …). See `docs/client/STYLING_MIGRATION.md`.
//!
//! Nothing here re-implements business logic: the shell is a Crux shell, so it
//! stays unidirectional — read the `ViewModel`, emit an `Event`, let the core
//! re-render. Controls are driven by Leptos signals.

mod badge;
mod breadcrumbs;
mod button;
mod card;
mod dialog;
mod input;
mod pagination;
mod select;
mod submit_button;
mod switch;
mod table;
mod tabs;

pub use badge::{Badge, BadgeColor};
pub use breadcrumbs::UiBreadcrumbs;
pub use button::{Button, ButtonAppearance};
pub use card::Card;
pub use dialog::UiDialog;
pub use input::UiInput;
pub use pagination::UiPagination;
pub use select::UiSelect;
pub use submit_button::UiSubmitButton;
pub use switch::Switch;
pub use table::{
    UiTable, UiTableBody, UiTableCell, UiTableHead, UiTableHeader, UiTableRow,
};
pub use tabs::{Tabs, TabsList, TabsTrigger};
