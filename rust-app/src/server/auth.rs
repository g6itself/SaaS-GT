use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Algorithm, Version
};
use jsonwebtoken::{decode, encode, Algorithm as JwtAlgorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub exp: usize,
}

/// Hash un mot de passe avec Argon2
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    
    // Paramètres extrêmement rapides en dev (128 KiB) pour éviter les timeouts
    let params = if cfg!(debug_assertions) {
        Params::new(128, 1, 1, None).unwrap()
    } else {
        Params::default()
    };
    
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verifie un mot de passe contre son hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Retourne le secret JWT — panique en production si non défini
fn jwt_secret() -> String {
    match std::env::var("JWT_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            if cfg!(debug_assertions) {
                tracing::warn!("JWT_SECRET non defini — utilisation du secret de dev (dangereux)");
                "dev-secret-change-me".into()
            } else {
                panic!("JWT_SECRET doit etre defini en production")
            }
        }
    }
}

/// Genere un JWT pour un utilisateur
pub fn create_token(user_id: Uuid, email: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = jwt_secret();
    let expiration_hours: u64 = std::env::var("JWT_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".into())
        .parse()
        .unwrap_or(24);

    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(expiration_hours as i64))
        .expect("timestamp valide")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        exp: expiration,
    };

    encode(
        &Header::new(JwtAlgorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Valide un JWT et retourne les claims
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = jwt_secret();
    let mut validation = Validation::new(JwtAlgorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Extrait le token JWT du header Authorization
pub fn extract_token_from_header(auth_header: &str) -> Option<&str> {
    if auth_header.starts_with("Bearer ") {
        Some(&auth_header[7..])
    } else {
        None
    }
}
