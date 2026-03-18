use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::server::auth_extractor::AuthUser;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/achievements")
            .route("/stats", web::get().to(get_stats)),
    );
}

async fn get_stats(
    pool: web::Data<PgPool>,
    auth: AuthUser,
) -> HttpResponse {
    let user_id = auth.user_id;

    // Stats globales depuis user_game_stats (totaux agrégés)
    let global_stats = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            COALESCE(SUM(achievements_unlocked), 0)::BIGINT AS unlocked,
            COALESCE(SUM(achievements_total), 0)::BIGINT AS total
        FROM user_game_stats
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool.get_ref())
    .await
    .unwrap_or((0, 0));

    // Stats par plateforme depuis user_game_stats + game_platform_ids
    let platform_stats = sqlx::query_as::<_, (String, i64, i64, i64)>(
        r#"
        SELECT
            gpi.platform::text,
            COALESCE(SUM(ugs.achievements_total), 0)::BIGINT AS total_achievements,
            COALESCE(SUM(ugs.achievements_unlocked), 0)::BIGINT AS unlocked_achievements,
            COUNT(DISTINCT ugs.game_id)::BIGINT AS total_games
        FROM user_game_stats ugs
        JOIN game_platform_ids gpi ON gpi.game_id = ugs.game_id
        JOIN platform_connections pc ON gpi.platform = pc.platform AND pc.user_id = ugs.user_id
        WHERE ugs.user_id = $1
        GROUP BY gpi.platform
        "#,
    )
    .bind(user_id)
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    HttpResponse::Ok().json(serde_json::json!({
        "total_achievements": global_stats.1,
        "unlocked_achievements": global_stats.0,
        "completion_percentage": if global_stats.1 > 0 {
            (global_stats.0 as f64 / global_stats.1 as f64 * 100.0).round()
        } else {
            0.0
        },
        "platforms": platform_stats.into_iter().map(|(platform, total, unlocked, games)| {
            serde_json::json!({
                "platform": platform,
                "total_achievements": total,
                "unlocked_achievements": unlocked,
                "total_games": games,
            })
        }).collect::<Vec<_>>(),
    }))
}
