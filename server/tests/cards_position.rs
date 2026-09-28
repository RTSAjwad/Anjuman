//! Tests for US-2.8a — `cards.position`.

mod common;

use common::{UserRole, create_user, seed_studiable_card};

#[tokio::test]
async fn new_cards_get_increasing_positions() {
    let _guard = common::db_guard().await;
    let app = common::TestApp::new().await;
    let (student_id, _token) = create_user(&app, UserRole::Student).await;

    // Two cards seeded sequentially; their positions must be monotonic.
    let card_a = seed_studiable_card(&app, student_id, "new", 0, 0, 0).await;
    let card_b = seed_studiable_card(&app, student_id, "new", 0, 0, 0).await;

    let pos_a: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", card_a)
        .fetch_one(&app.db)
        .await
        .expect("pos a");
    let pos_b: i64 = sqlx::query_scalar!("SELECT position FROM cards WHERE id = $1", card_b)
        .fetch_one(&app.db)
        .await
        .expect("pos b");

    assert!(
        pos_a < pos_b,
        "later-inserted card must have a higher position ({pos_a} !< {pos_b})"
    );
}
