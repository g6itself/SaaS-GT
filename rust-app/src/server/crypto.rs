use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// Dérive une clé AES-256 (32 octets) depuis la variable d'environnement ENCRYPTION_KEY.
/// Retourne une erreur si la variable est absente ou trop courte.
/// Accepte 64 caractères hex (= 32 octets) ou une chaîne UTF-8 d'au moins 32 octets.
fn encryption_key() -> Result<[u8; 32], String> {
    let raw = std::env::var("ENCRYPTION_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or("ENCRYPTION_KEY non définie — définir cette variable d'environnement")?;

    // Format hex (64 chars → 32 octets)
    let bytes: Vec<u8> = if raw.len() == 64 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        (0..64)
            .step_by(2)
            .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap_or(0))
            .collect()
    } else {
        raw.into_bytes()
    };

    if bytes.len() < 32 {
        return Err(format!(
            "ENCRYPTION_KEY trop courte ({} octets) — minimum 32 octets ou 64 caractères hex",
            bytes.len()
        ));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes[..32]);
    Ok(key)
}

/// Chiffre une valeur avec AES-256-GCM.
/// Retourne une chaîne base64(nonce || ciphertext_avec_tag).
pub fn encrypt(plaintext: &str) -> Result<String, String> {
    let key_bytes = encryption_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 12 octets

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("Erreur chiffrement: {}", e))?;

    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(B64.encode(&combined))
}

/// Déchiffre une valeur chiffrée par `encrypt`.
pub fn decrypt(encoded: &str) -> Result<String, String> {
    let combined = B64
        .decode(encoded)
        .map_err(|e| format!("Erreur décodage base64: {}", e))?;

    if combined.len() < 12 {
        return Err("Données chiffrées invalides (trop courtes)".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key_bytes = encryption_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Erreur déchiffrement: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("Erreur décodage UTF-8: {}", e))
}
