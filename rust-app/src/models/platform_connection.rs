use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct PlatformConnection {
    pub id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub platform_user_id: String,
    pub platform_username: Option<String>,
    #[serde(skip_serializing)]
    pub access_token: Option<String>,
    #[serde(skip_serializing)]
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ConnectPlatformRequest {
    pub platform_user_id: String,
    pub platform_username: Option<String>,
    pub access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApikeyRequest {
    pub access_token: String,
}

/// Vue publique d'une connexion plateforme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConnectionPublic {
    pub id: Uuid,
    pub platform: String,
    pub platform_username: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub connected: bool,
}

/// Stats d'achievements par plateforme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformStats {
    pub platform: String,
    pub total_achievements: i32,
    pub unlocked_achievements: i32,
    pub total_games: i32,
}
