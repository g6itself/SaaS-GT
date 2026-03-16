use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Game {
    pub id: Uuid,
    pub title: String,
    pub normalized_title: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct GamePlatformId {
    pub id: Uuid,
    pub game_id: Uuid,
    pub platform: String,
    pub platform_game_id: String,
    pub platform_name: Option<String>,
    pub total_achievements: i32,
}

/// Vue enrichie d'un jeu avec les stats d'achievements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWithStats {
    pub id: Uuid,
    pub title: String,
    pub platforms: Vec<String>,
    pub total_achievements: i32,
    pub unlocked_achievements: i32,
}
