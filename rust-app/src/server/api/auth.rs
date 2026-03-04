use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::models::user::{AuthResponse, LoginRequest, RegisterRequest, UserPublic};
use crate::server::auth::{
    create_token, extract_token_from_header, hash_password, validate_token, verify_password,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/me", web::get().to(me)),
    );
}

async fn register(
    pool: web::Data<PgPool>,
    body: web::Json<RegisterRequest>,
) -> HttpResponse {
    let password_hash = match hash_password(&body.password) {
        Ok(hash) => hash,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Erreur lors du hashage du mot de passe"
        })),
    };

    let result = sqlx::query_as::<_, (uuid::Uuid, String, String, Option<String>, Option<String>)>(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, email, username, display_name, avatar_url
        "#,
    )
    .bind(&body.email)
    .bind(&body.username)
    .bind(&password_hash)
    .fetch_one(pool.get_ref())
    .await;

    match result {
        Ok((id, email, username, display_name, avatar_url)) => {
            let token = match create_token(id, &email) {
                Ok(t) => t,
                Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Erreur lors de la creation du token"
                })),
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
        Err(e) => {
            tracing::error!("Erreur inscription: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Erreur interne du serveur"
            }))
        }
    }
}

async fn login(
    pool: web::Data<PgPool>,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    let result = sqlx::query_as::<_, (uuid::Uuid, String, String, String, Option<String>, Option<String>)>(
        "SELECT id, email, username, password_hash, display_name, avatar_url FROM users WHERE email = $1 AND is_active = true",
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
                        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Erreur lors de la creation du token"
                        })),
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

async fn me(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> HttpResponse {
    let auth_header = match req.headers().get("Authorization") {
        Some(h) => match h.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Token invalide"
            })),
        },
        None => return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Token manquant"
        })),
    };

    let token = match extract_token_from_header(&auth_header) {
        Some(t) => t,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Format de token invalide"
        })),
    };

    let claims = match validate_token(token) {
        Ok(c) => c,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Token expire ou invalide"
        })),
    };

    let result = sqlx::query_as::<_, (uuid::Uuid, String, String, Option<String>, Option<String>)>(
        "SELECT id, email, username, display_name, avatar_url FROM users WHERE id = $1 AND is_active = true",
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
