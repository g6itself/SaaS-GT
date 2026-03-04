use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::server::auth::{extract_token_from_header, validate_token};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/achievements")
            .route("/stats", web::get().to(get_stats))
            .route("/recent", web::get().to(get_recent)),
    );
}

fn get_user_id(req: &HttpRequest) -> Result<uuid::Uuid, HttpResponse> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "Token manquant"}))
        })?;

    let token = extract_token_from_header(auth_header).ok_or_else(|| {
        HttpResponse::Unauthorized().json(serde_json::json!({"error": "Format de token invalide"}))
    })?;

    let claims = validate_token(token).map_err(|_| {
        HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "Token expire ou invalide"}))
    })?;

    Ok(claims.sub)
}

async fn get_stats(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Stats globales
    let global_stats = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            COUNT(DISTINCT ua.achievement_id) FILTER (WHERE ua.is_unlocked = true) as unlocked,
            COUNT(DISTINCT a.id) as total
        FROM achievements a
        JOIN game_platform_ids gpi ON a.game_platform_id = gpi.id
        JOIN platform_connections pc ON gpi.platform = pc.platform AND pc.user_id = $1
        LEFT JOIN user_achievements ua ON a.id = ua.achievement_id AND ua.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool.get_ref())
    .await
    .unwrap_or((0, 0));

    // Stats par plateforme
    let platform_stats = sqlx::query_as::<_, (String, i64, i64, i64)>(
        r#"
        SELECT
            gpi.platform::text,
            COUNT(DISTINCT a.id) as total_achievements,
            COUNT(DISTINCT ua.achievement_id) FILTER (WHERE ua.is_unlocked = true) as unlocked_achievements,
            COUNT(DISTINCT gpi.game_id) as total_games
        FROM game_platform_ids gpi
        JOIN achievements a ON gpi.id = a.game_platform_id
        JOIN platform_connections pc ON gpi.platform = pc.platform AND pc.user_id = $1
        LEFT JOIN user_achievements ua ON a.id = ua.achievement_id AND ua.user_id = $1
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

async fn get_recent(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let recent = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>, Option<String>, String, String, Option<chrono::DateTime<chrono::Utc>>)>(
        r#"
        SELECT
            a.id, a.name, a.description, a.icon_url,
            g.title as game_title,
            gpi.platform::text,
            ua.unlocked_at
        FROM user_achievements ua
        JOIN achievements a ON ua.achievement_id = a.id
        JOIN game_platform_ids gpi ON a.game_platform_id = gpi.id
        JOIN games g ON gpi.game_id = g.id
        WHERE ua.user_id = $1 AND ua.is_unlocked = true
        ORDER BY ua.unlocked_at DESC NULLS LAST
        LIMIT 10
        "#,
    )
    .bind(user_id)
    .fetch_all(pool.get_ref())
    .await;

    match recent {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(id, name, description, icon_url, game_title, platform, unlocked_at)| {
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "description": description,
                        "icon_url": icon_url,
                        "game_title": game_title,
                        "platform": platform,
                        "unlocked_at": unlocked_at,
                    })
                })
                .collect();
            HttpResponse::Ok().json(result)
        }
        Err(e) => {
            tracing::error!("Erreur recent achievements: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}
