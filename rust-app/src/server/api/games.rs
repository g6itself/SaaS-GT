use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::server::auth::{extract_token_from_header, validate_token};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/games")
            .route("", web::get().to(list_games))
            .route("/search", web::get().to(search_games))
            .route("/{id}", web::get().to(get_game)),
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

    let games = sqlx::query_as::<_, (uuid::Uuid, String, i32, i32)>(
        r#"
        SELECT
            g.id, g.title,
            COALESCE(ugs.achievements_total, 0)::INT AS total_achievements,
            COALESCE(ugs.achievements_unlocked, 0)::INT AS unlocked_achievements
        FROM games g
        LEFT JOIN user_game_stats ugs ON ugs.game_id = g.id AND ugs.user_id = $1
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
                .map(|(id, title, total, unlocked)| {
                    serde_json::json!({
                        "id": id,
                        "title": title,
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
    let search_term = query.q.trim().to_lowercase();

    // Requête vide → liste vide
    if search_term.is_empty() {
        return HttpResponse::Ok().json(serde_json::Value::Array(vec![]));
    }

    // Pour les requêtes courtes (< 3 chars), pg_trgm n'est pas fiable :
    // on utilise un ILIKE prefix en priorité, puis trigram pour les plus longues.
    let games = if search_term.len() < 3 {
        sqlx::query_as::<_, (uuid::Uuid, String)>(
            r#"
            SELECT id, title
            FROM games
            WHERE normalized_title ILIKE $1
            ORDER BY title
            LIMIT 20
            "#,
        )
        .bind(format!("{}%", search_term))
        .fetch_all(pool.get_ref())
        .await
        .map(|rows| rows.into_iter().map(|(id, t)| (id, t, 1.0f32)).collect::<Vec<_>>())
    } else {
        let like_pattern = format!("{}%", search_term);
        // Trigram similarity + boost si le titre commence par la requête
        sqlx::query_as::<_, (uuid::Uuid, String, f32)>(
            r#"
            SELECT id, title,
                   CASE WHEN normalized_title ILIKE $2
                        THEN 1.0 + similarity(normalized_title, $1)
                        ELSE similarity(normalized_title, $1)
                   END AS score
            FROM games
            WHERE normalized_title % $1 OR normalized_title ILIKE $2
            ORDER BY score DESC, title
            LIMIT 20
            "#,
        )
        .bind(&search_term)
        .bind(&like_pattern)
        .fetch_all(pool.get_ref())
        .await
    };

    match games {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(id, title, _score)| {
                    serde_json::json!({
                        "id": id,
                        "title": title,
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

    let game = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, title FROM games WHERE id = $1",
    )
    .bind(game_id)
    .fetch_optional(pool.get_ref())
    .await;

    match game {
        Ok(Some((id, title))) => {
            // Récupérer les plateformes liées
            let platforms = sqlx::query_as::<_, (String, String, Option<String>, i32)>(
                "SELECT platform::text, platform_game_id, platform_name, total_achievements FROM game_platform_ids WHERE game_id = $1",
            )
            .bind(id)
            .fetch_all(pool.get_ref())
            .await
            .unwrap_or_default();

            // Stats de completion depuis user_game_stats
            let stats = sqlx::query_as::<_, (i32, i32)>(
                "SELECT COALESCE(achievements_total, 0), COALESCE(achievements_unlocked, 0) FROM user_game_stats WHERE game_id = $1 AND user_id = $2",
            )
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool.get_ref())
            .await
            .unwrap_or_default()
            .unwrap_or((0, 0));

            HttpResponse::Ok().json(serde_json::json!({
                "id": id,
                "title": title,
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
