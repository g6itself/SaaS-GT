use sqlx::PgPool;
use uuid::Uuid;

use super::SyncStats;

const STEAM_API_BASE: &str = "https://api.steampowered.com";

/// Résout la clé API à utiliser : clé personnelle de l'utilisateur en priorité,
/// sinon la variable d'environnement du serveur.
fn resolve_api_key(user_key: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(key) = user_key {
        let key = key.trim();
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }
    std::env::var("STEAM_API_KEY")
        .map_err(|_| "Aucune clé API Steam configurée (ni personnelle ni serveur)".into())
}

// ─── Helpers publics ────────────────────────────────────────────────────────

/// Encode un caractère pour l'utiliser dans une valeur de paramètre d'URL.
fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// Construit l'URL de redirection Steam OpenID 2.0.
pub fn steam_openid_url(return_to: &str, realm: &str) -> String {
    format!(
        "https://steamcommunity.com/openid/login\
         ?openid.ns={ns}\
         &openid.claimed_id={ci}\
         &openid.identity={id}\
         &openid.mode=checkid_setup\
         &openid.return_to={rt}\
         &openid.realm={rl}",
        ns = percent_encode("http://specs.openid.net/auth/2.0"),
        ci = percent_encode("http://specs.openid.net/auth/2.0/identifier_select"),
        id = percent_encode("http://specs.openid.net/auth/2.0/identifier_select"),
        rt = percent_encode(return_to),
        rl = percent_encode(realm),
    )
}

/// Vérifie la réponse OpenID auprès de Steam et retourne le SteamID64.
pub async fn verify_steam_openid(
    params: &std::collections::HashMap<String, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Remplacer openid.mode par check_authentication
    let mut verify_params = params.clone();
    verify_params.insert(
        "openid.mode".to_string(),
        "check_authentication".to_string(),
    );

    let client = reqwest::Client::new();
    let resp = client
        .post("https://steamcommunity.com/openid/login")
        .form(&verify_params)
        .send()
        .await?
        .text()
        .await?;

    if !resp.contains("is_valid:true") {
        return Err("Vérification Steam OpenID échouée".into());
    }

    // Extraire le SteamID64 depuis claimed_id
    // Format : https://steamcommunity.com/openid/id/76561198XXXXXXXXX
    let claimed_id = params
        .get("openid.claimed_id")
        .ok_or("claimed_id manquant dans la réponse Steam")?;

    let steam_id = claimed_id
        .rsplit('/')
        .next()
        .ok_or("Format de claimed_id invalide")?
        .to_string();

    // Valider que c'est bien un entier 64-bit
    steam_id
        .parse::<u64>()
        .map_err(|_| "SteamID invalide (pas un entier 64-bit)")?;

    Ok(steam_id)
}

/// Récupère le nom d'affichage Steam (personaname) via l'API.
/// Utilise la clé API du serveur. Retourne l'ID si la clé est absente ou si l'appel échoue.
pub async fn fetch_steam_player_summary(steam_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let api_key = std::env::var("STEAM_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Ok(steam_id.to_string());
    }
    let url = format!(
        "{}/ISteamUser/GetPlayerSummaries/v2/?key={}&steamids={}",
        STEAM_API_BASE, api_key, steam_id
    );
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    let name = resp["response"]["players"][0]["personaname"]
        .as_str()
        .unwrap_or(steam_id)
        .to_string();
    Ok(name)
}

/// Recupere la liste des jeux possedes par un utilisateur Steam
async fn fetch_owned_games(
    steam_id: &str,
    api_key: &str,
) -> Result<Vec<SteamOwnedGame>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=1&format=json",
        STEAM_API_BASE, api_key, steam_id
    );

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let games = resp["response"]["games"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| {
                    let appid = g["appid"].as_u64()?;
                    let name = g["name"].as_str().unwrap_or("").to_string();
                    if name.is_empty() { return None; }
                    Some(SteamOwnedGame { appid, name })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(games)
}

/// Recupere le schema des achievements d'un jeu
async fn fetch_game_schema(
    app_id: u64,
    api_key: &str,
) -> Result<Vec<SteamAchievementSchema>, Box<dyn std::error::Error>> {
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
    api_key: &str,
) -> Result<Vec<SteamPlayerAchievement>, Box<dyn std::error::Error>> {
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

/// Synchronise tous les achievements Steam d'un utilisateur.
/// `user_api_key` : clé API personnelle du joueur (prioritaire sur la variable serveur).
pub async fn sync_steam_achievements(
    pool: &PgPool,
    user_id: Uuid,
    steam_id: &str,
    user_api_key: Option<&str>,
) -> Result<SyncStats, Box<dyn std::error::Error>> {
    let api_key = resolve_api_key(user_api_key)?;

    let mut games_synced: u32 = 0;
    let mut achievements_synced: u32 = 0;

    // 1. Recuperer la liste des jeux
    let owned_games = fetch_owned_games(steam_id, &api_key).await?;

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
        let schema = match fetch_game_schema(game.appid, &api_key).await {
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
        let player_achievements = match fetch_player_achievements(steam_id, game.appid, &api_key).await {
            Ok(pa) => pa,
            Err(_) => continue,
        };

        let mut unlocked_count: i32 = 0;

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

                if pa.achieved {
                    unlocked_count += 1;
                }
                achievements_synced += 1;
            }
        }

        // 7. Upsert user_game_stats pour ce jeu
        let total = schema.len() as i32;
        let pct = if total > 0 {
            (unlocked_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        sqlx::query(
            r#"
            INSERT INTO user_game_stats
                (user_id, game_id, achievements_unlocked, achievements_total, completion_pct, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (user_id, game_id) DO UPDATE
            SET achievements_unlocked = $3,
                achievements_total     = $4,
                completion_pct         = $5,
                updated_at             = NOW()
            "#,
        )
        .bind(user_id)
        .bind(game_id)
        .bind(unlocked_count)
        .bind(total)
        .bind(pct)
        .execute(pool)
        .await?;

        games_synced += 1;
    }

    // 8. Recalcul bulk exact sur users (plus fiable que les triggers incrémentaux)
    let final_stats = sqlx::query_as::<_, (i64, i64)>(
        r#"
        UPDATE users
        SET
            total_achievements_count = (
                SELECT COUNT(*)
                FROM user_achievements
                WHERE user_id = $1 AND is_unlocked = true
            ),
            games_completed = (
                SELECT COUNT(*)
                FROM user_game_stats
                WHERE user_id = $1
                  AND achievements_total > 0
                  AND achievements_unlocked >= achievements_total
            ),
            total_possible_achievements = (
                SELECT COALESCE(SUM(achievements_total), 0)
                FROM user_game_stats
                WHERE user_id = $1
            )
        WHERE id = $1
        RETURNING total_achievements_count, games_completed::BIGINT
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or((0, 0));

    Ok(SyncStats {
        games_synced,
        achievements_synced,
        total_achievements: final_stats.0 as u32,
        games_completed: final_stats.1 as u32,
    })
}

// ─── Types Steam API ────────────────────────────────────────────────────────

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
