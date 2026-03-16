use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::models::platform_connection::{
    ConnectPlatformRequest, PlatformConnectionPublic, UpdateApikeyRequest,
};
use crate::server::auth::{extract_token_from_header, validate_token};

// ─── Cache Steam en mémoire ───────────────────────────────────────────────────

const STEAM_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

struct SteamCacheEntry {
    data: serde_json::Value,
    fetched_at: Instant,
}

impl SteamCacheEntry {
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < STEAM_CACHE_TTL
    }
}

pub struct SteamCache {
    recent_achievements: HashMap<String, SteamCacheEntry>,
    completed_games: HashMap<String, SteamCacheEntry>,
}

impl SteamCache {
    pub fn new() -> Self {
        Self {
            recent_achievements: HashMap::new(),
            completed_games: HashMap::new(),
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/platforms")
            .route("", web::get().to(list_connections))
            // Routes Steam OpenID (avant /{platform} pour éviter tout conflit)
            .route("/steam/auth", web::get().to(steam_openid_auth))
            .route("/steam/callback", web::get().to(steam_openid_callback))
            // Données Steam en temps réel
            .route("/steam/games", web::get().to(steam_games))
            .route("/steam/games/completed", web::get().to(steam_completed_games))
            .route("/steam/achievements/recent", web::get().to(steam_recent_achievements))
            // Données GOG depuis la DB
            .route("/gog/verify", web::get().to(gog_verify_user))
            .route("/gog/games", web::get().to(gog_games))
            .route("/gog/games/completed", web::get().to(gog_completed_games))
            .route("/gog/achievements/recent", web::get().to(gog_recent_achievements))
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
        r#"
        SELECT pc.id, pc.platform::text, pc.platform_username, pc.last_synced_at,
               CASE
                 WHEN pc.platform = 'steam'::platform_type
                   THEN (SELECT steam_api_key_enc IS NOT NULL FROM users WHERE id = $1)
                 ELSE (pc.access_token IS NOT NULL)
               END AS has_api_key
        FROM platform_connections pc
        WHERE pc.user_id = $1
        "#,
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
    cache: web::Data<Mutex<SteamCache>>,
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

                    let api_key_source = if decrypted_key.is_some() {
                        "personal"
                    } else if access_token.is_some() {
                        "connection"
                    } else {
                        "server"
                    };

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

                            // Invalider le cache Steam pour que le prochain chargement soit frais
                            if let Ok(mut c) = cache.lock() {
                                c.recent_achievements.remove(&platform_user_id);
                                c.completed_games.remove(&platform_user_id);
                            }

                            HttpResponse::Ok().json(serde_json::json!({
                                "message": "Synchronisation Steam terminee",
                                "games_synced": stats.games_synced,
                                "achievements_synced": stats.achievements_synced,
                                "total_achievements": stats.total_achievements,
                                "games_completed": stats.games_completed,
                                "api_key_source": api_key_source,
                            }))
                        }
                        Err(e) => {
                            tracing::error!("Erreur sync Steam: {}", e);
                            HttpResponse::InternalServerError()
                                .json(serde_json::json!({"error": format!("Erreur sync Steam: {}", e)}))
                        }
                    }
                }
                "gog" => {
                    match crate::server::platforms::gog::sync_gog_achievements(
                        pool.get_ref(),
                        user_id,
                        &platform_user_id,
                        access_token.as_deref().unwrap_or(""),
                    )
                    .await
                    {
                        Ok(stats) => {
                            let _ = sqlx::query(
                                "UPDATE platform_connections SET last_synced_at = NOW() WHERE user_id = $1 AND platform = 'gog'::platform_type",
                            )
                            .bind(user_id)
                            .execute(pool.get_ref())
                            .await;

                            HttpResponse::Ok().json(serde_json::json!({
                                "message": "Synchronisation GOG terminée",
                                "games_synced": stats.games_synced,
                                "achievements_synced": stats.achievements_synced,
                                "total_achievements": stats.total_achievements,
                                "games_completed": stats.games_completed,
                            }))
                        }
                        Err(e) => {
                            tracing::error!("Erreur sync GOG: {}", e);
                            HttpResponse::InternalServerError()
                                .json(serde_json::json!({"error": format!("Erreur sync GOG: {}", e)}))
                        }
                    }
                }
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

/// GET /api/platforms/gog/verify?username={input} — résoudre un username ou ID GOG en ID numérique
/// Accepte un nom d'utilisateur GOG (ex: "g6itself") ou un ID numérique déjà connu.
/// Retourne { valid, user_id, username } — utilisé par le frontend avant de connecter le compte.
async fn gog_verify_user(query: web::Query<std::collections::HashMap<String, String>>) -> HttpResponse {
    let input = match query.get("username") {
        Some(u) if !u.is_empty() => u.clone(),
        _ => return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Paramètre username manquant" })),
    };

    match crate::server::platforms::gog::verify_gog_user(&input).await {
        Ok((user_id, username)) => HttpResponse::Ok().json(serde_json::json!({
            "valid": true,
            "user_id": user_id,
            "username": username,
        })),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({
            "valid": false,
            "error": format!("{}", e),
        })),
    }
}

/// GET /api/platforms/steam/games — liste des jeux Steam avec données de complétion
async fn steam_games(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (steam_id, api_key) =
        match crate::server::platforms::steam::get_user_steam_credentials(pool.get_ref(), user_id)
            .await
        {
            Ok(creds) => creds,
            Err(e) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": format!("{}", e) }))
            }
        };

    let mut games =
        match crate::server::platforms::steam::fetch_owned_games(&steam_id, &api_key).await {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("Erreur fetch_owned_games: {}", e);
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": "Erreur lors de la récupération des jeux Steam" }));
            }
        };

    // Récupérer les données de complétion depuis user_game_stats
    let stats_rows = sqlx::query_as::<_, (String, i32, i32, f64)>(
        r#"
        SELECT gpi.platform_game_id,
               ugs.achievements_unlocked,
               ugs.achievements_total,
               ugs.completion_pct
        FROM user_game_stats ugs
        JOIN game_platform_ids gpi ON gpi.game_id = ugs.game_id
            AND gpi.platform = 'steam'::platform_type
        WHERE ugs.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    // Index appid → (unlocked, total, pct)
    let stats_map: std::collections::HashMap<u64, (i32, i32, f64)> = stats_rows
        .into_iter()
        .filter_map(|(appid_str, unlocked, total, pct)| {
            appid_str.parse::<u64>().ok().map(|id| (id, (unlocked, total, pct)))
        })
        .collect();

    // Construire la réponse JSON enrichie
    let result: Vec<serde_json::Value> = games
        .iter_mut()
        .map(|g| {
            let (unlocked, total, pct) = stats_map.get(&g.appid).copied().unwrap_or((0, 0, 0.0));
            serde_json::json!({
                "appid": g.appid,
                "name": g.name,
                "playtime_minutes": g.playtime_minutes,
                "img_icon_url": g.img_icon_url,
                "last_played_at": g.last_played_at,
                "achievements_unlocked": unlocked,
                "achievements_total": total,
                "completion_pct": pct,
            })
        })
        .collect();

    HttpResponse::Ok().json(result)
}

/// GET /api/platforms/steam/games/completed — 5 derniers jeux complétés à 100%
async fn steam_completed_games(
    pool: web::Data<PgPool>,
    cache: web::Data<Mutex<SteamCache>>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (steam_id, api_key) =
        match crate::server::platforms::steam::get_user_steam_credentials(pool.get_ref(), user_id)
            .await
        {
            Ok(creds) => creds,
            Err(e) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": format!("{}", e) }))
            }
        };

    // Vérifier le cache
    if let Ok(c) = cache.lock() {
        if let Some(entry) = c.completed_games.get(&steam_id) {
            if entry.is_fresh() {
                return HttpResponse::Ok().json(&entry.data);
            }
        }
    }

    match crate::server::platforms::steam::fetch_completed_games(&steam_id, &api_key).await {
        Ok(games) => {
            let json_val = serde_json::to_value(&games).unwrap_or_default();
            // Ne cacher que si on a trouvé au moins un jeu (évite de figer un tableau vide)
            if !games.is_empty() {
                if let Ok(mut c) = cache.lock() {
                    c.completed_games.insert(steam_id, SteamCacheEntry {
                        data: json_val.clone(),
                        fetched_at: Instant::now(),
                    });
                }
            }
            HttpResponse::Ok().json(json_val)
        }
        Err(e) => {
            tracing::error!("Erreur fetch_completed_games: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur lors de la récupération des jeux complétés" }))
        }
    }
}

/// GET /api/platforms/steam/achievements/recent — 5 derniers achievements débloqués
async fn steam_recent_achievements(
    pool: web::Data<PgPool>,
    cache: web::Data<Mutex<SteamCache>>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (steam_id, api_key) =
        match crate::server::platforms::steam::get_user_steam_credentials(pool.get_ref(), user_id)
            .await
        {
            Ok(creds) => creds,
            Err(e) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": format!("{}", e) }))
            }
        };

    // Vérifier le cache
    if let Ok(c) = cache.lock() {
        if let Some(entry) = c.recent_achievements.get(&steam_id) {
            if entry.is_fresh() {
                return HttpResponse::Ok().json(&entry.data);
            }
        }
    }

    match crate::server::platforms::steam::fetch_recent_achievements(&steam_id, &api_key).await {
        Ok(achievements) => {
            let json_val = serde_json::to_value(&achievements).unwrap_or_default();
            if !achievements.is_empty() {
                if let Ok(mut c) = cache.lock() {
                    c.recent_achievements.insert(steam_id, SteamCacheEntry {
                        data: json_val.clone(),
                        fetched_at: Instant::now(),
                    });
                }
            }
            HttpResponse::Ok().json(json_val)
        }
        Err(e) => {
            tracing::error!("Erreur fetch_recent_achievements: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur lors de la récupération des achievements récents" }))
        }
    }
}

/// GET /api/platforms/gog/games — liste des jeux GOG avec données de complétion (depuis DB)
async fn gog_games(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match crate::server::platforms::gog::fetch_gog_games_from_db(pool.get_ref(), user_id).await {
        Ok(games) => HttpResponse::Ok().json(games),
        Err(e) => {
            tracing::error!("Erreur fetch_gog_games_from_db: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur lors de la récupération des jeux GOG" }))
        }
    }
}

/// GET /api/platforms/gog/games/completed — jeux GOG complétés à 100% (depuis DB)
async fn gog_completed_games(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match crate::server::platforms::gog::fetch_gog_completed_from_db(pool.get_ref(), user_id).await {
        Ok(games) => HttpResponse::Ok().json(games),
        Err(e) => {
            tracing::error!("Erreur fetch_gog_completed_from_db: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur lors de la récupération des jeux GOG complétés" }))
        }
    }
}

/// GET /api/platforms/gog/achievements/recent — achievements GOG récents (API GOG + token Bearer)
async fn gog_recent_achievements(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (gog_user_id, access_token) =
        match crate::server::platforms::gog::get_user_gog_credentials(pool.get_ref(), user_id).await {
            Ok(creds) => creds,
            Err(e) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": format!("{}", e) }))
            }
        };

    match crate::server::platforms::gog::fetch_gog_recent_achievements(
        pool.get_ref(),
        user_id,
        &gog_user_id,
        access_token.as_deref(),
    )
    .await
    {
        Ok(achievements) => HttpResponse::Ok().json(achievements),
        Err(e) => {
            tracing::error!("Erreur fetch_gog_recent_achievements: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur lors de la récupération des achievements GOG récents" }))
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
