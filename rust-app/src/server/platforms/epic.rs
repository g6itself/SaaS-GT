use sqlx::PgPool;
use uuid::Uuid;

use super::SyncStats;

/// Synchronise les achievements Epic Games d'un utilisateur.
///
/// NOTE: Depuis janvier 2025, les donnees d'achievements Epic ne sont plus
/// accessibles publiquement via l'API. L'integration necessite :
/// - Un compte Epic Games Developer
/// - L'enregistrement de l'application sur le Developer Portal
/// - L'utilisation du SDK EOS (Epic Online Services)
///
/// Cette integration est fournie en tant que stub.
pub async fn sync_epic_achievements(
    _pool: &PgPool,
    _user_id: Uuid,
    _epic_account_id: &str,
    _access_token: &str,
) -> Result<SyncStats, Box<dyn std::error::Error>> {
    // TODO: Implementer l'integration Epic Games via EOS SDK
    // Documentation :
    // - https://dev.epicgames.com/docs/epic-games-store/services/epic-achievements/overview
    // - https://dev.epicgames.com/docs/en-US/api-ref/interfaces/achievements
    //
    // Limitations connues :
    // - Acces restreint aux donnees de progression depuis janvier 2025
    // - Necessite le SDK EOS (C++ natif, FFI requis)
    // - Inscription obligatoire au Developer Portal

    Err("Integration Epic Games pas encore implementee. Acces API restreint.".into())
}
