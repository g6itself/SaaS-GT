// ── Extracteur JWT centralisé pour actix-web ─────────────────────────────────
//
// Usage dans un handler :
//   async fn my_handler(auth: AuthUser, pool: web::Data<PgPool>) -> HttpResponse { ... }
//   let user_id: uuid::Uuid = auth.user_id;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures_util::future::{ready, Ready};

use crate::server::auth::{extract_token_from_header, validate_token, Claims};

/// Extracteur injecté dans les handlers actix-web.
/// Rejette la requête avec 401 si le token est absent, malformé ou expiré.
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub claims: Claims,
}

impl FromRequest for AuthUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = extract_auth(req);
        ready(result)
    }
}

fn extract_auth(req: &HttpRequest) -> Result<AuthUser, actix_web::Error> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            actix_web::error::ErrorUnauthorized(
                serde_json::json!({"error": "Token manquant"}).to_string(),
            )
        })?;

    let token = extract_token_from_header(auth_header).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized(
            serde_json::json!({"error": "Format de token invalide"}).to_string(),
        )
    })?;

    let claims = validate_token(token).map_err(|_| {
        actix_web::error::ErrorUnauthorized(
            serde_json::json!({"error": "Token expire ou invalide"}).to_string(),
        )
    })?;

    Ok(AuthUser {
        user_id: claims.sub,
        claims,
    })
}
