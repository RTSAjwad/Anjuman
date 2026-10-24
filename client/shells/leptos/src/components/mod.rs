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
mod filter_menu;
mod flag;
mod floating_layer;
mod input;
mod inspector;
mod pagination;
mod select;
mod sidebar;
mod switch;
mod table;
mod tabs;

pub use badge::{Badge, BadgeVariant};
pub use breadcrumbs::Breadcrumb;
pub use button::{Button, ButtonVariant};
pub use card::Card;
pub use dialog::Dialog;
pub use filter_menu::{deck_filter_tree, filter_branch, filter_leaf};
pub use flag::FlagSwatch;
pub use floating_layer::FloatingLayer;
pub use input::Input;
pub use inspector::{Inspector, InspectorFields, InspectorRow, LabeledSwitch};
pub use pagination::Pagination;
pub use select::Select;
pub use sidebar::SidebarItem;
pub use table::{
    Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
};
pub use tabs::{Tabs, TabsList, TabsTrigger};
