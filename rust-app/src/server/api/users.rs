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
    total_achievements: i64,
    completion_avg: f64,
    total_possible_achievements: i64,
    games_completed: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformRow {
    platform: String,
    platform_username: Option<String>,
    connected_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct GameRow {
    title: String,
    cover_image_url: Option<String>,
    completion_pct: f64,
    playtime_minutes: Option<i32>,
    achievements_unlocked: i32,
    achievements_total: i32,
    platform: String,
    platform_game_id: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct AchievementRow {
    name: String,
    description: Option<String>,
    icon_url: Option<String>,
    rarity: String,
    points: i32,
    game_title: String,
    unlocked_at: Option<DateTime<Utc>>,
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
            .route("/me/avatar", web::post().to(upload_avatar))
            .route("/me/steam-apikey", web::get().to(get_steam_apikey))
            .route("/me/steam-apikey", web::patch().to(set_steam_apikey))
            .route("/me/titles", web::get().to(get_my_titles))
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
            "UPDATE users SET steam_api_key_enc = NULL, updated_at = NOW() WHERE id = $1",
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

    let encrypted = match crate::server::crypto::encrypt(key) {
        Ok(enc) => enc,
        Err(e) => {
            tracing::error!("Erreur chiffrement cle API: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Erreur lors du chiffrement" }));
        }
    };

    match sqlx::query(
        "UPDATE users SET steam_api_key_enc = $1, updated_at = NOW() WHERE id = $2 AND is_active = true",
    )
    .bind(&encrypted)
    .bind(claims.sub)
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "ok": true, "has_key": true })),
        Err(e) => {
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
        SELECT
            u.username,
            u.display_name,
            u.profile_image_url,
            u.active_title,
            u.league::TEXT AS league,
            u.total_points,
            u.created_at,
            COALESCE(lc.rank_global, 0)::BIGINT AS rank,
            u.total_achievements_count::BIGINT AS total_achievements,
            COALESCE(lc.completion_avg, 0.0)::FLOAT8 AS completion_avg,
            u.total_possible_achievements::BIGINT AS total_possible_achievements,
            u.games_completed::BIGINT AS games_completed
        FROM users u
        LEFT JOIN leaderboard_cache lc ON lc.user_id = u.id
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

    let games = sqlx::query_as::<_, GameRow>(
        r#"
        SELECT
            g.title,
            g.cover_image_url,
            ugs.completion_pct::FLOAT8 AS completion_pct,
            ugs.playtime_minutes,
            ugs.achievements_unlocked,
            ugs.achievements_total,
            gpi.platform::TEXT AS platform,
            gpi.platform_game_id
        FROM user_game_stats ugs
        JOIN games g ON g.id = ugs.game_id
        JOIN game_platform_ids gpi ON g.id = gpi.game_id
        WHERE ugs.user_id = (SELECT id FROM users WHERE username = $1)
        ORDER BY ugs.last_played_at DESC NULLS LAST, ugs.completion_pct DESC
        "#,
    )
    .bind(&username)
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let completed_games = sqlx::query_as::<_, GameRow>(
        r#"
        SELECT
            g.title,
            g.cover_image_url,
            ugs.completion_pct::FLOAT8 AS completion_pct,
            ugs.playtime_minutes,
            ugs.achievements_unlocked,
            ugs.achievements_total,
            gpi.platform::TEXT AS platform,
            gpi.platform_game_id
        FROM user_game_stats ugs
        JOIN games g ON g.id = ugs.game_id
        JOIN game_platform_ids gpi ON g.id = gpi.game_id
        WHERE ugs.user_id = (SELECT id FROM users WHERE username = $1)
          AND ugs.achievements_total > 0
          AND ugs.achievements_unlocked >= ugs.achievements_total
        ORDER BY ugs.updated_at DESC
        LIMIT 5
        "#,
    )
    .bind(&username)
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let recent_achievements = sqlx::query_as::<_, AchievementRow>(
        r#"
        SELECT
            a.name,
            a.description,
            a.icon_url,
            a.rarity::TEXT AS rarity,
            a.points,
            g.title AS game_title,
            ua.unlocked_at
        FROM user_achievements ua
        JOIN achievements a ON a.id = ua.achievement_id
        JOIN game_platform_ids gpi ON gpi.id = a.game_platform_id
        JOIN games g ON g.id = gpi.game_id
        WHERE ua.user_id = (SELECT id FROM users WHERE username = $1)
          AND ua.is_unlocked = true
        ORDER BY ua.unlocked_at DESC
        LIMIT 5
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
        "total_achievements": profile.total_achievements,
        "total_possible_achievements": profile.total_possible_achievements,
        "completion_avg": profile.completion_avg,
        "games_completed": profile.games_completed,
        "member_since": profile.created_at,
        "platforms": platforms,
        "games": games,
        "completed_games": completed_games,
        "recent_achievements": recent_achievements,
    }))
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
