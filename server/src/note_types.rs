// Note type definitions and card rendering.
//
// Note types are stored in the database (not hardcoded). Each note type
// has a name, a set of field names, and one or more card templates.
//
// Templates define front and back patterns using `{{FieldName}}` placeholders.
// Cards are rendered at display time from the template patterns + note fields.

use serde::Serialize;
use sqlx::PgPool;

pub use anjuman_contracts::note_types::Template;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A note type definition as stored in the database.
#[derive(Debug, Clone, Serialize)]
pub struct NoteType {
    pub id: i64,
    pub name: String,
    pub field_names: Vec<String>,
    pub sort_field: String,
    /// The shared styling CSS block (per note type).
    pub styling: String,
    pub templates: Vec<Template>,
}

impl NoteType {
    /// The effective sort field: uses the configured sort_field if set,
    /// otherwise falls back to the first field name (Anki default).
    pub fn effective_sort_field(&self) -> &str {
        if self.sort_field.is_empty() {
            self.field_names.first().map(|s| s.as_str()).unwrap_or("")
        } else {
            &self.sort_field
        }
    }
}


/// The fields of a note, as deserialised from JSON.
pub type NoteFields = serde_json::Map<String, serde_json::Value>;

/// The rendered output for a single card.
#[derive(Debug, Clone, Serialize)]
pub struct RenderedCard {
    /// The stable id of the template that generated this card.
    pub template_id: i64,
    /// The rendered front (question) text.
    pub front: String,
    /// The rendered back (answer) text.
    pub back: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Look up a note type by ID, including its templates.
pub async fn get_note_type(db: &PgPool, id: i64) -> Result<NoteType, String> {
    let row = sqlx::query!(
        "SELECT id, name, field_names, sort_field, styling FROM note_types WHERE id = $1",
        id
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Database error: {e}"))?
    .ok_or_else(|| format!("Note type {id} not found"))?;

    let field_names: Vec<String> = serde_json::from_value(row.field_names)
        .map_err(|e| format!("Invalid field_names JSON: {e}"))?;

    let templates = sqlx::query!(
        "SELECT id, ord, name, front_pattern, back_pattern FROM note_type_templates WHERE note_type_id = $1 ORDER BY ord",
        id
    )
    .fetch_all(db)
    .await
    .map_err(|e| format!("Database error: {e}"))?;

    let templates: Vec<Template> = templates
        .into_iter()
        .map(|t| Template {
            id: t.id,
            ord: t.ord,
            name: t.name,
            front_pattern: t.front_pattern,
            back_pattern: t.back_pattern,
        })
        .collect();

    Ok(NoteType {
        id: row.id,
        name: row.name,
        field_names,
        sort_field: row.sort_field,
        styling: row.styling,
        templates,
    })
}

/// Look up a note type by name (for legacy compatibility).
pub async fn get_note_type_by_name(db: &PgPool, name: &str) -> Result<NoteType, String> {
    let row = sqlx::query!("SELECT id FROM note_types WHERE name = $1", name)
        .fetch_optional(db)
        .await
        .map_err(|e| format!("Database error: {e}"))?
        .ok_or_else(|| format!("Note type '{name}' not found"))?;

    get_note_type(db, row.id).await
}

/// List all note types in a school.
pub async fn list_note_types(db: &PgPool, school_id: i64) -> Result<Vec<NoteType>, String> {
    let rows = sqlx::query!(
        "SELECT id, name, field_names FROM note_types WHERE school_id = $1 ORDER BY name",
        school_id
    )
    .fetch_all(db)
    .await
    .map_err(|e| format!("Database error: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        let nt = get_note_type(db, row.id).await?;
        result.push(nt);
    }
    Ok(result)
}

/// Validate that the given fields match the note type's requirements.
pub fn validate_fields(field_names: &[String], fields: &NoteFields) -> Result<(), String> {
    let missing: Vec<&String> = field_names
        .iter()
        .filter(|name| !fields.contains_key(name.as_str()))
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        let names: Vec<String> = missing.iter().map(|n| n.to_string()).collect();
        Err(format!("Missing required fields: {}", names.join(", ")))
    }
}

/// Render all cards for a note by applying each template to the fields.
///
/// Each rendered side is wrapped in a `.card` element (Anki's per-card wrapper,
/// injected at render time — not part of the template text), with the card-type
/// ordinal driving the class suffix: `.card` for the first template, `.card1`,
/// `.card2`, … for later ones. See US-4.6.1.
pub fn render_cards(templates: &[Template], fields: &NoteFields) -> Vec<RenderedCard> {
    templates
        .iter()
        .map(|template| render_one_card(template, fields))
        .collect()
}

/// Render a single card given a note type and note fields.
pub fn render_card(
    templates: &[Template],
    template_id: i64,
    fields: &NoteFields,
) -> Option<RenderedCard> {
    templates
        .iter()
        .find(|t| t.id == template_id)
        .map(|template| render_one_card(template, fields))
}

/// Render one template into a `RenderedCard`, wrapping each side in the `.card`
/// element keyed by the template's ordinal.
fn render_one_card(template: &Template, fields: &NoteFields) -> RenderedCard {
    let front = wrap_card(&render_template(&template.front_pattern, fields), template.ord);
    let back = wrap_card(&render_template(&template.back_pattern, fields), template.ord);
    RenderedCard {
        template_id: template.id,
        front,
        back,
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// The `.card` class name for a template at the given 0-based ordinal, matching
/// Anki: the first card type is `.card`, and every later one is `.cardN` (N =
/// ordinal).
fn card_wrapper_class(ord: i64) -> String {
    if ord == 0 {
        "card".to_string()
    } else {
        format!("card{ord}")
    }
}

/// Wrap a rendered card side (front/back) in Anki's per-card `.card` wrapper.
fn wrap_card(body: &str, ord: i64) -> String {
    // `body` is the renderer's *own* output (template-expanded field text), so
    // building this as a string is safe; we never parse/rewrite the author's
    // HTML — only surround it.
    format!(
        "<div class=\"{}\">{body}</div>",
        card_wrapper_class(ord)
    )
}

/// Replace `{{FieldName}}` placeholders in a template string.
fn render_template(template: &str, fields: &NoteFields) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next();
            let mut placeholder = String::new();
            loop {
                match chars.next() {
                    Some('}') if chars.peek() == Some(&'}') => {
                        chars.next();
                        break;
                    }
                    Some(c) => placeholder.push(c),
                    None => {
                        result.push_str("{{");
                        result.push_str(&placeholder);
                        break;
                    }
                }
            }
            let value = fields
                .get(placeholder.trim())
                .and_then(|v| v.as_str())
                .unwrap_or(&placeholder);
            result.push_str(value);
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> NoteFields {
        [
            ("Front".to_string(), serde_json::Value::String("hola".to_string())),
            ("Back".to_string(), serde_json::Value::String("hello".to_string())),
        ]
        .into_iter()
        .collect()
    }

    fn template(id: i64, ord: i64) -> Template {
        Template {
            id,
            ord,
            name: format!("Card {ord}"),
            front_pattern: "{{Front}}".to_string(),
            back_pattern: "{{Front}}<hr>{{Back}}".to_string(),
        }
    }

    /// `render_card` wraps the rendered front/back in `.card` so the note type's
    /// styling applies (US-4.6.1).
    #[test]
    fn render_card_wraps_sides_in_card_element() {
        let templates = vec![template(10, 0)];
        let rendered = render_card(&templates, 10, &fields()).expect("card rendered");

        assert_eq!(rendered.front, "<div class=\"card\">hola</div>");
        assert_eq!(rendered.back, "<div class=\"card\">hola<hr>hello</div>");
    }

    /// The wrapper class reflects the template's ordinal: `.card` for the first
    /// template, `.card1` (etc.) for later ones, matching Anki's multi-card types.
    #[test]
    fn render_cards_suffixes_wrapper_class_by_ordinal() {
        let templates = vec![template(10, 0), template(11, 1)];
        let rendered = render_cards(&templates, &fields());

        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0].template_id, 10);
        assert!(rendered[0].front.starts_with("<div class=\"card\">"));
        assert_eq!(rendered[1].template_id, 11);
        assert!(rendered[1].front.starts_with("<div class=\"card1\">"));
        assert!(rendered[1].back.starts_with("<div class=\"card1\">"));
    }

    /// The renderer keeps author HTML inside the template intact, only wrapping
    /// around it (US-4.6.1 — no rewrite/strip).
    #[test]
    fn render_card_preserves_author_html_in_template() {
        let mut t = template(10, 0);
        t.front_pattern = "<div class='mine'>{{Front}}</div>".to_string();
        let rendered = render_card(&[t], 10, &fields()).expect("card rendered");

        assert_eq!(
            rendered.front,
            "<div class=\"card\"><div class='mine'>hola</div></div>"
        );
    }
}
