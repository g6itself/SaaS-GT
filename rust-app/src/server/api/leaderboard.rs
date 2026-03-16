use actix_web::{web, HttpResponse};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize, sqlx::FromRow)]
struct LeaderboardEntry {
    rank: i64,
    username: String,
    display_name: Option<String>,
    total_points: i64,
    league: String,
    profile_image_url: Option<String>,
    total_achievements: i64,
    completion_avg: f64,
    rank_snapshot: Option<i64>,
    rank_snapshot_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/leaderboard").route("", web::get().to(get_leaderboard)),
    );
}

async fn get_leaderboard(pool: web::Data<PgPool>) -> HttpResponse {
    let result = sqlx::query_as::<_, LeaderboardEntry>(
        r#"
        SELECT
            ROW_NUMBER() OVER (ORDER BY u.total_points DESC)::BIGINT AS rank,
            u.username,
            u.display_name,
            u.total_points,
            u.league::TEXT AS league,
            u.profile_image_url,
            COALESCE(lc.total_achievements, 0)::BIGINT AS total_achievements,
            COALESCE(lc.completion_avg, 0.00)::FLOAT8 AS completion_avg,
            u.rank_snapshot,
            u.rank_snapshot_at
        FROM users u
        LEFT JOIN leaderboard_cache lc ON lc.user_id = u.id
        WHERE u.is_active = true
        ORDER BY u.total_points DESC
        LIMIT 50
        "#,
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(entries) => HttpResponse::Ok().json(entries),
        Err(e) => {
            tracing::error!("Erreur leaderboard: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur interne du serveur"
            }))
        }
    }
}
