# Carve-out (shell agent → core agent): surface note fields on `CardBrowserView` (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending

## Context

The browser is being refactored to a single **note-details panel** shared by the
Cards and Notes views (selecting a card selects its note; selecting a note
selects it directly). The panel shows the note's fields as editable inputs and
saves via the existing `Event::NoteSave`.

The **notes** path already surfaces everything the panel needs (`NoteView.fields:
HashMap<String, String>`, plus `note_type_id`/`note_type_name`, and the ordered
field names via `vm.note_types[i].field_names`). The **cards** path does not: the
shell-facing `CardBrowserView` maps only `front`/`back` and drops the note's
field map — even though the wire type already carries it.

## The gap

`contracts::cards::CardBrowserResponse` already has:

```rust
pub fields: Map<String, Value>,
pub note_type_id: i64,
```

but `client/shared/src/app.rs`'s `CardBrowserView` (the Facet shell mirror) drops
both:

```rust
pub struct CardBrowserView {
    pub card_id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub deck_title: String,
    pub note_type_name: String,
    pub front: String,
    pub back: String,
    pub state: String,
    pub flag: i64,
    pub suspended: bool,
    pub buried: bool,
    // ❌ no `fields`, no `note_type_id`
}
```

So a selected card cannot populate the note panel without a round-trip that
doesn't exist.

## What to do (all in `client/shared/src/app.rs`)

1. **Add two fields to `CardBrowserView`:**
   - `pub note_type_id: i64`
   - `pub fields: HashMap<String, String>`

2. **Map them in `From<&CardBrowserResponse> for CardBrowserView`**, reusing the
   exact same `serde_json::Value → String` coercion as `NoteView::from`
   (strings pass through, other scalars `.to_string()`):

   ```rust
   note_type_id: c.note_type_id,
   fields: c.fields.iter().map(|(k, v)| {
       let s = match v {
           serde_json::Value::String(s) => s.clone(),
           other => other.to_string(),
       };
       (k.clone(), s)
   }).collect(),
   ```

   (Optional tidy-up: the coercion is duplicated verbatim in `NoteView::from`;
   you may extract a small `fn value_to_string(&Value) -> String` helper and call
   it from both, mirroring the server's `handlers/notes.rs::value_to_string`. If
   you'd rather not, keep it inline — shell doesn't mind either way.)

## Not needed (no change)

- **No `contracts/` edit** — `CardBrowserResponse.fields`/`note_type_id` already
  exist on the wire.
- **No `server/` edit** — `card_browser.rs` already populates `fields`.
- No new event — the panel reuses `Event::NoteSave`/`BrowserSelectCard`/
  `NoteBrowserSelectNote`.

## Validation (DoD)

- `cargo build --manifest-path client/Cargo.toml` + `cargo test -p shared` green.
- Update/extend the existing `browser_card` test fixture + the
  `CardBrowserView::from` mapping test so a `CardBrowserView` carries its note's
  `fields` and `note_type_id` through (analogous to `note_view_carries_sort_field`).
- No new warnings.

## Not my scope

The shell-side panel (`client/shells/react`) is mine; I'll consume this once it's
landed. Please confirm when `CardBrowserView` carries `fields` + `note_type_id`.
