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

/// Count tag rows that no note references (the orphans "Clear Unused Tags" would
/// remove).
pub async fn count_unused_tags(db: &PgPool) -> Result<i64, sqlx::Error> {
    let count: Option<i64> = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM note_tags)"
    )
    .fetch_one(db)
    .await?;
    Ok(count.unwrap_or(0))
}

/// Rename a tag and its whole `::`-subtree, merging onto an existing target when
/// one is already present (Anki parity — US-6.7).
///
/// `old_name` → `new_name` rewrites every tag whose name is `old_name` or
/// `old_name::*`, replacing the `old_name` prefix with `new_name`. If a target
/// name already exists, the source's `note_tags` links are *merged* (deduped by
/// `(note_id, tag_id)`) into it and the vacated source row is deleted — rather
/// than erroring on the `UNIQUE(name)` constraint.
///
/// Returns the surviving tag for the `new_name` position (the merged target if
/// one existed, otherwise the renamed source), surfaced to the caller.
pub async fn rename_tag_subtree(
    db: &PgPool,
    old_name: &str,
    new_name: &str,
) -> Result<Tag, sqlx::Error> {
    let old = normalize_name(old_name);
    let new = normalize_name(new_name);

    let mut tx = db.begin().await?;

    // The exact tag being renamed must exist.
    let source = sqlx::query!("SELECT id, name FROM tags WHERE name = $1", old)
        .fetch_optional(&mut *tx)
        .await?;
    let Some(source) = source else {
        return Err(sqlx::Error::RowNotFound);
    };

    // No-op rename: nothing to rewrite.
    if old == new {
        tx.commit().await?;
        return Ok(Tag { id: source.id, name: new });
    }

    // The subtree: the source name plus every `old::…` descendant. `LIKE`
    // placeholders are escaped so a literal `%`/`_` in a tag name can't match
    // unexpectedly.
    let old_prefix = format!("{}::", escape_like(&old));
    let affected = sqlx::query!(
        "SELECT id, name FROM tags WHERE name = $1 OR name LIKE $2 ESCAPE '\\' ORDER BY name",
        old,
        format!("{old_prefix}%")
    )
    .fetch_all(&mut *tx)
    .await?;

    for row in &affected {
        // The `name = old` case has `suffix` empty; every descendant keeps its
        // `old::` remainder.
        let suffix = row.name.strip_prefix(&old).unwrap();
        let target_name = format!("{new}{suffix}");

        // Upsert the target name (merge: reuse an existing row, else create).
        let target_id: i64 = sqlx::query_scalar!(
            "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id",
            target_name
        )
        .fetch_one(&mut *tx)
        .await?;

        // Move this source row's note links into the target (deduped).
        sqlx::query!(
            "INSERT INTO note_tags (note_id, tag_id)
             SELECT nt.note_id, $2 FROM note_tags nt WHERE nt.tag_id = $1
             ON CONFLICT (note_id, tag_id) DO NOTHING",
            row.id,
            target_id
        )
        .execute(&mut *tx)
        .await?;

        // Remove the now-vacated source row (its links are already moved/deduped;
        // ON DELETE CASCADE won't remove the freshly-inserted target links).
        sqlx::query!("DELETE FROM tags WHERE id = $1", row.id)
            .execute(&mut *tx)
            .await?;
    }

    // The surviving row (the merged target for `new`, if one existed).
    let result = sqlx::query!("SELECT id, name FROM tags WHERE name = $1", new)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Tag { id: result.id, name: result.name })
}

/// Escape `%`, `_`, and `\` in a string so it can be used as a literal inside a
/// `LIKE` pattern with `ESCAPE '\'`.
pub(crate) fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Escape a string for safe interpolation into a SQL string literal (doubling
/// single quotes). Tags are interpolated by the dynamic WHERE builders rather
/// than bound, so this is the analogue of a parameter for those paths.
fn sql_string_literal(s: &str) -> String {
    s.replace('\'', "''")
}

/// Build the `EXISTS` WHERE fragment for the card/note browser tag filter, using
/// Anki's `::-descendant` semantics (US-6.7): a selected tag `biology` matches
/// the tag itself **and** any `biology::…` descendant. The sidebar is a
/// multi-select OR — a note/card matches when its note has ANY selected tag (or
/// descendant thereof). Wild cards are **not** interpreted here (the search-box
/// `tag:` predicate owns glob semantics), so any `%`/`_`/`*` is treated literally.
///
/// `alias` is the SQL alias of the owning table (`n` for notes, `c` for cards,
/// both of which join `note_tags`/`tags` through their note id).
pub(crate) fn tag_where_sql(csv: &str) -> String {
    let tags: Vec<&str> = csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if tags.is_empty() {
        return String::new();
    }

    let predicates: Vec<String> = tags
        .iter()
        .map(|t| {
            let lit = sql_string_literal(t);
            let like = format!("{}::%", escape_like(t));
            let like_lit = sql_string_literal(&like);
            // Exact, or a `::-descendant of the exact name.
            format!(
                "(tg.name = '{lit}' OR tg.name LIKE '{like_lit}' ESCAPE '\\')"
            )
        })
        .collect();

    format!(
        "AND EXISTS (SELECT 1 FROM note_tags nt JOIN tags tg ON tg.id = nt.tag_id WHERE nt.note_id = n.id AND ({clauses}))",
        clauses = predicates.join(" OR ")
    )
}

// ---------------------------------------------------------------------------
// Tree (US-6.4) — the sidebar nesting, `::`-split, with implicit interior nodes.
// ---------------------------------------------------------------------------

use anjuman_contracts::tags::TagNode;
use std::collections::BTreeMap;

#[derive(Default)]
struct Node {
    children: BTreeMap<String, Node>,
}

impl Node {
    fn insert(&mut self, segments: &[&str]) {
        if let Some((&head, rest)) = segments.split_first() {
            self.children.entry(head.to_string()).or_default().insert(rest);
        }
    }

    fn into_tag_nodes(&self) -> Vec<TagNode> {
        let mut out: Vec<TagNode> = self
            .children
            .iter()
            .map(|(name, child)| TagNode {
                name: name.clone(),
                children: child.into_tag_nodes(),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
}

/// Build the tag tree from a list of full tag names. Interior nodes are
/// *implicit* — `bio::cells` renders `bio > cells` even with no bare `bio` tag.
/// Returns roots sorted by name.
pub fn build_tag_tree<'a, I>(names: I) -> Vec<TagNode>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut root = Node::default();
    for name in names {
        let segments: Vec<&str> = name.split("::").collect();
        root.insert(&segments);
    }
    root.into_tag_nodes()
}

/// Does a tag *name* match a `tag:` query, with Anki's semantics?
///
/// - `none` — matches only the "no tags" sentinel (callers pass that case
///   separately; a real tag name never matches `none`).
/// - `foo` (exact) — matches the tag named `foo` only.
/// - `foo::` prefix semantics are expressed by the *caller* via `tag:foo` →
///   `tag_matches(name, "foo") || tag_matches(name, "foo::*")`; see `tag_query_matches`.
/// - `foo*` / `*foo` / `fo*o` — glob (Anki's `*` wildcard).
///
/// This is the lowest-level predicate: exact name vs. single-wildcard glob.
pub fn tag_matches(name: &str, query: &str) -> bool {
    if query == "none" {
        return false; // `none` matches no concrete tag name
    }
    if !query.contains('*') {
        return name == query;
    }
    glob_match(query, name)
}

/// The full `tag:`-query predicate over a single tag name, matching Anki:
/// `tag:animal` matches `animal` and any subtag `animal::*`; `tag:ani*` globs.
/// `tag:none` is handled by the caller (it matches notes with *no* tags, not a
/// tag name).
pub fn tag_query_matches(name: &str, query: &str) -> bool {
    if query == "none" {
        return false;
    }
    // Exact name, or a `::`-descendant of an exact parent (`animal` → `animal::mammal`).
    if !query.contains('*') {
        return name == query || name.starts_with(&format!("{query}::"));
    }
    // Glob with `::` prefix semantics: `ani*` matches `ani`, `ani::x`, `animal`, …
    glob_match(query, name) || (name.contains("::") && name.split("::").any(|seg| glob_match(query, seg)))
}

/// Simple `*`-wildcard glob matcher (single embedded wildcard supported for the
/// Anki `tag:ani*`/`tag:*ani` cases; `*` in the middle also works).
fn glob_match(pattern: &str, text: &str) -> bool {
    match pattern.find('*') {
        None => pattern == text,
        Some(i) => {
            let (prefix, suffix) = pattern.split_at(i);
            let suffix = &suffix[1..]; // strip the '*'
            text.starts_with(prefix) && text.ends_with(suffix) && text.len() >= prefix.len() + suffix.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_matches_exact_and_glob() {
        assert!(tag_matches("animal", "animal"));
        assert!(!tag_matches("animal::mammal", "animal"), "exact does not match subtag");
        assert!(tag_matches("animal::mammal", "animal::*"));
        assert!(tag_matches("animal::mammal", "ani*"));
        assert!(!tag_matches("animal", "none"));
    }

    #[test]
    fn tag_query_matches_prefix_and_subtags() {
        // `tag:animal` matches `animal` and any `animal::*` subtag.
        assert!(tag_query_matches("animal", "animal"));
        assert!(tag_query_matches("animal::mammal", "animal"));
        assert!(tag_query_matches("animal::mammal::bovine", "animal"));
        assert!(!tag_query_matches("animalia", "animal"));
        assert!(!tag_query_matches("plant", "animal"));
        // Glob.
        assert!(tag_query_matches("animal", "ani*"));
        assert!(tag_query_matches("animal::mammal", "ani*"));
    }

    #[test]
    fn build_tag_tree_implicit_interior_nodes() {
        let tree = build_tag_tree(["bio::cells", "bio"].iter().map(|s| *s));
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "bio");
        // `bio` has a child `cells` (the `bio::cells` tag), plus is itself a leaf.
        let children: Vec<&str> = tree[0].children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(children, vec!["cells"]);
    }
}
