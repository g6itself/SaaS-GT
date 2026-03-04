use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct UserAchievement {
    pub id: Uuid,
    pub user_id: Uuid,
    pub achievement_id: Uuid,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub is_unlocked: bool,
    pub synced_at: DateTime<Utc>,
}

/// Vue enrichie pour l'affichage frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAchievementDetail {
    pub achievement_name: String,
    pub achievement_description: Option<String>,
    pub achievement_icon_url: Option<String>,
    pub game_title: String,
    pub platform: String,
    pub is_unlocked: bool,
    pub unlocked_at: Option<DateTime<Utc>>,
}
