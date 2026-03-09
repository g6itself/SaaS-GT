use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::models::platform_connection::{
    ConnectPlatformRequest, PlatformConnectionPublic, UpdateApikeyRequest,
};
use crate::server::auth::{extract_token_from_header, validate_token};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/platforms")
            .route("", web::get().to(list_connections))
            // Routes Steam OpenID (avant /{platform} pour éviter tout conflit)
            .route("/steam/auth", web::get().to(steam_openid_auth))
            .route("/steam/callback", web::get().to(steam_openid_callback))
            .route("/{platform}", web::post().to(connect_platform))
            .route("/{platform}", web::delete().to(disconnect_platform))
            .route("/{platform}/sync", web::post().to(sync_platform))
            .route("/{platform}/apikey", web::patch().to(update_platform_apikey)),
    );
}

// ─── Steam OpenID ───────────────────────────────────────────────────────────

/// Retourne l'URL de redirection Steam OpenID pour l'utilisateur authentifié.
async fn steam_openid_auth(req: HttpRequest) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let base_url = std::env::var("APP_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3100".to_string());

    let return_to = format!(
        "{}/api/platforms/steam/callback?uid={}",
        base_url, user_id
    );

    let auth_url =
        crate::server::platforms::steam::steam_openid_url(&return_to, &base_url);

    HttpResponse::Ok().json(serde_json::json!({ "redirect_url": auth_url }))
}

/// Callback Steam OpenID : vérifie la signature, enregistre la connexion,
/// redirige vers le profil.
async fn steam_openid_callback(
    pool: web::Data<PgPool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    // Récupérer l'user_id passé dans le return_to
    let uid_str = match query.get("uid") {
        Some(v) => v.clone(),
        None => return profile_redirect(Some("Paramètre uid manquant")),
    };
    let user_id = match uid_str.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => return profile_redirect(Some("uid invalide")),
    };

    // Vérifier la signature OpenID Steam et extraire le SteamID64
    let steam_id = match crate::server::platforms::steam::verify_steam_openid(&query).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Steam OpenID verification failed: {}", e);
            return profile_redirect(Some("Vérification Steam échouée"));
        }
    };

    // Récupérer le nom d'affichage Steam automatiquement
    let platform_username =
        crate::server::platforms::steam::fetch_steam_player_summary(&steam_id)
            .await
            .unwrap_or_else(|_| steam_id.clone());

    let result = sqlx::query(
        r#"
        INSERT INTO platform_connections (user_id, platform, platform_user_id, platform_username)
        VALUES ($1, 'steam'::platform_type, $2, $3)
        ON CONFLICT (user_id, platform) DO UPDATE
        SET platform_user_id = $2, platform_username = $3, updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&steam_id)
    .bind(&platform_username)
    .execute(pool.get_ref())
    .await;

    if let Err(e) = result {
        tracing::error!("Erreur sauvegarde connexion Steam: {}", e);
        return profile_redirect(Some("Erreur lors de l'enregistrement"));
    }

    profile_redirect(None)
}

fn profile_redirect(error: Option<&str>) -> HttpResponse {
    let url = match error {
        None => "/profile.html?steam_ok=1".to_string(),
        Some(msg) => format!(
            "/profile.html?steam_err={}",
            msg.replace(' ', "+")
        ),
    };
    HttpResponse::Found()
        .append_header(("Location", url))
        .finish()
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

    let connections = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>, bool)>(
        "SELECT id, platform::text, platform_username, last_synced_at, (access_token IS NOT NULL) as has_api_key FROM platform_connections WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool.get_ref())
    .await;

    match connections {
        Ok(rows) => {
            let result: Vec<PlatformConnectionPublic> = rows
                .into_iter()
                .map(|(id, platform, platform_username, last_synced_at, has_api_key)| {
                    PlatformConnectionPublic {
                        id,
                        platform,
                        platform_username,
                        last_synced_at,
                        connected: true,
                        has_api_key,
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

    // Si le nom n'est pas fourni et que c'est Steam, le récupérer automatiquement
    let platform_username: Option<String> = match body.platform_username.clone() {
        Some(name) if !name.is_empty() => Some(name),
        _ if platform == "steam" => {
            crate::server::platforms::steam::fetch_steam_player_summary(&body.platform_user_id)
                .await
                .ok()
        }
        _ => None,
    };

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
    .bind(&platform_username)
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
        Ok(Some((_conn_id, platform_user_id, access_token))) => {
            // Lancer la synchronisation selon la plateforme
            match platform.as_str() {
                "steam" => {
                    // Priorite : cle chiffree dans users > access_token > STEAM_API_KEY env
                    let encrypted_key = sqlx::query_scalar::<_, Option<String>>(
                        "SELECT steam_api_key_enc FROM users WHERE id = $1",
                    )
                    .bind(user_id)
                    .fetch_optional(pool.get_ref())
                    .await
                    .ok()
                    .flatten()
                    .flatten();

                    let decrypted_key = encrypted_key.as_deref().and_then(|enc| {
                        crate::server::crypto::decrypt(enc)
                            .map_err(|e| tracing::warn!("Echec dechiffrement cle API: {}", e))
                            .ok()
                    });

                    let user_api_key = decrypted_key
                        .as_deref()
                        .or(access_token.as_deref());

                    match crate::server::platforms::steam::sync_steam_achievements(
                        pool.get_ref(),
                        user_id,
                        &platform_user_id,
                        user_api_key,
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
                                "total_achievements": stats.total_achievements,
                                "games_completed": stats.games_completed,
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

async fn update_platform_apikey(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateApikeyRequest>,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let platform = path.into_inner();

    let result = sqlx::query(
        "UPDATE platform_connections SET access_token = $1, updated_at = NOW() \
         WHERE user_id = $2 AND platform = $3::platform_type",
    )
    .bind(&body.access_token)
    .bind(user_id)
    .bind(&platform)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            HttpResponse::Ok().json(serde_json::json!({"message": "Cle API mise a jour"}))
        }
        Ok(_) => HttpResponse::NotFound()
            .json(serde_json::json!({"error": "Connexion non trouvee pour cette plateforme"})),
        Err(e) => {
            tracing::error!("Erreur mise a jour cle API: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur interne du serveur"}))
        }
    }
}
