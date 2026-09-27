//! Analytics and dashboard DTOs.

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Per-student aggregate statistics.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StudentStats {
    pub student_id: i64,
    pub total_reviews: i64,
    pub reviews_today: i64,
    pub retention_rate: f64,
    pub average_rating: f64,
    pub cards_total: i64,
    pub cards_mastered: i64,
    pub cards_learning: i64,
    pub cards_struggling: i64,
    pub study_streak_days: i64,
    pub time_spent_today_seconds: i64,
    pub sessions_this_week: i64,
}

/// A subset of student stats used for class-level rollups.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StudentStatsWithEmail {
    pub student_id: i64,
    pub email: String,
    pub total_reviews: i64,
    pub retention_rate: f64,
    pub cards_mastered: i64,
    pub last_active: Option<DateTime<Utc>>,
}

/// A single day's activity point.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DailyPoint {
    pub date: String,
    pub reviews: i64,
    pub avg_rating: f64,
    pub avg_time_ms: f64,
}

/// A card flagged as difficult.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DifficultCard {
    pub card_id: i64,
    pub front: String,
    pub avg_rating: f64,
    pub total_reviews: i64,
}

/// Aggregate analytics for a class.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClassAnalytics {
    pub class_name: String,
    pub students_enrolled: i64,
    pub average_retention: f64,
    pub most_difficult_cards: Vec<DifficultCard>,
    pub students: Vec<StudentStatsWithEmail>,
}

/// Detailed per-student analytics (with daily history).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StudentDetail {
    pub student_id: i64,
    pub email: String,
    pub total_reviews: i64,
    pub reviews_today: i64,
    pub retention_rate: f64,
    pub average_rating: f64,
    pub cards_total: i64,
    pub cards_mastered: i64,
    pub cards_learning: i64,
    pub cards_struggling: i64,
    pub study_streak_days: i64,
    pub time_spent_today_seconds: i64,
    pub sessions_this_week: i64,
    pub daily: Vec<DailyPoint>,
}

/// The teacher dashboard view.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DashboardResponse {
    pub total_students: i64,
    pub active_classes: i64,
    pub reviews_today: i64,
    pub average_retention: f64,
    pub classes: Vec<ClassCard>,
    pub attention_needed: Vec<AttentionStudent>,
}

/// A class as summarized on the dashboard.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClassCard {
    pub class_id: i64,
    pub name: String,
    pub student_count: i64,
    pub avg_retention: f64,
    pub last_activity: Option<DateTime<Utc>>,
}

/// A student flagged for the teacher's attention.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AttentionStudent {
    pub student_id: i64,
    pub email: String,
    pub reason: String,
    pub last_active: Option<DateTime<Utc>>,
    pub retention: f64,
    pub class_name: String,
}
