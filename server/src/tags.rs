// Tag storage helpers (note tags).
//
// A tag is a note-level label; the `::` hierarchy is carried in the name.
// Names are normalised (trimmed + lowercased) on write, matching Anki.

use sqlx::PgPool;

use anjuman_contracts::tags::Tag;

/// Normalise a raw tag name the way Anki does: trim surrounding whitespace and
/// lower-case. `::` hierarchy is meaningful but unvalidated (freedom to name).
pub fn normalize_name(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Return the tags attached to a note, ordered by name.
pub async fn tags_for_note(db: &PgPool, note_id: i64) -> Result<Vec<Tag>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT t.id, t.name FROM tags t JOIN note_tags nt ON nt.tag_id = t.id WHERE nt.note_id = $1 ORDER BY t.name",
        note_id
    )
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(|r| Tag { id: r.id, name: r.name }).collect())
}

/// The distinct tag namespace (every tag that exists), ordered by name. This is
/// the sidebar tree source.
pub async fn all_tags(db: &PgPool) -> Result<Vec<Tag>, sqlx::Error> {
    let rows = sqlx::query!("SELECT id, name FROM tags ORDER BY name")
        .fetch_all(db)
        .await?;

    Ok(rows.into_iter().map(|r| Tag { id: r.id, name: r.name }).collect())
}

/// Look up a tag id by its (normalised) name, inserting it if absent. Returns
/// the id. Use inside the caller's transaction for atomicity with `note_tags`.
pub async fn ensure_tag(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    name: &str,
) -> Result<i64, sqlx::Error> {
    let normalized = normalize_name(name);
    if normalized.is_empty() {
        // An empty name is never a valid tag; the caller should reject it, but
        // guard here too.
        return Ok(0i64);
    }

    let existing = sqlx::query_scalar!("SELECT id FROM tags WHERE name = $1", normalized)
        .fetch_optional(&mut **tx)
        .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let inserted = sqlx::query!("INSERT INTO tags (name) VALUES ($1) RETURNING id", normalized)
        .fetch_one(&mut **tx)
        .await?;

    Ok(inserted.id)
}

/// Delete tags that are no longer attached to any note (the "clear unused tags"
/// operation). Returns the number removed.
pub async fn clear_unused_tags(db: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)"
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}
