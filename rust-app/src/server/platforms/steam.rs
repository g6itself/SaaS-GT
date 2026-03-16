use futures_util::future::join_all;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::SyncStats;

const STEAM_API_BASE: &str = "https://api.steampowered.com";

// ─── Helper interne : achievements d'un seul jeu (pour parallélisation) ─────

/// Récupère et construit les achievements débloqués d'un seul jeu.
/// Fait 3 appels en parallèle : GetPlayerAchievements + GetSchemaForGame + GetGlobalAchievementPercentages.
async fn fetch_game_achievements_parallel(
    client: &reqwest::Client,
    steam_id: &str,
    api_key: &str,
    appid: u64,
    game_name: &str,
) -> Vec<SteamRecentAchievement> {
    let ach_url = format!(
        "{}/ISteamUserStats/GetPlayerAchievements/v1/?key={}&steamid={}&appid={}&l=french",
        STEAM_API_BASE, api_key, steam_id, appid
    );
    let schema_url = format!(
        "{}/ISteamUserStats/GetSchemaForGame/v2/?key={}&appid={}&l=french",
        STEAM_API_BASE, api_key, appid
    );
    let pct_url = format!(
        "{}/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={}",
        STEAM_API_BASE, appid
    );

    // Les 3 appels en parallèle
    let (ach_res, schema_res, pct_res) = tokio::join!(
        client.get(&ach_url).send(),
        client.get(&schema_url).send(),
        client.get(&pct_url).send(),
    );

    let ach_json: serde_json::Value = match ach_res {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => return vec![],
    };
    if !ach_json["playerstats"]["success"].as_bool().unwrap_or(false) {
        return vec![];
    }

    let schema_json: serde_json::Value = match schema_res {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => serde_json::Value::Null,
    };
    let pct_json: serde_json::Value = match pct_res {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => serde_json::Value::Null,
    };

    let schema_map: std::collections::HashMap<String, serde_json::Value> = schema_json
        ["game"]["availableGameStats"]["achievements"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a["name"].as_str().map(|n| (n.to_string(), a.clone())))
                .collect()
        })
        .unwrap_or_default();

    let pct_map: std::collections::HashMap<String, f32> = pct_json
        ["achievementpercentages"]["achievements"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let name = a["name"].as_str()?.to_string();
                    let pct = a["percent"].as_f64()? as f32;
                    Some((name, pct))
                })
                .collect()
        })
        .unwrap_or_default();

    let achievements = ach_json["playerstats"]["achievements"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut result = Vec::new();
    for a in &achievements {
        if a["achieved"].as_i64().unwrap_or(0) != 1 {
            continue;
        }
        let api_name = match a["apiname"].as_str() {
            Some(n) => n.to_string(),
            None => continue,
        };
        let unlock_time = a["unlocktime"].as_i64().unwrap_or(0);
        if unlock_time == 0 {
            continue;
        }

        let schema_entry = schema_map.get(&api_name);
        let display_name = schema_entry
            .and_then(|e| e["displayName"].as_str())
            .unwrap_or(&api_name)
            .to_string();
        let description = schema_entry
            .and_then(|e| e["description"].as_str())
            .map(String::from);
        let icon_url = schema_entry
            .and_then(|e| e["icon"].as_str())
            .map(String::from);

        let global_pct = pct_map.get(&api_name).copied();
        let rarity = compute_rarity(global_pct);
        let points = rarity_points(rarity);

        result.push(SteamRecentAchievement {
            name: display_name,
            description,
            icon_url,
            rarity: rarity.to_string(),
            points,
            game_title: game_name.to_string(),
            game_appid: appid,
            unlocked_at: chrono::DateTime::from_timestamp(unlock_time, 0),
        });
    }
    // Trier par date décroissante au niveau du jeu avant de remonter
    result.sort_by(|a, b| b.unlocked_at.cmp(&a.unlocked_at));
    result
}

/// Résout la clé API à utiliser : clé personnelle de l'utilisateur en priorité,
/// sinon la variable d'environnement du serveur.
pub fn resolve_api_key(user_key: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(key) = user_key {
        let key = key.trim();
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }
    std::env::var("STEAM_API_KEY")
        .map_err(|_| "Aucune clé API Steam configurée (ni personnelle ni serveur)".into())
}

/// Récupère le SteamID64 et la clé API déchiffrée d'un utilisateur.
pub async fn get_user_steam_credentials(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let steam_id = sqlx::query_scalar::<_, String>(
        "SELECT platform_user_id FROM platform_connections WHERE user_id = $1 AND platform = 'steam'::platform_type",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or("Compte Steam non connecté")?;

    let encrypted_key = sqlx::query_scalar::<_, Option<String>>(
        "SELECT steam_api_key_enc FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    let decrypted_key = encrypted_key.as_deref().and_then(|enc| {
        crate::server::crypto::decrypt(enc)
            .map_err(|e| tracing::warn!("Echec dechiffrement cle API: {}", e))
            .ok()
    });

    let api_key = resolve_api_key(decrypted_key.as_deref())?;
    Ok((steam_id, api_key))
}

// ─── Helpers publics ────────────────────────────────────────────────────────

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

    let claimed_id = params
        .get("openid.claimed_id")
        .ok_or("claimed_id manquant dans la réponse Steam")?;

    let steam_id = claimed_id
        .rsplit('/')
        .next()
        .ok_or("Format de claimed_id invalide")?
        .to_string();

    steam_id
        .parse::<u64>()
        .map_err(|_| "SteamID invalide (pas un entier 64-bit)")?;

    Ok(steam_id)
}

/// Récupère le nom d'affichage Steam (personaname) via l'API.
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

/// Récupère la liste des jeux possédés par un utilisateur Steam.
pub async fn fetch_owned_games(
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
                    let playtime_minutes = g["playtime_forever"].as_u64().unwrap_or(0) as i32;
                    let img_icon_url = g["img_icon_url"].as_str()
                        .filter(|s| !s.is_empty())
                        .map(|hash| format!(
                            "https://media.steampowered.com/steamcommunity/public/images/apps/{}/{}.jpg",
                            appid, hash
                        ));
                    let last_played_at = g["rtime_last_played"].as_i64()
                        .filter(|&t| t > 0)
                        .and_then(|t| chrono::DateTime::from_timestamp(t, 0));
                    Some(SteamOwnedGame { appid, name, playtime_minutes, img_icon_url, last_played_at })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(games)
}

/// Récupère les succès récemment débloqués (5 derniers) depuis les jeux récemment joués.
/// Les appels par jeu sont parallélisés (3 appels simultanés par jeu, 5 jeux en parallèle).
pub async fn fetch_recent_achievements(
    steam_id: &str,
    api_key: &str,
) -> Result<Vec<SteamRecentAchievement>, Box<dyn std::error::Error>> {
    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()?,
    );

    // Récupérer tous les jeux joués, triés par date de dernière session décroissante.
    // On utilise GetOwnedGames (plus complet que GetRecentlyPlayedGames limité à 2 semaines).
    let owned_url = format!(
        "{}/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=1&format=json",
        STEAM_API_BASE, api_key, steam_id
    );
    let owned_resp: serde_json::Value = client.get(&owned_url).send().await?.json().await?;
    let mut all_played: Vec<serde_json::Value> = owned_resp["response"]["games"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|g| g["rtime_last_played"].as_i64().unwrap_or(0) > 0)
        .collect();
    // Trier par date de dernière session décroissante
    all_played.sort_by(|a, b| {
        let ta = a["rtime_last_played"].as_i64().unwrap_or(0);
        let tb = b["rtime_last_played"].as_i64().unwrap_or(0);
        tb.cmp(&ta)
    });
    // Inspecter les 20 jeux les plus récemment joués pour couvrir une fenêtre large
    let recent_games: Vec<serde_json::Value> = all_played.into_iter().take(20).collect();

    // Lancer les 20 jeux en parallèle (chacun fait 3 appels Steam en parallèle via tokio::join!)
    let tasks: Vec<_> = recent_games
        .iter()
        .filter_map(|game| {
            let appid = game["appid"].as_u64()?;
            let game_name = game["name"].as_str().unwrap_or("").to_string();
            let client = Arc::clone(&client);
            let steam_id = steam_id.to_string();
            let api_key = api_key.to_string();
            Some(tokio::spawn(async move {
                fetch_game_achievements_parallel(&client, &steam_id, &api_key, appid, &game_name)
                    .await
            }))
        })
        .collect();

    let results = join_all(tasks).await;
    let mut all_achievements: Vec<SteamRecentAchievement> = results
        .into_iter()
        .filter_map(|r| r.ok())
        .flatten()
        .collect();

    // Trier par date décroissante, garder les 5 plus récents
    all_achievements.sort_by(|a, b| b.unlocked_at.cmp(&a.unlocked_at));
    all_achievements.truncate(5);

    Ok(all_achievements)
}

/// Calcule la rareté d'un achievement selon son pourcentage global de déblocage.
pub fn compute_rarity(pct: Option<f32>) -> &'static str {
    match pct {
        Some(p) if p < 5.0  => "mythic",
        Some(p) if p < 15.0 => "legendary",
        Some(p) if p < 40.0 => "epic",
        Some(p) if p < 70.0 => "rare",
        _                   => "common",
    }
}

/// Retourne les points associés à une rareté.
pub fn rarity_points(rarity: &str) -> i32 {
    match rarity {
        "mythic"    => 2000,
        "legendary" => 500,
        "epic"      => 100,
        "rare"      => 25,
        _           => 5,
    }
}

/// Synchronise les totaux Steam d'un utilisateur (stockage minimal).
/// Ne stocke que achievements_unlocked, achievements_total, completion_pct par jeu,
/// puis recalcule les totaux utilisateur.
pub async fn sync_steam_achievements(
    pool: &PgPool,
    user_id: Uuid,
    steam_id: &str,
    user_api_key: Option<&str>,
) -> Result<SyncStats, Box<dyn std::error::Error>> {
    let api_key = resolve_api_key(user_api_key)?;

    let mut games_synced: u32 = 0;
    let mut achievements_synced: u32 = 0;

    // Client HTTP partagé pour toute la boucle de sync (avec timeout)
    let sync_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // 1. Récupérer la liste des jeux
    let owned_games = fetch_owned_games(steam_id, &api_key).await?;

    for game in &owned_games {
        // 2. Upsert le jeu dans la BDD (clé de jointure uniquement)
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

        // 3. Upsert game_platform_ids
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

        let _game_platform_id = gpi_row.0;

        // 4. Récupérer le nombre d'achievements du joueur pour ce jeu (sans stocker les métadonnées)
        let player_ach_url = format!(
            "{}/ISteamUserStats/GetPlayerAchievements/v1/?key={}&steamid={}&appid={}",
            STEAM_API_BASE, api_key, steam_id, game.appid
        );
        let ach_resp: serde_json::Value = match sync_client.get(&player_ach_url).send().await?.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        if !ach_resp["playerstats"]["success"].as_bool().unwrap_or(false) {
            // Jeu sans achievements — upsert avec 0/0
            sqlx::query(
                r#"
                INSERT INTO user_game_stats
                    (user_id, game_id, achievements_unlocked, achievements_total, completion_pct, updated_at)
                VALUES ($1, $2, 0, 0, 0.0, NOW())
                ON CONFLICT (user_id, game_id) DO UPDATE
                SET updated_at = NOW()
                "#,
            )
            .bind(user_id)
            .bind(game_id)
            .execute(pool)
            .await?;
            games_synced += 1;
            continue;
        }

        let ach_list = ach_resp["playerstats"]["achievements"].as_array();
        let (unlocked_count, total_count) = match ach_list {
            Some(arr) => {
                let total = arr.len() as i32;
                let unlocked = arr.iter()
                    .filter(|a| a["achieved"].as_i64().unwrap_or(0) == 1)
                    .count() as i32;
                (unlocked, total)
            }
            None => (0, 0),
        };

        let pct = if total_count > 0 {
            (unlocked_count as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        // 5. Upsert user_game_stats (totaux uniquement)
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
        .bind(total_count)
        .bind(pct)
        .execute(pool)
        .await?;

        achievements_synced += unlocked_count as u32;
        games_synced += 1;
    }

    // 6. Recalcul bulk des totaux utilisateur (achievements, jeux, points)
    // Formule points : 10 pts par achievement débloqué + 50 pts par jeu complété à 100%
    let final_stats = sqlx::query_as::<_, (i64, i64)>(
        r#"
        UPDATE users
        SET
            total_achievements_count = (
                SELECT COALESCE(SUM(achievements_unlocked), 0)
                FROM user_game_stats
                WHERE user_id = $1
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
            ),
            total_points = (
                SELECT COALESCE(SUM(achievements_unlocked), 0) * 10
                     + COUNT(*) FILTER (WHERE achievements_total > 0 AND achievements_unlocked >= achievements_total) * 50
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

    // Mettre à jour le snapshot de rang si jamais initialisé ou si >24h
    sqlx::query(
        r#"
        UPDATE users u SET
            rank_snapshot    = ranked.rank_pts,
            rank_snapshot_at = NOW()
        FROM (
            SELECT id, RANK() OVER (ORDER BY total_points DESC)::BIGINT AS rank_pts
            FROM users
            WHERE is_active = true
        ) ranked
        WHERE u.id = $1
          AND ranked.id = $1
          AND (u.rank_snapshot_at IS NULL OR u.rank_snapshot_at < NOW() - INTERVAL '24 hours')
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .ok();

    Ok(SyncStats {
        games_synced,
        achievements_synced,
        total_achievements: final_stats.0 as u32,
        games_completed: final_stats.1 as u32,
    })
}

/// Récupère les 5 derniers jeux complétés à 100% depuis l'API Steam.
/// Interroge les 20 jeux les plus récemment joués en parallèle pour trouver les complétés.
pub async fn fetch_completed_games(
    steam_id: &str,
    api_key: &str,
) -> Result<Vec<SteamCompletedGame>, Box<dyn std::error::Error>> {
    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()?,
    );

    // Récupérer tous les jeux possédés triés par date de dernière session (les plus récents en premier)
    let url = format!(
        "{}/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=1&format=json",
        STEAM_API_BASE, api_key, steam_id
    );
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    let mut games: Vec<serde_json::Value> = resp["response"]["games"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|g| g["appid"].as_u64().is_some())
        .collect();

    // Trier par rtime_last_played décroissant pour trouver les complétés récents en premier
    games.sort_by(|a, b| {
        let ta = a["rtime_last_played"].as_i64().unwrap_or(0);
        let tb = b["rtime_last_played"].as_i64().unwrap_or(0);
        tb.cmp(&ta)
    });

    // Tous les jeux avec un nom valide, par batch de 30 en parallèle
    // On cherche dans tous les jeux (pas seulement les 20 récents) car un jeu 100%
    // peut avoir été joué il y a longtemps
    let all_candidates: Vec<_> = games
        .iter()
        .filter_map(|g| {
            let appid = g["appid"].as_u64()?;
            let name = g["name"].as_str().filter(|s| !s.is_empty())?.to_string();
            Some((appid, name))
        })
        .collect();

    let mut completed: Vec<SteamCompletedGame> = Vec::new();

    // Traiter par batches de 30 pour ne pas saturer Steam API
    for chunk in all_candidates.chunks(30) {
        if completed.len() >= 5 { break; }

        let tasks: Vec<_> = chunk
            .iter()
            .map(|(appid, name)| {
                let client = Arc::clone(&client);
                let steam_id = steam_id.to_string();
                let api_key = api_key.to_string();
                let appid = *appid;
                let name = name.clone();
                tokio::spawn(async move {
                    let ach_url = format!(
                        "{}/ISteamUserStats/GetPlayerAchievements/v1/?key={}&steamid={}&appid={}",
                        STEAM_API_BASE, api_key, steam_id, appid
                    );
                    let ach_resp: serde_json::Value = match client.get(&ach_url).send().await {
                        Ok(r) => r.json().await.unwrap_or_default(),
                        Err(_) => return None,
                    };
                    if !ach_resp["playerstats"]["success"].as_bool().unwrap_or(false) {
                        return None;
                    }
                    let achievements = match ach_resp["playerstats"]["achievements"].as_array() {
                        Some(a) if !a.is_empty() => a.clone(),
                        _ => return None,
                    };
                    let total = achievements.len() as u32;
                    let unlocked = achievements
                        .iter()
                        .filter(|a| a["achieved"].as_u64().unwrap_or(0) == 1)
                        .count() as u32;
                    if unlocked == total {
                        Some(SteamCompletedGame { appid, name, achievements_total: total })
                    } else {
                        None
                    }
                })
            })
            .collect();

        let batch_results = join_all(tasks).await;
        for r in batch_results.into_iter().filter_map(|r| r.ok().flatten()) {
            completed.push(r);
            if completed.len() >= 5 { break; }
        }
    }

    Ok(completed)
}

// ─── Types publics ───────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct SteamCompletedGame {
    pub appid: u64,
    pub name: String,
    pub achievements_total: u32,
}

#[derive(serde::Serialize)]
pub struct SteamOwnedGame {
    pub appid: u64,
    pub name: String,
    pub playtime_minutes: i32,
    pub img_icon_url: Option<String>,
    pub last_played_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(serde::Serialize)]
pub struct SteamRecentAchievement {
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub rarity: String,
    pub points: i32,
    pub game_title: String,
    pub game_appid: u64,
    pub unlocked_at: Option<chrono::DateTime<chrono::Utc>>,
}
