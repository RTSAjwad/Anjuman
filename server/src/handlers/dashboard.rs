// Teacher dashboard.
//
// GET /dashboard — Aggregated overview across all the teacher's classes.

use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use sqlx::FromRow;

use anjuman_contracts::analytics::{AttentionStudent, ClassCard, DashboardResponse};
use anjuman_contracts::UserRole;

use crate::{auth::AuthUser, state::AppState};

// ---------------------------------------------------------------------------
// Row structs
// ---------------------------------------------------------------------------

#[derive(FromRow)]
struct ClassRow {
    class_id: i64,
    name: String,
}

#[derive(FromRow)]
struct RetentionRow {
    total_reviews: i64,
    good_reviews: i64,
}

#[derive(FromRow)]
struct InactiveStudentRow {
    id: i64,
    email: String,
    last_active: Option<chrono::DateTime<Utc>>,
}

#[derive(FromRow)]
struct LowRetentionRow {
    id: i64,
    email: String,
    last_active: Option<chrono::DateTime<Utc>>,
    total: i64,
    good: i64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn start_of_today() -> chrono::DateTime<Utc> {
    Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Build a positional `IN (...)` clause spanning `start..start+count`.
fn build_in_clause(start: usize, count: usize) -> String {
    (start..start + count)
        .map(|i| format!("${i}"))
        .collect::<Vec<_>>()
        .join(",")
}

async fn student_retention(db: &sqlx::PgPool, student_id: i64) -> Result<f64, StatusCode> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reviews WHERE student_id = $1")
        .bind(student_id)
        .fetch_one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if total == 0 {
        return Ok(0.0);
    }
    let good: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM reviews WHERE student_id = $1 AND rating >= 3")
            .bind(student_id)
            .fetch_one(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(good as f64 / total as f64)
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn dashboard(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<DashboardResponse>, (StatusCode, &'static str)> {
    if claims.role == UserRole::Student {
        return Err((
            StatusCode::FORBIDDEN,
            "Only teachers and admins can view the dashboard",
        ));
    }

    let today = start_of_today();
    let seven_days_ago = today - chrono::Duration::days(7);

    // --- Get all classes ---
    let classes: Vec<ClassRow> = if claims.role == UserRole::Admin {
        sqlx::query_as("SELECT id as class_id, name FROM classes WHERE school_id = $1 ORDER BY name")
            .bind(claims.school_id)
            .fetch_all(&state.db).await
    } else {
        sqlx::query_as("SELECT id as class_id, name FROM classes WHERE school_id = $1 AND created_by = $2 ORDER BY name")
            .bind(claims.school_id).bind(claims.sub)
            .fetch_all(&state.db).await
    }.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    let class_ids: Vec<i64> = classes.iter().map(|c| c.class_id).collect();

    // --- Quick stats ---
    let (total_students, reviews_today, average_retention) = if class_ids.is_empty() {
        (0, 0, 0.0)
    } else {
        let in_clause = build_in_clause(1, class_ids.len());

        let total: i64 = {
            let sql = format!(
                "SELECT COUNT(DISTINCT user_id) FROM class_members WHERE role = 'student' AND class_id IN ({})",
                in_clause
            );
            let mut q = sqlx::query_scalar(&sql);
            for id in &class_ids {
                q = q.bind(id);
            }
            q.fetch_one(&state.db)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        };

        let today_reviews: i64 = {
            let today_param = format!("${}", class_ids.len() + 1);
            let sql = format!(
                "SELECT COUNT(*) FROM reviews r JOIN class_members cm ON cm.user_id = r.student_id WHERE cm.class_id IN ({}) AND r.reviewed_at >= {}",
                in_clause, today_param
            );
            let mut q = sqlx::query_scalar(&sql);
            for id in &class_ids {
                q = q.bind(id);
            }
            q = q.bind(today);
            q.fetch_one(&state.db)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        };

        let retention = {
            let sql = format!(
                "SELECT COUNT(*) as total_reviews, SUM(CASE WHEN r.rating >= 3 THEN 1 ELSE 0 END) as good_reviews FROM reviews r JOIN class_members cm ON cm.user_id = r.student_id WHERE cm.class_id IN ({})",
                in_clause
            );
            let mut q = sqlx::query_as::<_, RetentionRow>(&sql);
            for id in &class_ids {
                q = q.bind(id);
            }
            let rows = q
                .fetch_all(&state.db)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;
            let (t, g) = rows.iter().fold((0, 0), |(t, g), r| {
                (t + r.total_reviews, g + r.good_reviews)
            });
            if t > 0 {
                round2(g as f64 / t as f64)
            } else {
                0.0
            }
        };

        (total, today_reviews, retention)
    };

    // --- Per-class cards ---
    let mut class_cards: Vec<ClassCard> = Vec::new();
    for class in &classes {
        let student_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM class_members WHERE class_id = $1 AND role = 'student'",
        )
        .bind(class.class_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        let retention_row = sqlx::query_as::<_, RetentionRow>(
            "SELECT COUNT(*) as total_reviews, SUM(CASE WHEN r.rating >= 3 THEN 1 ELSE 0 END) as good_reviews FROM reviews r JOIN class_members cm ON cm.user_id = r.student_id WHERE cm.class_id = $1"
        ).bind(class.class_id).fetch_one(&state.db).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        let avg_ret = if retention_row.total_reviews > 0 {
            round2(retention_row.good_reviews as f64 / retention_row.total_reviews as f64)
        } else {
            0.0
        };

        let last_active: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
            "SELECT MAX(r.reviewed_at) FROM reviews r JOIN class_members cm ON cm.user_id = r.student_id WHERE cm.class_id = $1"
        ).bind(class.class_id).fetch_one(&state.db).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        class_cards.push(ClassCard {
            class_id: class.class_id,
            name: class.name.clone(),
            student_count,
            avg_retention: avg_ret,
            last_activity: last_active,
        });
    }

    // --- Students needing attention ---
    let mut attention_needed: Vec<AttentionStudent> = Vec::new();

    for class in &classes {
        // Inactive (7 days).
        let inactive = sqlx::query_as::<_, InactiveStudentRow>(
            r#"
            SELECT u.id, u.email, MAX(r.reviewed_at) as last_active
            FROM class_members cm
            JOIN users u ON u.id = cm.user_id
            LEFT JOIN reviews r ON r.student_id = cm.user_id
            WHERE cm.class_id = $1 AND cm.role = 'student'
            GROUP BY u.id
            HAVING MAX(r.reviewed_at) IS NULL OR MAX(r.reviewed_at) < $2
            "#,
        )
        .bind(class.class_id)
        .bind(seven_days_ago)
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        for s in inactive {
            let ret = student_retention(&state.db, s.id).await.unwrap_or(0.0);
            attention_needed.push(AttentionStudent {
                student_id: s.id,
                email: s.email,
                reason: "inactive_7d".to_string(),
                last_active: s.last_active,
                retention: round2(ret),
                class_name: class.name.clone(),
            });
        }

        // Low retention (< 50%, >= 10 reviews).
        let low = sqlx::query_as::<_, LowRetentionRow>(
            r#"
            SELECT u.id, u.email, MAX(r.reviewed_at) as last_active,
                   COUNT(*) as total, SUM(CASE WHEN r.rating >= 3 THEN 1 ELSE 0 END) as good
            FROM class_members cm
            JOIN users u ON u.id = cm.user_id
            JOIN reviews r ON r.student_id = cm.user_id
            WHERE cm.class_id = $1 AND cm.role = 'student'
            GROUP BY u.id
            HAVING COUNT(*) >= 10 AND (SUM(CASE WHEN r.rating >= 3 THEN 1 ELSE 0 END)::float / COUNT(*)::float) < 0.5
            "#,
        )
        .bind(class.class_id)
        .fetch_all(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        for s in low {
            let ret = if s.total > 0 {
                s.good as f64 / s.total as f64
            } else {
                0.0
            };
            if !attention_needed.iter().any(|a| a.student_id == s.id) {
                attention_needed.push(AttentionStudent {
                    student_id: s.id,
                    email: s.email,
                    reason: "low_retention".to_string(),
                    last_active: s.last_active,
                    retention: round2(ret),
                    class_name: class.name.clone(),
                });
            }
        }
    }

    Ok(Json(DashboardResponse {
        total_students,
        active_classes: classes.len() as i64,
        reviews_today,
        average_retention,
        classes: class_cards,
        attention_needed,
    }))
}
