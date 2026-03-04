use sqlx::PgPool;
use uuid::Uuid;

use super::SyncStats;

/// Synchronise les achievements GOG d'un utilisateur.
///
/// NOTE: L'API GOG est non officielle et peut changer sans preavis.
/// Cette integration est marquee comme experimentale.
pub async fn sync_gog_achievements(
    _pool: &PgPool,
    _user_id: Uuid,
    _gog_user_id: &str,
    _access_token: &str,
) -> Result<SyncStats, Box<dyn std::error::Error>> {
    // TODO: Implementer l'integration GOG
    // Endpoints connus (reverse-engineered) :
    // - GET https://embed.gog.com/account/getFilteredProducts?mediaType=1 (liste des jeux)
    // - GET https://gameplay.gog.com/clients/{product_id}/users/{user_id}/achievements
    //
    // L'authentification necessite un token obtenu via le client GOG Galaxy.
    // Le flow OAuth GOG necessite :
    // 1. Client ID / Client Secret (obtenu via le GOG Developer Portal)
    // 2. Authorization code flow
    // 3. Token refresh

    Err("Integration GOG pas encore implementee. Cette fonctionnalite est experimentale.".into())
}
