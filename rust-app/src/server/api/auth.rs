use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::models::user::{AuthResponse, LoginRequest, RegisterRequest, UserPublic};
use crate::server::auth::{
    create_token, extract_token_from_header, hash_password, validate_token, verify_password,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/me", web::get().to(me))
            .route("/check-username", web::get().to(check_username)),
    );
}

// ── Validation helpers ────────────────────────────────────────────────────────

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() {
        return false;
    }
    let domain = parts[1];
    domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.') && domain.len() > 2
}

fn is_valid_password(password: &str) -> (bool, bool) {
    let min_length = password.len() >= 12;
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    (min_length, has_special)
}

pub fn generate_avatar_url(username: &str) -> String {
    let name = if username.trim().is_empty() {
        "?"
    } else {
        username.trim()
    };
    let raw_text = name.chars().take(2).collect::<String>().to_uppercase();
    // Echapper les caracteres XML pour eviter l'injection SVG
    let text = raw_text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");

    let mut h: u32 = 0;
    for c in name.chars() {
        h = (h.wrapping_mul(31)).wrapping_add(c as u32) & 0xffff;
    }

    let hue1 = h % 360;
    let hue2 = (hue1 + 40) % 360;

    format!(
        "data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\" width=\"100\" height=\"100\"><defs><linearGradient id=\"g_{h}\" x1=\"0%25\" y1=\"0%25\" x2=\"100%25\" y2=\"100%25\"><stop offset=\"0%25\" stop-color=\"hsl({hue1}, 70%25, 55%25)\" /><stop offset=\"100%25\" stop-color=\"hsl({hue2}, 70%25, 45%25)\" /></linearGradient></defs><rect x=\"0\" y=\"0\" width=\"100\" height=\"100\" fill=\"url(%23g_{h})\"/><text x=\"50\" y=\"50\" dominant-baseline=\"central\" text-anchor=\"middle\" font-family=\"'Nunito', 'Quicksand', 'Segoe UI Rounded', sans-serif\" font-size=\"42\" font-weight=\"700\" letter-spacing=\"1\" fill=\"%23ffffff\">{text}</text></svg>",
        h=h,
        hue1=hue1,
        hue2=hue2,
        text=text
    )
}

// ── GET /api/auth/check-username?username=xxx ─────────────────────────────────

async fn check_username(
    pool: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let username = match query.get("username") {
        Some(u) => u.trim().to_string(),
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Parametre username manquant"
            }))
        }
    };

    if username.len() < 3 {
        return HttpResponse::Ok().json(serde_json::json!({ "available": false }));
    }

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(username) = LOWER($1))",
    )
    .bind(&username)
    .fetch_one(pool.get_ref())
    .await
    .unwrap_or(false);

    HttpResponse::Ok().json(serde_json::json!({ "available": !exists }))
}

// ── POST /api/auth/register ───────────────────────────────────────────────────

async fn register(pool: web::Data<PgPool>, body: web::Json<RegisterRequest>) -> HttpResponse {
    // ── Validation email ──
    if !is_valid_email(&body.email) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Adresse email invalide"
        }));
    }

    // ── Validation mot de passe ──
    let (pw_len_ok, pw_spec_ok) = is_valid_password(&body.password);
    if !pw_len_ok {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Le mot de passe doit contenir au moins 12 caracteres"
        }));
    }
    if !pw_spec_ok {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Le mot de passe doit contenir au moins un caractere special"
        }));
    }

    // ── Validation pseudo ──
    let trimmed = body.username.trim();
    if trimmed.len() < 3 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Le pseudo doit contenir au moins 3 caracteres"
        }));
    }
    if trimmed.len() > 32 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Le pseudo ne peut pas depasser 32 caracteres"
        }));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Le pseudo ne peut contenir que des lettres, chiffres, tirets (-) et underscores (_)"
        }));
    }

    let password_hash = match hash_password(&body.password) {
        Ok(hash) => hash,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur lors du hashage du mot de passe"
            }))
        }
    };

    let trimmed_username = body.username.trim().to_string();

    let result = sqlx::query_as::<_, (uuid::Uuid, String, String, Option<String>, Option<String>)>(
        r#"
        INSERT INTO users (email, username, password_hash, display_name, profile_image_url, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
        RETURNING id, email, username, display_name, profile_image_url AS avatar_url
        "#,
    )
    .bind(&body.email)
    .bind(&trimmed_username)
    .bind(&password_hash)
    .bind(&trimmed_username)    // display_name = username
    .bind(&generate_avatar_url(&trimmed_username))
    .fetch_one(pool.get_ref())
    .await;

    match result {
        Ok((id, email, username, display_name, avatar_url)) => {
            let token = match create_token(id, &email) {
                Ok(t) => t,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Erreur lors de la creation du token"
                    }))
                }
            };

            HttpResponse::Created().json(AuthResponse {
                token,
                user: UserPublic {
                    id,
                    email,
                    username,
                    display_name,
                    avatar_url,
                },
            })
        }
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("users_email_key") => {
            HttpResponse::Conflict().json(serde_json::json!({
                "error": "Un compte avec cet email existe deja"
            }))
        }
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("users_username_key") => {
            HttpResponse::Conflict().json(serde_json::json!({
                "error": "Ce nom d'utilisateur est deja pris"
            }))
        }
        Err(e) => {
            tracing::error!("Erreur inscription: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur interne du serveur"
            }))
        }
    }
}

// ── POST /api/auth/login ──────────────────────────────────────────────────────

async fn login(pool: web::Data<PgPool>, body: web::Json<LoginRequest>) -> HttpResponse {
    let result = sqlx::query_as::<_, (uuid::Uuid, String, String, String, Option<String>, Option<String>)>(
        "SELECT id, email, username, password_hash, display_name, profile_image_url AS avatar_url FROM users WHERE email = $1 AND is_active = true",
    )
    .bind(&body.email)
    .fetch_optional(pool.get_ref())
    .await;

    match result {
        Ok(Some((id, email, username, password_hash, display_name, avatar_url))) => {
            match verify_password(&body.password, &password_hash) {
                Ok(true) => {
                    let token = match create_token(id, &email) {
                        Ok(t) => t,
                        Err(_) => {
                            return HttpResponse::InternalServerError().json(serde_json::json!({
                                "error": "Erreur lors de la creation du token"
                            }))
                        }
                    };

                    HttpResponse::Ok().json(AuthResponse {
                        token,
                        user: UserPublic {
                            id,
                            email,
                            username,
                            display_name,
                            avatar_url,
                        },
                    })
                }
                _ => HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Email ou mot de passe incorrect"
                })),
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Email ou mot de passe incorrect"
        })),
        Err(e) => {
            tracing::error!("Erreur login: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur interne du serveur"
            }))
        }
    }
}

// ── GET /api/auth/me ──────────────────────────────────────────────────────────

async fn me(pool: web::Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let auth_header = match req.headers().get("Authorization") {
        Some(h) => match h.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                return HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Token invalide"
                }))
            }
        },
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Token manquant"
            }))
        }
    };

    let token = match extract_token_from_header(&auth_header) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Format de token invalide"
            }))
        }
    };

    let claims = match validate_token(token) {
        Ok(c) => c,
        Err(_) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Token expire ou invalide"
            }))
        }
    };

    let result = sqlx::query_as::<_, (uuid::Uuid, String, String, Option<String>, Option<String>)>(
        "SELECT id, email, username, display_name, profile_image_url AS avatar_url FROM users WHERE id = $1 AND is_active = true",
    )
    .bind(claims.sub)
    .fetch_optional(pool.get_ref())
    .await;

    match result {
        Ok(Some((id, email, username, display_name, avatar_url))) => {
            HttpResponse::Ok().json(UserPublic {
                id,
                email,
                username,
                display_name,
                avatar_url,
            })
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Utilisateur non trouve"
        })),
        Err(e) => {
            tracing::error!("Erreur me: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur interne du serveur"
            }))
        }
    }
}
