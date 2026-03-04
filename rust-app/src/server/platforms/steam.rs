use sqlx::PgPool;
use uuid::Uuid;

use super::SyncStats;

const STEAM_API_BASE: &str = "https://api.steampowered.com";

fn get_steam_api_key() -> String {
    std::env::var("STEAM_API_KEY").expect("STEAM_API_KEY doit etre definie")
}

/// Recupere la liste des jeux possedes par un utilisateur Steam
async fn fetch_owned_games(
    steam_id: &str,
) -> Result<Vec<SteamOwnedGame>, Box<dyn std::error::Error>> {
    let api_key = get_steam_api_key();
    let url = format!(
        "{}/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=1&format=json",
        STEAM_API_BASE, api_key, steam_id
    );

    let client = reqwest::Client::new();
    let resp: SteamOwnedGamesResponse = client.get(&url).send().await?.json().await?;

    Ok(resp.response.games.unwrap_or_default())
}

/// Recupere le schema des achievements d'un jeu
async fn fetch_game_schema(
    app_id: u64,
) -> Result<Vec<SteamAchievementSchema>, Box<dyn std::error::Error>> {
    let api_key = get_steam_api_key();
    let url = format!(
        "{}/ISteamUserStats/GetSchemaForGame/v2/?key={}&appid={}&l=french",
        STEAM_API_BASE, api_key, app_id
    );

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let achievements = resp["game"]["availableGameStats"]["achievements"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(SteamAchievementSchema {
                        name: a["name"].as_str()?.to_string(),
                        display_name: a["displayName"].as_str().unwrap_or("").to_string(),
                        description: a["description"].as_str().map(String::from),
                        icon: a["icon"].as_str().map(String::from),
                        hidden: a["hidden"].as_i64().unwrap_or(0) == 1,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(achievements)
}

/// Recupere les achievements debloques par un joueur pour un jeu
async fn fetch_player_achievements(
    steam_id: &str,
    app_id: u64,
) -> Result<Vec<SteamPlayerAchievement>, Box<dyn std::error::Error>> {
    let api_key = get_steam_api_key();
    let url = format!(
        "{}/ISteamUserStats/GetPlayerAchievements/v1/?key={}&steamid={}&appid={}",
        STEAM_API_BASE, api_key, steam_id, app_id
    );

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    if !resp["playerstats"]["success"].as_bool().unwrap_or(false) {
        return Ok(vec![]);
    }

    let achievements = resp["playerstats"]["achievements"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(SteamPlayerAchievement {
                        api_name: a["apiname"].as_str()?.to_string(),
                        achieved: a["achieved"].as_i64().unwrap_or(0) == 1,
                        unlock_time: a["unlocktime"].as_i64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(achievements)
}

/// Synchronise tous les achievements Steam d'un utilisateur
pub async fn sync_steam_achievements(
    pool: &PgPool,
    user_id: Uuid,
    steam_id: &str,
) -> Result<SyncStats, Box<dyn std::error::Error>> {
    let mut games_synced: u32 = 0;
    let mut achievements_synced: u32 = 0;

    // 1. Recuperer la liste des jeux
    let owned_games = fetch_owned_games(steam_id).await?;

    for game in &owned_games {
        // 2. Upsert le jeu dans la BDD
        let normalized_title = game.name.to_lowercase();
        let game_row = sqlx::query_as::<_, (Uuid,)>(
            r#"
            INSERT INTO games (title, normalized_title)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            RETURNING id
            "#,
        )
        .bind(&game.name)
        .bind(&normalized_title)
        .fetch_optional(pool)
        .await?;

        let game_id = if let Some((id,)) = game_row {
            id
        } else {
            // Le jeu existe deja, le recuperer
            match sqlx::query_as::<_, (Uuid,)>(
                "SELECT id FROM games WHERE normalized_title = $1",
            )
            .bind(&normalized_title)
            .fetch_optional(pool)
            .await?
            {
                Some((id,)) => id,
                None => continue,
            }
        };

        // 3. Upsert game_platform_id
        let gpi_row = sqlx::query_as::<_, (Uuid,)>(
            r#"
            INSERT INTO game_platform_ids (game_id, platform, platform_game_id, platform_name)
            VALUES ($1, 'steam'::platform_type, $2, $3)
            ON CONFLICT (platform, platform_game_id) DO UPDATE SET platform_name = $3
            RETURNING id
            "#,
        )
        .bind(game_id)
        .bind(game.appid.to_string())
        .bind(&game.name)
        .fetch_one(pool)
        .await?;

        let game_platform_id = gpi_row.0;

        // 4. Recuperer le schema des achievements
        let schema = match fetch_game_schema(game.appid).await {
            Ok(s) => s,
            Err(_) => continue, // Certains jeux n'ont pas d'achievements
        };

        if schema.is_empty() {
            continue;
        }

        // 5. Upsert les achievements
        for ach in &schema {
            sqlx::query(
                r#"
                INSERT INTO achievements (game_platform_id, platform_achievement_id, name, description, icon_url, is_hidden)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (game_platform_id, platform_achievement_id) DO UPDATE
                SET name = $3, description = $4, icon_url = $5, is_hidden = $6
                "#,
            )
            .bind(game_platform_id)
            .bind(&ach.name)
            .bind(&ach.display_name)
            .bind(&ach.description)
            .bind(&ach.icon)
            .bind(ach.hidden)
            .execute(pool)
            .await?;
        }

        // Mettre a jour total_achievements
        sqlx::query(
            "UPDATE game_platform_ids SET total_achievements = $1 WHERE id = $2",
        )
        .bind(schema.len() as i32)
        .bind(game_platform_id)
        .execute(pool)
        .await?;

        // 6. Recuperer les achievements du joueur
        let player_achievements = match fetch_player_achievements(steam_id, game.appid).await {
            Ok(pa) => pa,
            Err(_) => continue,
        };

        for pa in &player_achievements {
            // Trouver l'achievement correspondant
            let ach_id = sqlx::query_as::<_, (Uuid,)>(
                "SELECT id FROM achievements WHERE game_platform_id = $1 AND platform_achievement_id = $2",
            )
            .bind(game_platform_id)
            .bind(&pa.api_name)
            .fetch_optional(pool)
            .await?;

            if let Some((achievement_id,)) = ach_id {
                let unlocked_at = if pa.achieved && pa.unlock_time > 0 {
                    chrono::DateTime::from_timestamp(pa.unlock_time, 0)
                } else {
                    None
                };

                sqlx::query(
                    r#"
                    INSERT INTO user_achievements (user_id, achievement_id, is_unlocked, unlocked_at)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (user_id, achievement_id) DO UPDATE
                    SET is_unlocked = $3, unlocked_at = $4, synced_at = NOW()
                    "#,
                )
                .bind(user_id)
                .bind(achievement_id)
                .bind(pa.achieved)
                .bind(unlocked_at)
                .execute(pool)
                .await?;

                achievements_synced += 1;
            }
        }

        games_synced += 1;
    }

    Ok(SyncStats {
        games_synced,
        achievements_synced,
    })
}

// ─── Types Steam API ────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SteamOwnedGamesResponse {
    response: SteamOwnedGamesInner,
}

#[derive(serde::Deserialize)]
struct SteamOwnedGamesInner {
    games: Option<Vec<SteamOwnedGame>>,
}

#[derive(serde::Deserialize)]
struct SteamOwnedGame {
    appid: u64,
    name: String,
}

struct SteamAchievementSchema {
    name: String,
    display_name: String,
    description: Option<String>,
    icon: Option<String>,
    hidden: bool,
}

struct SteamPlayerAchievement {
    api_name: String,
    achieved: bool,
    unlock_time: i64,
}
