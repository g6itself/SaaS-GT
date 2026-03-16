use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::server::auth::{extract_token_from_header, validate_token};

// ── Structs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
struct UserProfileRow {
    username: String,
    display_name: Option<String>,
    profile_image_url: Option<String>,
    active_title: String,
    league: String,
    total_points: i64,
    created_at: DateTime<Utc>,
    rank: i64,
    total_players: i64,
    rank_snapshot: Option<i64>,
    rank_snapshot_at: Option<DateTime<Utc>>,
    total_achievements: i64,
    completion_avg: f64,
    total_possible_achievements: i64,
    games_completed: i64,
    last_seen_at: Option<DateTime<Utc>>,
    is_online: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformRow {
    platform: String,
    platform_username: Option<String>,
    connected_at: DateTime<Utc>,
}


#[derive(Debug, Deserialize)]
struct UpdateProfileRequest {
    display_name: Option<String>,
    profile_image_url: Option<String>,
    active_title: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct TitleRow {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SteamApikeyRequest {
    api_key: String,
}

// ── Routes ────────────────────────────────────────────────────────────────────

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .route("/me", web::put().to(update_me))
            .route("/me", web::delete().to(delete_me))
            .route("/me/delete", web::post().to(delete_me))
            .route("/me/avatar", web::post().to(upload_avatar))
            .route("/me/steam-apikey", web::get().to(get_steam_apikey))
            .route("/me/steam-apikey", web::patch().to(set_steam_apikey))
            .route("/me/titles", web::get().to(get_my_titles))
            .route("/me/heartbeat", web::patch().to(heartbeat))
            .route("/{username}", web::get().to(get_user_profile)),
    );
}

// ── POST /api/users/me/avatar ─────────────────────────────────────────────────

async fn upload_avatar(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    mut payload: actix_multipart::Multipart,
) -> HttpResponse {
    use futures_util::StreamExt;
    use std::io::Write;
    use tokio::fs;

    let auth_header = match req.headers().get("Authorization") {
        Some(h) => h.to_str().unwrap_or("").to_string(),
        None => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({ "error": "Token manquant" }))
        }
    };

    let token = match extract_token_from_header(&auth_header) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({ "error": "Format de token invalide" }))
        }
    };

    let claims = match validate_token(token) {
        Ok(c) => c,
        Err(_) => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({ "error": "Token invalide" }))
        }
    };

    let upload_dir = "uploads";
    if let Err(e) = fs::create_dir_all(upload_dir).await {
        tracing::error!("Erreur création dossier uploads: {}", e);
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": "Erreur serveur" }));
    }

    let mut saved_filename = String::new();
    let mut saved_filepath = String::new(); // chemin complet pour nettoyage si la DB échoue

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(_) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": "Erreur upload" }))
            }
        };

        // Validation MIME type
        let content_type = field
            .content_type()
            .map(|ct| ct.to_string())
            .unwrap_or_default();
        if !["image/png", "image/jpeg", "image/webp", "image/gif"].contains(&content_type.as_str())
        {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Type MIME non supporte (PNG, JPG, WEBP, GIF uniquement)"
            }));
        }

        let content_disposition = field.content_disposition();
        let filename = content_disposition
            .get_filename()
            .map_or_else(|| uuid::Uuid::new_v4().to_string(), |f| f.to_string());

        // Validation extension
        let ext = if filename.to_lowercase().ends_with(".png") {
            ".png"
        } else if filename.to_lowercase().ends_with(".jpg")
            || filename.to_lowercase().ends_with(".jpeg")
        {
            ".jpg"
        } else if filename.to_lowercase().ends_with(".webp") {
            ".webp"
        } else if filename.to_lowercase().ends_with(".gif") {
            ".gif"
        } else {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Format non supporté (PNG, JPG, WEBP, GIF)" }));
        };

        saved_filename = format!("{}_{}{}", claims.sub, uuid::Uuid::new_v4().simple(), ext);
        let filepath = format!("{}/{}", upload_dir, saved_filename);
        saved_filepath = filepath.clone();

        let mut f = match std::fs::File::create(&filepath) {
            Ok(file) => file,
            Err(e) => {
                tracing::error!("Erreur création fichier local: {}", e);
                saved_filepath.clear(); // pas encore créé, rien à nettoyer
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": "Erreur serveur" }));
            }
        };

        let mut size: usize = 0;
        let max_size = 20 * 1024 * 1024; // 20 MB

        while let Some(chunk) = field.next().await {
            let data = match chunk {
                Ok(d) => d,
                Err(_) => {
                    let _ = std::fs::remove_file(&filepath);
                    return HttpResponse::BadRequest()
                        .json(serde_json::json!({ "error": "Erreur transfert" }));
                }
            };
            size += data.len();
            if size > max_size {
                let _ = std::fs::remove_file(&filepath);
                return HttpResponse::PayloadTooLarge()
                    .json(serde_json::json!({ "error": "Fichier trop volumineux (20 Mo max)" }));
            }
            f = match web::block(move || f.write_all(&data).map(|_| f)).await {
                Ok(Ok(file)) => file,
                Ok(Err(e)) => {
                    tracing::error!("Erreur ecriture fichier: {}", e);
                    let _ = std::fs::remove_file(&filepath);
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": "Erreur serveur lors de l'ecriture" }));
                }
                Err(e) => {
                    tracing::error!("Erreur block: {}", e);
                    let _ = std::fs::remove_file(&filepath);
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": "Erreur serveur" }));
                }
            };
        }
    }

    if saved_filename.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Aucun fichier reçu" }));
    }

    let new_url = format!("/uploads/avatars/{}", saved_filename);

    match sqlx::query("UPDATE users SET profile_image_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(&new_url)
        .bind(claims.sub)
        .execute(pool.get_ref())
        .await
    {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "profile_image_url": new_url
        })),
        Err(e) => {
            tracing::error!("Erreur BDD update avatar: {}", e);
            // Supprimer le fichier écrit sur disque pour éviter les orphelins
            if let Err(rm_err) = std::fs::remove_file(&saved_filepath) {
                tracing::warn!(
                    "Fichier orphelin non supprimé {}: {}",
                    saved_filepath,
                    rm_err
                );
            }
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur serveur" }))
        }
    }
}

// ── PUT /api/users/me ─────────────────────────────────────────────────────────

async fn update_me(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    body: web::Json<UpdateProfileRequest>,
) -> HttpResponse {
    let auth_header = match req.headers().get("Authorization") {
        Some(h) => h.to_str().unwrap_or("").to_string(),
        None => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({ "error": "Token manquant" }))
        }
    };

    let token = match extract_token_from_header(&auth_header) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({ "error": "Format de token invalide" }))
        }
    };

    let claims = match validate_token(token) {
        Ok(c) => c,
        Err(_) => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({ "error": "Token invalide" }))
        }
    };

    let display_name = body
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);

    // Validation display_name
    if let Some(ref name) = display_name {
        if name.len() < 2 || name.len() > 32 {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Le nom d'affichage doit contenir entre 2 et 32 caracteres"
            }));
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
        {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Le nom d'affichage contient des caracteres non autorises"
            }));
        }
    }

    let profile_image_url = match body.profile_image_url.as_deref().map(str::trim) {
        // URL externe fournie : valider le protocole
        Some(url)
            if !url.is_empty()
                && !url.starts_with('/')
                && !url.starts_with("https://")
                && !url.starts_with("http://")
                && !url.starts_with("data:image/") =>
        {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "URL d'image invalide"
            }));
        }
        Some("") => {
            let username =
                sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
                    .bind(claims.sub)
                    .fetch_one(pool.get_ref())
                    .await
                    .unwrap_or_else(|_| "?".to_string());
            Some(crate::server::api::auth::generate_avatar_url(&username))
        }
        Some(s) => Some(s.to_owned()),
        None => None,
    };

    // Si on fournit une valeur, on l'utilise, sinon on garde la valeur actuelle de la base (COALESCE)
    // Coalesce ($1, display_name) permet de ne pas écraser la valeur si $1 est NULL (None en Rust).
    match sqlx::query(
        r#"
        UPDATE users 
        SET 
            display_name = COALESCE($1, display_name), 
            profile_image_url = COALESCE($2, profile_image_url),
            active_title = COALESCE($3, active_title),
            updated_at = NOW() 
        WHERE id = $4 AND is_active = true
        "#,
    )
    .bind(&display_name)
    .bind(&profile_image_url)
    .bind(&body.active_title)
    .bind(claims.sub)
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true })),
        Err(e) => {
            tracing::error!("Erreur update_me: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur interne du serveur" }))
        }
    }
}

// ── DELETE /api/users/me ──────────────────────────────────────────────────────

async fn delete_me(pool: web::Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let claims = match auth_from_request(&req) {
        Ok(c) => c,
        Err(r) => return r,
    };

    match sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(claims.sub)
        .execute(pool.get_ref())
        .await
    {
        Ok(_) => {
            tracing::info!("Compte supprime: {}", claims.sub);
            HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
        }
        Err(e) => {
            tracing::error!("Erreur DELETE user {}: {}", claims.sub, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur lors de la suppression du compte"
            }))
        }
    }
}

// ── GET /api/users/me/steam-apikey ───────────────────────────────────────────

async fn get_steam_apikey(pool: web::Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let claims = match auth_from_request(&req) {
        Ok(c) => c,
        Err(r) => return r,
    };

    let row = sqlx::query_scalar::<_, Option<String>>(
        "SELECT steam_api_key_enc FROM users WHERE id = $1 AND is_active = true",
    )
    .bind(claims.sub)
    .fetch_optional(pool.get_ref())
    .await;

    match row {
        Ok(Some(Some(_))) => HttpResponse::Ok().json(serde_json::json!({ "has_key": true })),
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "has_key": false })),
        Err(e) => {
            tracing::error!("Erreur get_steam_apikey: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur interne du serveur" }))
        }
    }
}

// ── PATCH /api/users/me/steam-apikey ─────────────────────────────────────────

async fn set_steam_apikey(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    body: web::Json<SteamApikeyRequest>,
) -> HttpResponse {
    let claims = match auth_from_request(&req) {
        Ok(c) => c,
        Err(r) => return r,
    };

    let key = body.api_key.trim();

    // Supprimer la cle si vide
    if key.is_empty() {
        let _ = sqlx::query(
            "UPDATE users SET steam_api_key_enc = NULL, steam_api_key_hash = NULL, updated_at = NOW() WHERE id = $1",
        )
        .bind(claims.sub)
        .execute(pool.get_ref())
        .await;
        return HttpResponse::Ok().json(serde_json::json!({ "ok": true, "has_key": false }));
    }

    // Valider format Steam API key (32 caracteres alphanumeriques)
    if key.len() != 32 || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "La cle API Steam doit contenir exactement 32 caracteres alphanumeriques"
        }));
    }

    // Vérifier que la clé est valide auprès de l'API Steam
    {
        let check_url = format!(
            "https://api.steampowered.com/ISteamWebAPIUtil/GetSupportedAPIList/v1/?key={}",
            key
        );
        let valid = match reqwest::get(&check_url).await {
            Ok(r) => r.status().is_success(),
            Err(_) => {
                return HttpResponse::BadGateway().json(serde_json::json!({
                    "error": "Impossible de vérifier la clé API Steam. Réessayez dans un instant."
                }));
            }
        };
        if !valid {
            return HttpResponse::UnprocessableEntity().json(serde_json::json!({
                "error": "Clé API Steam invalide. Vérifiez la clé sur steamcommunity.com/dev/apikey"
            }));
        }
    }

    // Vérifier l'unicité : la clé ne doit pas être déjà utilisée par un autre compte.
    // On utilise un hash SHA-256 déterministe calculé par PostgreSQL (extension pgcrypto).
    let already_used = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM users
            WHERE steam_api_key_hash = encode(digest($1, 'sha256'), 'hex')
              AND id != $2
        )
        "#,
    )
    .bind(key)
    .bind(claims.sub)
    .fetch_one(pool.get_ref())
    .await
    .unwrap_or(false);

    if already_used {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "Cette clé API Steam est déjà utilisée par un autre compte"
        }));
    }

    let encrypted = match crate::server::crypto::encrypt(key) {
        Ok(enc) => enc,
        Err(e) => {
            tracing::error!("Erreur chiffrement cle API: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur lors du chiffrement" }));
        }
    };

    match sqlx::query(
        r#"
        UPDATE users
        SET steam_api_key_enc  = $1,
            steam_api_key_hash = encode(digest($2, 'sha256'), 'hex'),
            updated_at         = NOW()
        WHERE id = $3 AND is_active = true
        "#,
    )
    .bind(&encrypted)
    .bind(key)
    .bind(claims.sub)
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true, "has_key": true })),
        Err(e) => {
            // Violation de la contrainte UNIQUE (concurrence)
            if e.to_string().contains("uq_users_steam_api_key_hash") {
                return HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Cette clé API Steam est déjà utilisée par un autre compte"
                }));
            }
            tracing::error!("Erreur set_steam_apikey: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur interne du serveur" }))
        }
    }
}

/// Extrait et valide le token JWT depuis la requete
fn auth_from_request(req: &HttpRequest) -> Result<crate::server::auth::Claims, HttpResponse> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Token manquant" }))
        })?;

    let token = crate::server::auth::extract_token_from_header(auth_header).ok_or_else(|| {
        HttpResponse::Unauthorized()
            .json(serde_json::json!({ "error": "Format de token invalide" }))
    })?;

    crate::server::auth::validate_token(token).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Token invalide" }))
    })
}

// ── GET /api/users/{username} ─────────────────────────────────────────────────

async fn get_user_profile(pool: web::Data<PgPool>, path: web::Path<String>) -> HttpResponse {
    let username = path.into_inner();

    let profile = sqlx::query_as::<_, UserProfileRow>(
        r#"
        WITH ranked AS (
            SELECT
                id,
                RANK() OVER (ORDER BY total_points DESC)::BIGINT AS rank_pts,
                COUNT(*) OVER ()::BIGINT AS total_players
            FROM users
            WHERE is_active = true
        )
        SELECT
            u.username,
            u.display_name,
            u.profile_image_url,
            u.active_title,
            u.league::TEXT AS league,
            u.total_points,
            u.created_at,
            r.rank_pts AS rank,
            r.total_players,
            u.rank_snapshot,
            u.rank_snapshot_at,
            u.total_achievements_count::BIGINT AS total_achievements,
            0.0::FLOAT8 AS completion_avg,
            u.total_possible_achievements::BIGINT AS total_possible_achievements,
            u.games_completed::BIGINT AS games_completed,
            u.last_seen_at,
            (u.last_seen_at IS NOT NULL AND u.last_seen_at > NOW() - INTERVAL '15 minutes') AS is_online
        FROM users u
        JOIN ranked r ON r.id = u.id
        WHERE u.username = $1 AND u.is_active = true
        "#,
    )
    .bind(&username)
    .fetch_optional(pool.get_ref())
    .await;

    let profile = match profile {
        Ok(Some(p)) => p,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Joueur introuvable"
            }))
        }
        Err(e) => {
            tracing::error!("Erreur profile {}: {}", username, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur interne du serveur"
            }));
        }
    };

    let platforms = sqlx::query_as::<_, PlatformRow>(
        r#"
        SELECT
            platform::TEXT AS platform,
            platform_username,
            created_at AS connected_at
        FROM platform_connections
        WHERE user_id = (SELECT id FROM users WHERE username = $1)
        ORDER BY created_at ASC
        "#,
    )
    .bind(&username)
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    HttpResponse::Ok().json(serde_json::json!({
        "username": profile.username,
        "display_name": profile.display_name,
        "profile_image_url": profile.profile_image_url,
        "active_title": profile.active_title,
        "league": profile.league,
        "total_points": profile.total_points,
        "rank": profile.rank,
        "total_players": profile.total_players,
        "rank_snapshot": profile.rank_snapshot,
        "rank_snapshot_at": profile.rank_snapshot_at,
        "total_achievements": profile.total_achievements,
        "total_possible_achievements": profile.total_possible_achievements,
        "completion_avg": profile.completion_avg,
        "games_completed": profile.games_completed,
        "member_since": profile.created_at,
        "platforms": platforms,
        "is_online": profile.is_online,
        "last_seen_at": profile.last_seen_at,
    }))
}

// ── PATCH /api/users/me/heartbeat ─────────────────────────────────────────────

async fn heartbeat(pool: web::Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let claims = match auth_from_request(&req) {
        Ok(c) => c,
        Err(r) => return r,
    };
    sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
        .bind(claims.sub)
        .execute(pool.get_ref())
        .await
        .ok();
    HttpResponse::NoContent().finish()
}

// ── GET /api/users/me/titles ─────────────────────────────────────────────────

async fn get_my_titles(pool: web::Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let claims = match auth_from_request(&req) {
        Ok(c) => c,
        Err(r) => return r,
    };

    let titles = sqlx::query_as::<_, TitleRow>(
        r#"
        SELECT t.name, t.description
        FROM titles t
        JOIN user_titles ut ON ut.title_id = t.id
        WHERE ut.user_id = $1
        ORDER BY t.created_at ASC
        "#,
    )
    .bind(claims.sub)
    .fetch_all(pool.get_ref())
    .await;

    match titles {
        Ok(t) => HttpResponse::Ok().json(t),
        Err(e) => {
            tracing::error!("Erreur get_my_titles: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur lors de la récupération des titres"
            }))
        }
    }
}
