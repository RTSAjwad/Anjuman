//! Bin target: mounts the Leptos app (client-side rendering).
//!
//! CSR-only, so the modules live in the binary crate directly (no separate
//! `lib` target — avoids a Cargo artifact name collision with the binary).

mod app;
mod core_link;
mod http;
mod kv;

fn main() {
    // Better panic messages in the browser console.
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(|| leptos::prelude::view! { <app::RootComponent /> });
}
