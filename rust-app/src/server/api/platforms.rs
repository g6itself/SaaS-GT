use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::models::platform_connection::{ConnectPlatformRequest, PlatformConnectionPublic};
use crate::server::auth::{extract_token_from_header, validate_token};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/platforms")
            .route("", web::get().to(list_connections))
            .route("/{platform}", web::post().to(connect_platform))
            .route("/{platform}", web::delete().to(disconnect_platform))
            .route("/{platform}/sync", web::post().to(sync_platform)),
    );
}

/// Extrait l'ID utilisateur depuis le token JWT
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

async fn list_connections(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let connections = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, platform::text, platform_username, last_synced_at FROM platform_connections WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool.get_ref())
    .await;

    match connections {
        Ok(rows) => {
            let result: Vec<PlatformConnectionPublic> = rows
                .into_iter()
                .map(|(id, platform, platform_username, last_synced_at)| {
                    PlatformConnectionPublic {
                        id,
                        platform,
                        platform_username,
                        last_synced_at,
                        connected: true,
                    }
                })
                .collect();
            HttpResponse::Ok().json(result)
        }
        Err(e) => {
            tracing::error!("Erreur liste connexions: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}

async fn connect_platform(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<ConnectPlatformRequest>,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let platform = path.into_inner();

    // Valider la plateforme
    if !["steam", "gog", "epic"].contains(&platform.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Plateforme non supportee. Valeurs: steam, gog, epic"}));
    }

    let result = sqlx::query(
        r#"
        INSERT INTO platform_connections (user_id, platform, platform_user_id, platform_username, access_token)
        VALUES ($1, $2::platform_type, $3, $4, $5)
        ON CONFLICT (user_id, platform) DO UPDATE
        SET platform_user_id = $3, platform_username = $4, access_token = $5, updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&platform)
    .bind(&body.platform_user_id)
    .bind(&body.platform_username)
    .bind(&body.access_token)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok()
            .json(serde_json::json!({"message": format!("Compte {} lie avec succes", platform)})),
        Err(e) => {
            tracing::error!("Erreur connexion plateforme: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur lors de la liaison du compte"}))
        }
    }
}

async fn disconnect_platform(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let platform = path.into_inner();

    let result = sqlx::query(
        "DELETE FROM platform_connections WHERE user_id = $1 AND platform = $2::platform_type",
    )
    .bind(user_id)
    .bind(&platform)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok()
            .json(serde_json::json!({"message": format!("Compte {} delie", platform)})),
        Ok(_) => HttpResponse::NotFound()
            .json(serde_json::json!({"error": "Connexion non trouvee"})),
        Err(e) => {
            tracing::error!("Erreur deconnexion plateforme: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}

async fn sync_platform(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let platform = path.into_inner();

    // Verifier que la connexion existe
    let connection = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>)>(
        "SELECT id, platform_user_id, access_token FROM platform_connections WHERE user_id = $1 AND platform = $2::platform_type",
    )
    .bind(user_id)
    .bind(&platform)
    .fetch_optional(pool.get_ref())
    .await;

    match connection {
        Ok(Some((_conn_id, platform_user_id, _access_token))) => {
            // Lancer la synchronisation selon la plateforme
            match platform.as_str() {
                "steam" => {
                    match crate::server::platforms::steam::sync_steam_achievements(
                        pool.get_ref(),
                        user_id,
                        &platform_user_id,
                    )
                    .await
                    {
                        Ok(stats) => {
                            // Mettre a jour last_synced_at
                            let _ = sqlx::query(
                                "UPDATE platform_connections SET last_synced_at = NOW() WHERE user_id = $1 AND platform = 'steam'::platform_type",
                            )
                            .bind(user_id)
                            .execute(pool.get_ref())
                            .await;

                            HttpResponse::Ok().json(serde_json::json!({
                                "message": "Synchronisation Steam terminee",
                                "games_synced": stats.games_synced,
                                "achievements_synced": stats.achievements_synced,
                            }))
                        }
                        Err(e) => {
                            tracing::error!("Erreur sync Steam: {}", e);
                            HttpResponse::InternalServerError()
                                .json(serde_json::json!({"error": format!("Erreur sync Steam: {}", e)}))
                        }
                    }
                }
                "gog" => HttpResponse::NotImplemented()
                    .json(serde_json::json!({"error": "Synchronisation GOG pas encore implementee"})),
                "epic" => HttpResponse::NotImplemented()
                    .json(serde_json::json!({"error": "Synchronisation Epic pas encore implementee"})),
                _ => HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Plateforme non supportee"})),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({"error": "Aucune connexion trouvee pour cette plateforme"})),
        Err(e) => {
            tracing::error!("Erreur sync: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}
