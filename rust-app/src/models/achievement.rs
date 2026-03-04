use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Achievement {
    pub id: Uuid,
    pub game_platform_id: Uuid,
    pub platform_achievement_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_hidden: bool,
    pub global_unlock_pct: Option<f32>,
}
