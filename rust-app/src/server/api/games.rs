use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::server::auth::{extract_token_from_header, validate_token};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/games")
            .route("", web::get().to(list_games))
            .route("/search", web::get().to(search_games))
            .route("/{id}", web::get().to(get_game))
            .route("/{id}/achievements", web::get().to(get_game_achievements)),
    );
}

#[derive(serde::Deserialize)]
struct PaginationParams {
    page: Option<i64>,
    per_page: Option<i64>,
}

#[derive(serde::Deserialize)]
struct SearchParams {
    q: String,
}

async fn list_games(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    query: web::Query<PaginationParams>,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let games = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>, i64, i64)>(
        r#"
        SELECT
            g.id, g.title, g.cover_image_url,
            COALESCE(COUNT(DISTINCT a.id), 0) as total_achievements,
            COALESCE(COUNT(DISTINCT ua.id) FILTER (WHERE ua.is_unlocked = true), 0) as unlocked_achievements
        FROM games g
        JOIN game_platform_ids gpi ON g.id = gpi.game_id
        JOIN achievements a ON gpi.id = a.game_platform_id
        LEFT JOIN user_achievements ua ON a.id = ua.achievement_id AND ua.user_id = $1
        GROUP BY g.id, g.title, g.cover_image_url
        ORDER BY g.title
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await;

    match games {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(id, title, cover_image_url, total, unlocked)| {
                    serde_json::json!({
                        "id": id,
                        "title": title,
                        "cover_image_url": cover_image_url,
                        "total_achievements": total,
                        "unlocked_achievements": unlocked,
                    })
                })
                .collect();
            HttpResponse::Ok().json(result)
        }
        Err(e) => {
            tracing::error!("Erreur liste jeux: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}

async fn search_games(
    pool: web::Data<PgPool>,
    query: web::Query<SearchParams>,
) -> HttpResponse {
    let search_term = query.q.to_lowercase();

    let games = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>, f32)>(
        r#"
        SELECT id, title, cover_image_url,
               similarity(normalized_title, $1) as sim
        FROM games
        WHERE normalized_title % $1
        ORDER BY sim DESC
        LIMIT 20
        "#,
    )
    .bind(&search_term)
    .fetch_all(pool.get_ref())
    .await;

    match games {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(id, title, cover_image_url, _sim)| {
                    serde_json::json!({
                        "id": id,
                        "title": title,
                        "cover_image_url": cover_image_url,
                    })
                })
                .collect();
            HttpResponse::Ok().json(result)
        }
        Err(e) => {
            tracing::error!("Erreur recherche jeux: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}

async fn get_game(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    path: web::Path<uuid::Uuid>,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let game_id = path.into_inner();

    let game = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>)>(
        "SELECT id, title, cover_image_url FROM games WHERE id = $1",
    )
    .bind(game_id)
    .fetch_optional(pool.get_ref())
    .await;

    match game {
        Ok(Some((id, title, cover_image_url))) => {
            // Recuperer les plateformes liees
            let platforms = sqlx::query_as::<_, (String, String, Option<String>, i32)>(
                "SELECT platform::text, platform_game_id, platform_name, total_achievements FROM game_platform_ids WHERE game_id = $1",
            )
            .bind(id)
            .fetch_all(pool.get_ref())
            .await
            .unwrap_or_default();

            // Compter les achievements debloques
            let stats = sqlx::query_as::<_, (i64, i64)>(
                r#"
                SELECT
                    COUNT(DISTINCT a.id) as total,
                    COUNT(DISTINCT ua.id) FILTER (WHERE ua.is_unlocked = true) as unlocked
                FROM game_platform_ids gpi
                JOIN achievements a ON gpi.id = a.game_platform_id
                LEFT JOIN user_achievements ua ON a.id = ua.achievement_id AND ua.user_id = $2
                WHERE gpi.game_id = $1
                "#,
            )
            .bind(id)
            .bind(user_id)
            .fetch_one(pool.get_ref())
            .await
            .unwrap_or((0, 0));

            HttpResponse::Ok().json(serde_json::json!({
                "id": id,
                "title": title,
                "cover_image_url": cover_image_url,
                "platforms": platforms.into_iter().map(|(p, pid, pname, total)| {
                    serde_json::json!({
                        "platform": p,
                        "platform_game_id": pid,
                        "platform_name": pname,
                        "total_achievements": total,
                    })
                }).collect::<Vec<_>>(),
                "total_achievements": stats.0,
                "unlocked_achievements": stats.1,
            }))
        }
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({"error": "Jeu non trouve"})),
        Err(e) => {
            tracing::error!("Erreur get game: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}

async fn get_game_achievements(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    path: web::Path<uuid::Uuid>,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let game_id = path.into_inner();

    let achievements = sqlx::query_as::<_, (uuid::Uuid, String, String, Option<String>, Option<String>, bool, Option<f32>, String, bool, Option<chrono::DateTime<chrono::Utc>>)>(
        r#"
        SELECT
            a.id, a.name, a.platform_achievement_id, a.description, a.icon_url,
            a.is_hidden, a.global_unlock_pct,
            gpi.platform::text,
            COALESCE(ua.is_unlocked, false) as is_unlocked,
            ua.unlocked_at
        FROM achievements a
        JOIN game_platform_ids gpi ON a.game_platform_id = gpi.id
        LEFT JOIN user_achievements ua ON a.id = ua.achievement_id AND ua.user_id = $2
        WHERE gpi.game_id = $1
        ORDER BY a.name
        "#,
    )
    .bind(game_id)
    .bind(user_id)
    .fetch_all(pool.get_ref())
    .await;

    match achievements {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(id, name, _api_name, description, icon_url, is_hidden, global_pct, platform, is_unlocked, unlocked_at)| {
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "description": if is_hidden && !is_unlocked { Some("Achievement cache".to_string()) } else { description },
                        "icon_url": icon_url,
                        "is_hidden": is_hidden,
                        "global_unlock_pct": global_pct,
                        "platform": platform,
                        "is_unlocked": is_unlocked,
                        "unlocked_at": unlocked_at,
                    })
                })
                .collect();
            HttpResponse::Ok().json(result)
        }
        Err(e) => {
            tracing::error!("Erreur achievements jeu: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
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
