use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::SyncStats;

const GOG_EMBED_BASE: &str = "https://embed.gog.com";
const GOG_GAMEPLAY_BASE: &str = "https://gameplay.gog.com";
const GOG_AUTH_BASE: &str = "https://auth.gog.com";

// Credentials GOG Galaxy publics — utilisés par Heroic, LGOGDownloader et autres outils tiers.
// Ces credentials sont connus publiquement et ne constituent pas un secret propriétaire.
const GOG_GALAXY_CLIENT_ID: &str = "46899977096215655";
const GOG_GALAXY_CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";

// ─── Struct réponse token OAuth2 ──────────────────────────────────────────────

#[derive(serde::Deserialize, Debug)]
pub struct GogTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    #[allow(dead_code)]
    pub token_type: String,
    pub user_id: String,
}

// ─── Credentials ─────────────────────────────────────────────────────────────

// redirect_uri imposé par GOG pour le client_id Galaxy public —
// c'est la seule valeur acceptée par tous les clients tiers (Heroic, Lutris, LGOGDownloader).
pub const GOG_REDIRECT_URI: &str = "https://embed.gog.com/on_login_success?origin=client";

/// Construit l'URL d'autorisation GOG OAuth2.
/// Le redirect_uri est fixe (embed.gog.com) — imposé par le client_id Galaxy public.
pub fn gog_oauth_url() -> String {
    format!(
        "https://auth.gog.com/auth?client_id={}&redirect_uri={}&response_type=code&layout=client2",
        GOG_GALAXY_CLIENT_ID,
        urlencoding::encode(GOG_REDIRECT_URI),
    )
}

/// Échange un authorization_code OAuth2 GOG contre access_token + refresh_token.
/// Le redirect_uri doit correspondre exactement à celui utilisé dans l'URL d'autorisation.
pub async fn exchange_gog_code(
    code: &str,
) -> Result<GogTokenResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    // GOG attend un GET avec query params (pas POST form) — même comportement que gogdl/Heroic
    let url = format!(
        "{}/token?client_id={}&client_secret={}&grant_type=authorization_code&code={}&redirect_uri={}",
        GOG_AUTH_BASE,
        GOG_GALAXY_CLIENT_ID,
        GOG_GALAXY_CLIENT_SECRET,
        urlencoding::encode(code),
        urlencoding::encode(GOG_REDIRECT_URI),
    );
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Échange code GOG échoué (HTTP {}): {}", status, body).into());
    }
    Ok(resp.json::<GogTokenResponse>().await?)
}

/// Renouvelle un access_token GOG expiré via le refresh_token.
/// GOG effectue une rotation : le refresh_token retourné remplace l'ancien.
pub async fn refresh_gog_token(
    refresh_token: &str,
) -> Result<GogTokenResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    // GOG attend un GET avec query params — même comportement que gogdl/Heroic
    let url = format!(
        "{}/token?client_id={}&client_secret={}&grant_type=refresh_token&refresh_token={}",
        GOG_AUTH_BASE,
        GOG_GALAXY_CLIENT_ID,
        GOG_GALAXY_CLIENT_SECRET,
        urlencoding::encode(refresh_token),
    );
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Refresh token GOG échoué (HTTP {}): {}", status, body).into());
    }
    Ok(resp.json::<GogTokenResponse>().await?)
}

/// Récupère le username GOG et le token OAuth depuis platform_connections.
/// Si le token expire dans moins de 5 minutes et qu'un refresh_token est disponible,
/// effectue un refresh automatique et met à jour la DB avant de retourner le nouveau token.
pub async fn get_user_gog_credentials(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT platform_user_id, access_token, refresh_token, token_expires_at \
         FROM platform_connections WHERE user_id = $1 AND platform = 'gog'::platform_type",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or("Compte GOG non connecté")?;

    let (gog_user_id, access_token, stored_refresh_token, token_expires_at) = row;

    // Auto-refresh si le token expire dans moins de 5 minutes
    if let (Some(ref rt), Some(expires_at)) = (&stored_refresh_token, token_expires_at) {
        if (expires_at - chrono::Utc::now()).num_minutes() < 5 {
            tracing::info!("GOG token bientôt expiré pour user={}, refresh automatique", user_id);
            match refresh_gog_token(rt).await {
                Ok(new_tokens) => {
                    let new_expires_at = chrono::Utc::now()
                        + chrono::Duration::seconds(new_tokens.expires_in);
                    if let Err(e) = sqlx::query(
                        "UPDATE platform_connections \
                         SET access_token = $1, refresh_token = $2, token_expires_at = $3, updated_at = NOW() \
                         WHERE user_id = $4 AND platform = 'gog'::platform_type",
                    )
                    .bind(&new_tokens.access_token)
                    .bind(&new_tokens.refresh_token)
                    .bind(new_expires_at)
                    .bind(user_id)
                    .execute(pool)
                    .await {
                        tracing::error!("Échec update token GOG après refresh user={}: {}", user_id, e);
                        // On continue avec l'ancien token plutôt que de planter
                    } else {
                        tracing::info!(
                            "GOG token rafraîchi user={}, expire dans {}s",
                            user_id, new_tokens.expires_in
                        );
                        return Ok((gog_user_id, Some(new_tokens.access_token)));
                    }
                }
                Err(e) => {
                    tracing::warn!("Échec refresh token GOG user={}: {} — ancien token conservé", user_id, e);
                }
            }
        }
    }

    Ok((gog_user_id, access_token))
}

// ─── Résolution compte par token OAuth (preuve de propriété) ─────────────────

/// Vérifie un token OAuth GOG et retourne (userId_numérique, username) du propriétaire.
///
/// Appelle embed.gog.com/userData.json — endpoint officiel GOG qui retourne les données
/// du compte propriétaire du token. C'est la seule preuve valide de possession du compte :
/// seul le titulaire peut obtenir un token valide via GOG Galaxy ou le portail GOG.
pub async fn resolve_gog_token(
    token: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client
        .get(format!("{}/userData.json", GOG_EMBED_BASE))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    if resp.status() == 401 || resp.status() == 403 {
        return Err("Token GOG invalide ou expiré. Obtenez un nouveau token dans GOG Galaxy (Paramètres → Connexions).".into());
    }
    if !resp.status().is_success() {
        return Err(format!(
            "API GOG inaccessible (HTTP {}). Réessayez dans un instant.",
            resp.status()
        ).into());
    }

    let data: serde_json::Value = resp.json().await?;

    // galaxyUserId est requis pour l'API gameplay.gog.com/clients/{game}/users/{galaxyUserId}/achievements
    // userId est un ID de compte différent, non utilisable pour les achievements
    let galaxy_user_id = data["galaxyUserId"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["galaxyUserId"].as_u64().map(|n| n.to_string()))
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        .ok_or("Réponse GOG invalide : galaxyUserId manquant ou format inattendu")?;

    let username = data["username"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or(&galaxy_user_id)
        .to_string();

    tracing::info!("Token GOG validé : galaxyUserId={} username={}", galaxy_user_id, username);
    Ok((galaxy_user_id, username))
}

// ─── Vérification compte (scraping — conservé pour rétrocompat) ───────────────

/// Résolution GOG : accepte un username ou un User ID numérique.
/// - Si c'est un ID numérique → retourne (id, username) tel quel.
/// - Si c'est un username → scrape gog.com/u/{username} pour extraire l'userId numérique.
/// Retourne (numeric_user_id, display_username).
pub async fn verify_gog_user(
    input: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Veuillez saisir votre nom d'utilisateur GOG.".into());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // Cas 1 : c'est déjà un ID numérique → retourner directement
    if input.chars().all(|c| c.is_ascii_digit()) && input.len() >= 8 {
        return Ok((input.to_string(), input.to_string()));
    }

    // Cas 2 : username → scraper gog.com/u/{username} pour extraire "userId"
    let url = format!("https://www.gog.com/u/{}", urlencoding::encode(input));
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(format!(
            "Compte GOG '{}' introuvable. Vérifiez votre pseudo sur gog.com/u/{}",
            input, input
        )
        .into());
    }

    let html = resp.text().await?;

    // Extraire l'userId numérique depuis le JSON embarqué dans la page
    let user_id = extract_gog_user_id_from_html(&html).ok_or_else(|| {
        format!(
            "Impossible d'extraire l'identifiant GOG pour '{}'. Votre profil est peut-être privé.",
            input
        )
    })?;

    Ok((user_id, input.to_string()))
}

/// Extrait le userId numérique GOG depuis le HTML de la page de profil.
fn extract_gog_user_id_from_html(html: &str) -> Option<String> {
    // Pattern : "userId":"52344342945716742" (présent dans le JSON embarqué AngularJS)
    let marker = "\"userId\":\"";
    let start = html.find(marker)? + marker.len();
    let end = html[start..].find('"')? + start;
    let id = &html[start..end];
    // Valider que c'est bien un ID numérique
    if id.chars().all(|c| c.is_ascii_digit()) && id.len() >= 8 {
        Some(id.to_string())
    } else {
        None
    }
}

// ─── Jeux possédés ───────────────────────────────────────────────────────────

/// Récupère les jeux possédés depuis embed.gog.com (token OAuth requis).
async fn fetch_gog_owned_games_raw(
    client: &reqwest::Client,
    token: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut all_products: Vec<serde_json::Value> = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "{}/account/getFilteredProducts?mediaType=1&page={}",
            GOG_EMBED_BASE, page
        );
        let resp: serde_json::Value = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?
            .json()
            .await?;

        let products = match resp["products"].as_array() {
            Some(p) if !p.is_empty() => p.clone(),
            _ => break,
        };

        let total_pages = resp["totalPages"].as_u64().unwrap_or(1) as u32;
        all_products.extend(products);

        if page >= total_pages { break; }
        page += 1;
    }

    Ok(all_products)
}

// ─── Achievements par jeu ─────────────────────────────────────────────────────

/// Récupère le nombre d'achievements débloqués et total pour un jeu GOG.
/// Retourne (unlocked, total) ou None si le jeu n'a pas d'achievements.
async fn fetch_gog_achievements(
    client: &reqwest::Client,
    user_id_numeric: &str,
    game_id: &str,
    access_token: &str,
) -> Option<(u32, u32)> {
    let url = format!(
        "{}/clients/{}/users/{}/achievements",
        GOG_GAMEPLAY_BASE, game_id, user_id_numeric
    );

    let resp: serde_json::Value = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
    {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => return None,
    };

    let items = resp["items"].as_array()?;
    if items.is_empty() { return None; }

    let total = items.len() as u32;
    let unlocked = items
        .iter()
        .filter(|a| {
            // date_unlocked est une string non vide si débloqué
            a["date_unlocked"]
                .as_str()
                .map(|s| !s.is_empty() && s != "0000-00-00T00:00:00.000Z")
                .unwrap_or(false)
        })
        .count() as u32;

    Some((unlocked, total))
}

// ─── Sync principal ───────────────────────────────────────────────────────────

/// Synchronise les achievements GOG d'un utilisateur.
pub async fn sync_gog_achievements(
    pool: &PgPool,
    user_id: Uuid,
    gog_username: &str,
    access_token: &str,
) -> Result<SyncStats, Box<dyn std::error::Error>> {
    if access_token.is_empty() {
        return Err("Token OAuth GOG requis pour synchroniser les jeux. Reconnectez votre compte GOG avec un token valide.".into());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;

    // 1. Le gog_username contient le userId numérique GOG
    let user_id_numeric = gog_username.trim().to_string();

    // 2. Récupérer les jeux possédés
    let products = fetch_gog_owned_games_raw(&client, access_token).await?;
    tracing::info!("GOG sync: {} jeux trouvés pour {}", products.len(), gog_username);

    let mut games_synced: u32 = 0;
    let mut achievements_synced: u32 = 0;

    for product in &products {
        let game_id_raw = match product["id"].as_u64() {
            Some(id) => id.to_string(),
            None => continue,
        };
        let title = product["title"]
            .as_str()
            .unwrap_or("Jeu inconnu")
            .to_string();

        // Construire l'URL de couverture GOG depuis le champ image (format GOG CDN)
        let cover_image_url: Option<String> = product["image"]
            .as_str()
            .map(|img| {
                if img.starts_with("http") {
                    img.to_string()
                } else {
                    format!("https:{}_product_card_v2_mobile_slider_639.jpg", img)
                }
            });

        // 3. Vérifier s'il y a des achievements
        let (unlocked, total) = match fetch_gog_achievements(&client, &user_id_numeric, &game_id_raw, access_token).await {
            Some(stats) if stats.1 > 0 => stats,
            _ => continue, // Pas d'achievements pour ce jeu
        };

        let pct = if total > 0 { unlocked as f64 / total as f64 * 100.0 } else { 0.0 };
        let normalized_title = title.to_lowercase();

        // 4. Upsert dans games
        sqlx::query(
            r#"
            INSERT INTO games (title, normalized_title)
            VALUES ($1, $2)
            ON CONFLICT (normalized_title) DO UPDATE SET title = EXCLUDED.title
            "#,
        )
        .bind(&title)
        .bind(&normalized_title)
        .execute(pool)
        .await
        .map_err(|e| format!("Erreur upsert game '{}': {}", title, e))?;

        let game_db_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM games WHERE normalized_title = $1",
        )
        .bind(&normalized_title)
        .fetch_one(pool)
        .await?;

        // 5. Upsert dans game_platform_ids (avec cover_image_url)
        sqlx::query(
            r#"
            INSERT INTO game_platform_ids (game_id, platform, platform_game_id, platform_name, total_achievements, cover_image_url)
            VALUES ($1, 'gog'::platform_type, $2, $3, $4, $5)
            ON CONFLICT (platform, platform_game_id) DO UPDATE
            SET platform_name      = EXCLUDED.platform_name,
                total_achievements = EXCLUDED.total_achievements,
                cover_image_url    = EXCLUDED.cover_image_url
            "#,
        )
        .bind(game_db_id)
        .bind(&game_id_raw)
        .bind(&title)
        .bind(total as i32)
        .bind(&cover_image_url)
        .execute(pool)
        .await
        .map_err(|e| format!("Erreur upsert game_platform_ids '{}': {}", title, e))?;

        // 6. Upsert dans user_game_stats
        sqlx::query(
            r#"
            INSERT INTO user_game_stats (user_id, game_id, achievements_unlocked, achievements_total, completion_pct)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, game_id) DO UPDATE
            SET achievements_unlocked = EXCLUDED.achievements_unlocked,
                achievements_total    = EXCLUDED.achievements_total,
                completion_pct        = EXCLUDED.completion_pct,
                updated_at            = NOW()
            "#,
        )
        .bind(user_id)
        .bind(game_db_id)
        .bind(unlocked as i32)
        .bind(total as i32)
        .bind(pct)
        .execute(pool)
        .await
        .map_err(|e| format!("Erreur upsert user_game_stats '{}': {}", title, e))?;

        games_synced += 1;
        achievements_synced += unlocked;
    }

    // 7+8. Transaction : snapshot rang + recalcul totaux (atomique)
    let final_stats: (i64, i64) = {
        let mut tx = match pool.begin().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Echec début transaction finales GOG user={}: {}", user_id, e);
                return Err(e.into());
            }
        };

        // 7. Snapshot du rang AVANT mise à jour des points
        if let Err(e) = sqlx::query(
            r#"
            UPDATE users u SET
                rank_snapshot    = ranked.rank_pts,
                rank_snapshot_at = NOW()
            FROM (
                SELECT id, RANK() OVER (ORDER BY total_points DESC)::BIGINT AS rank_pts
                FROM users WHERE is_active = true
            ) ranked
            WHERE u.id = $1
              AND ranked.id = $1
              AND (u.rank_snapshot_at IS NULL OR u.rank_snapshot_at < NOW() - INTERVAL '24 hours')
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await {
            tracing::warn!("Echec snapshot rang GOG user={}: {}", user_id, e);
        }

        // 8. Recalculer les totaux utilisateur
        let stats = sqlx::query_as::<_, (i64, i64)>(
            r#"
            UPDATE users
            SET
                total_achievements_count = (
                    SELECT COALESCE(SUM(achievements_unlocked), 0)
                    FROM user_game_stats WHERE user_id = $1
                ),
                games_completed = (
                    SELECT COUNT(*) FROM user_game_stats
                    WHERE user_id = $1 AND achievements_total > 0 AND achievements_unlocked >= achievements_total
                ),
                total_possible_achievements = (
                    SELECT COALESCE(SUM(achievements_total), 0)
                    FROM user_game_stats WHERE user_id = $1
                ),
                total_points = (
                    SELECT COALESCE(SUM(achievements_unlocked), 0) * 10
                         + COUNT(*) FILTER (WHERE achievements_total > 0 AND achievements_unlocked >= achievements_total) * 50
                    FROM user_game_stats WHERE user_id = $1
                )
            WHERE id = $1
            RETURNING total_achievements_count, games_completed::BIGINT
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Erreur recalcul totaux GOG user={}: {}", user_id, e);
            None
        })
        .unwrap_or((0, 0));

        if let Err(e) = tx.commit().await {
            tracing::error!("Echec commit transaction finales GOG user={}: {}", user_id, e);
        }

        stats
    };

    tracing::info!(
        "GOG sync terminé pour {} : {} jeux, {} achievements",
        gog_username, games_synced, achievements_synced
    );

    Ok(SyncStats {
        games_synced,
        achievements_synced,
        total_achievements: final_stats.0 as u32,
        games_completed: final_stats.1 as u32,
    })
}

// ─── Lecture des données GOG depuis la DB ─────────────────────────────────────

fn gog_rarity(global_pct: f64) -> &'static str {
    if global_pct < 5.0 { "mythic" }
    else if global_pct < 15.0 { "legendary" }
    else if global_pct < 40.0 { "epic" }
    else if global_pct < 70.0 { "rare" }
    else { "common" }
}

fn gog_points(rarity: &str) -> i32 {
    match rarity {
        "mythic"    => 2000,
        "legendary" => 500,
        "epic"      => 100,
        "rare"      => 25,
        _           => 5,
    }
}

#[derive(Serialize)]
pub struct GogGame {
    pub platform_game_id: String,
    pub title: String,
    pub cover_image_url: Option<String>,
    pub achievements_unlocked: i32,
    pub achievements_total: i32,
    pub completion_pct: f64,
    pub playtime_minutes: i32,
}

#[derive(Serialize)]
pub struct GogCompletedGame {
    pub platform_game_id: String,
    pub title: String,
    pub cover_image_url: Option<String>,
    pub achievements_total: i32,
}

#[derive(Serialize)]
pub struct GogRecentAchievement {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub image_url: String,
    pub unlocked_at: String,
    pub game_name: String,
    pub platform_game_id: String,
    pub rarity: String,
    pub points: i32,
    pub global_percentage: f64,
}

/// Récupère les jeux GOG avec stats de complétion depuis la DB.
pub async fn fetch_gog_games_from_db(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<GogGame>, Box<dyn std::error::Error>> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, i32, i32, f64, i32)>(
        r#"
        SELECT gpi.platform_game_id, gpi.platform_name, gpi.cover_image_url,
               ugs.achievements_unlocked, ugs.achievements_total, ugs.completion_pct::FLOAT8,
               ugs.playtime_minutes
        FROM user_game_stats ugs
        JOIN game_platform_ids gpi ON gpi.game_id = ugs.game_id
            AND gpi.platform = 'gog'::platform_type
        WHERE ugs.user_id = $1
        ORDER BY ugs.completion_pct DESC, ugs.achievements_unlocked DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(platform_game_id, title, cover_image_url, achievements_unlocked, achievements_total, completion_pct, playtime_minutes)| GogGame {
            platform_game_id,
            title,
            cover_image_url,
            achievements_unlocked,
            achievements_total,
            completion_pct,
            playtime_minutes,
        })
        .collect())
}

/// Récupère les jeux GOG complétés à 100% depuis la DB.
pub async fn fetch_gog_completed_from_db(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<GogCompletedGame>, Box<dyn std::error::Error>> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, i32)>(
        r#"
        SELECT gpi.platform_game_id, gpi.platform_name, gpi.cover_image_url, ugs.achievements_total
        FROM user_game_stats ugs
        JOIN game_platform_ids gpi ON gpi.game_id = ugs.game_id
            AND gpi.platform = 'gog'::platform_type
        WHERE ugs.user_id = $1
          AND ugs.achievements_total > 0
          AND ugs.achievements_unlocked >= ugs.achievements_total
        ORDER BY ugs.updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(platform_game_id, title, cover_image_url, achievements_total)| GogCompletedGame {
            platform_game_id,
            title,
            cover_image_url,
            achievements_total,
        })
        .collect())
}

/// Récupère les achievements GOG récents via l'API GOG (token Bearer requis).
/// Parcourt les jeux récents en DB et appelle gameplay.gog.com pour chaque jeu.
/// Retourne une liste vide si aucun token n'est fourni ou si l'API est inaccessible.
pub async fn fetch_gog_recent_achievements(
    pool: &PgPool,
    user_id: Uuid,
    gog_user_id: &str,
    access_token: Option<&str>,
) -> Result<Vec<GogRecentAchievement>, Box<dyn std::error::Error>> {
    let token = match access_token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(vec![]), // Sans token, l'API GOG refuse l'accès
    };

    let client = reqwest::Client::builder()
        .user_agent("GOG Galaxy/2.0")
        .timeout(std::time::Duration::from_secs(8))
        .build()?;

    // Priorité 1 : jeux déjà synchros en DB avec achievements débloqués (rapide)
    let db_game_ids = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT gpi.platform_game_id, gpi.platform_name
        FROM user_game_stats ugs
        JOIN game_platform_ids gpi ON gpi.game_id = ugs.game_id
            AND gpi.platform = 'gog'::platform_type
        WHERE ugs.user_id = $1
          AND ugs.achievements_unlocked > 0
        ORDER BY ugs.updated_at DESC
        LIMIT 10
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Priorité 2 : si DB vide (avant première sync), interroger l'API GOG directement
    let game_list: Vec<(String, String)> = if db_game_ids.is_empty() {
        match fetch_gog_owned_games_raw(&client, token).await {
            Ok(products) => products
                .into_iter()
                .filter_map(|p| {
                    let id = p["id"].as_u64()?.to_string();
                    let name = p["title"].as_str().unwrap_or("Jeu inconnu").to_string();
                    Some((id, name))
                })
                .collect(),
            Err(e) => {
                tracing::warn!("Impossible de récupérer les jeux GOG pour achievements récents: {}", e);
                return Ok(vec![]);
            }
        }
    } else {
        db_game_ids
    };

    let mut all_recent: Vec<GogRecentAchievement> = Vec::new();

    for (game_id, game_name) in &game_list {
        let url = format!(
            "{}/clients/{}/users/{}/achievements",
            GOG_GAMEPLAY_BASE, game_id, gog_user_id
        );

        let resp: serde_json::Value = match client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
            Ok(r) if r.status().as_u16() == 401 => {
                // Token expiré — inutile de continuer
                tracing::warn!("GOG token expiré pour user_id={}", user_id);
                break;
            }
            _ => continue,
        };

        let items = match resp["items"].as_array() {
            Some(a) if !a.is_empty() => a,
            _ => continue,
        };

        let mut unlocked: Vec<&serde_json::Value> = items
            .iter()
            .filter(|a| {
                a["date_unlocked"]
                    .as_str()
                    .map(|s| !s.is_empty() && s != "0000-00-00T00:00:00.000Z")
                    .unwrap_or(false)
            })
            .collect();

        // Trier par date décroissante
        unlocked.sort_by(|a, b| {
            let da = a["date_unlocked"].as_str().unwrap_or("");
            let db = b["date_unlocked"].as_str().unwrap_or("");
            db.cmp(da)
        });

        for a in unlocked.into_iter().take(3) {
            let global_pct = a["rarity"].as_f64().unwrap_or(0.0);
            let rarity = gog_rarity(global_pct);
            let points = gog_points(rarity);
            // Normaliser le format de date GOG vers ISO 8601 strict :
            // "2023-12-26T15:32:12+0000" → "2023-12-26T15:32:12+00:00"
            let raw_date = a["date_unlocked"].as_str().unwrap_or("");
            let unlocked_at = if raw_date.len() == 24 && raw_date.ends_with("+0000") {
                format!("{}+00:00", &raw_date[..19])
            } else if raw_date.len() == 24 && raw_date.ends_with("-0000") {
                format!("{}+00:00", &raw_date[..19])
            } else {
                raw_date.to_string()
            };
            all_recent.push(GogRecentAchievement {
                name: a["achievement_id"].as_str().unwrap_or("").to_string(),
                display_name: a["name"].as_str().unwrap_or("").to_string(),
                description: a["description"].as_str().unwrap_or("").to_string(),
                image_url: a["image_url_unlocked"].as_str().unwrap_or("").to_string(),
                unlocked_at,
                game_name: game_name.clone(),
                platform_game_id: game_id.clone(),
                rarity: rarity.to_string(),
                points,
                global_percentage: global_pct,
            });
        }

        if all_recent.len() >= 15 {
            break;
        }
    }

    // Trier par date et retourner les 5 plus récents
    all_recent.sort_by(|a, b| b.unlocked_at.cmp(&a.unlocked_at));
    all_recent.truncate(5);

    Ok(all_recent)
}
