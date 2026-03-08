use leptos::prelude::*;

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn entry_name(e: &serde_json::Value) -> String {
    e["display_name"]
        .as_str()
        .or_else(|| e["username"].as_str())
        .unwrap_or("—")
        .to_string()
}

fn entry_username(e: &serde_json::Value) -> String {
    e["username"].as_str().unwrap_or("").to_string()
}

#[component]
pub fn LeaderboardPage() -> impl IntoView {
    let entries: RwSignal<Vec<serde_json::Value>> = RwSignal::new(vec![]);
    let loading = RwSignal::new(true);

    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            let client = reqwest::Client::new();
            match client.get("/api/leaderboard").send().await {
                Ok(r) if r.status().is_success() => {
                    if let Ok(data) = r.json::<Vec<serde_json::Value>>().await {
                        entries.set(data);
                    }
                }
                _ => {}
            }
            loading.set(false);
        });
    });

    view! {
        <div class="page-wrapper">
            <div class="leaderboard-container">
                <div class="leaderboard-header">
                    <h1 class="page-title">"Classement mondial"</h1>
                    <p class="page-subtitle">"Les meilleurs chasseurs de trophées de la plateforme"</p>
                </div>

                {move || {
                    if loading.get() {
                        return view! {
                            <div style="text-align:center;padding:64px;color:var(--text-secondary)">
                                "Chargement..."
                            </div>
                        }
                        .into_any();
                    }

                    let data = entries.get();

                    if data.is_empty() {
                        return view! {
                            <div class="empty-state">
                                <p>"Aucun joueur pour le moment."</p>
                                <a href="/register" class="btn btn-primary">
                                    "Créer un compte gratuit"
                                </a>
                            </div>
                        }
                        .into_any();
                    }

                    let top3: Vec<serde_json::Value> = data.iter().take(3).cloned().collect();
                    let visible: Vec<serde_json::Value> =
                        data.iter().skip(3).take(3).cloned().collect();
                    let blurred: Vec<serde_json::Value> =
                        data.iter().skip(6).take(4).cloned().collect();

                    view! {
                        // ── Podium Top 3 (order: silver, gold, bronze) ──────────
                        <div class="podium">
                            {[1usize, 0, 2]
                                .into_iter()
                                .filter_map(|i| top3.get(i).cloned())
                                .map(|e| {
                                    let rank = e["rank"].as_i64().unwrap_or(0) as usize;
                                    let name = entry_name(&e);
                                    let username = entry_username(&e);
                                    let trophies = e["total_achievements"].as_i64().unwrap_or(0);
                                    let pct = e["completion_avg"].as_f64().unwrap_or(0.0);
                                    let profile_url = format!(
                                        "/profile?u={}",
                                        percent_encode(&username)
                                    );
                                    let (card_class, avatar_class, crown, height) = match rank {
                                        1 => (
                                            "podium-card podium-gold",
                                            "podium-avatar gold-glow",
                                            "👑",
                                            "140px",
                                        ),
                                        2 => (
                                            "podium-card podium-silver",
                                            "podium-avatar silver-glow",
                                            "🥈",
                                            "100px",
                                        ),
                                        _ => (
                                            "podium-card podium-bronze",
                                            "podium-avatar bronze-glow",
                                            "🥉",
                                            "70px",
                                        ),
                                    };
                                    let medal = match rank {
                                        1 => "🥇",
                                        2 => "🥈",
                                        _ => "🥉",
                                    };
                                    view! {
                                        <a
                                            href=profile_url
                                            class=card_class
                                            style="text-decoration:none;color:inherit;cursor:pointer"
                                        >
                                            <div class="podium-crown">{crown}</div>
                                            <div class="podium-rank">"#"{rank}</div>
                                            <div class=avatar_class>{medal}</div>
                                            <div class="podium-name">{name}</div>
                                            <div class="podium-trophies">
                                                {trophies}
                                                " trophées"
                                            </div>
                                            <div class="podium-pct">
                                                {format!("{:.1}%", pct)}
                                                " complétion"
                                            </div>
                                            <div
                                                class="podium-bar"
                                                style=format!("height:{}", height)
                                            ></div>
                                        </a>
                                    }
                                })
                                .collect_view()}
                        </div>

                        // ── Table rangs 4–6 (visible) ────────────────────────────
                        <div class="leaderboard-table">
                            <div class="table-header">
                                <span>"Rang"</span>
                                <span>"Joueur"</span>
                                <span>"Trophées"</span>
                                <span>"Complétion"</span>
                            </div>
                            {visible
                                .into_iter()
                                .map(|e| {
                                    let rank = e["rank"].as_i64().unwrap_or(0);
                                    let name = entry_name(&e);
                                    let username = entry_username(&e);
                                    let trophies = e["total_achievements"].as_i64().unwrap_or(0);
                                    let pct = e["completion_avg"].as_f64().unwrap_or(0.0);
                                    let profile_url = format!(
                                        "/profile?u={}",
                                        percent_encode(&username)
                                    );
                                    view! {
                                        <a
                                            href=profile_url
                                            class="table-row"
                                            style="text-decoration:none;color:inherit;cursor:pointer"
                                        >
                                            <span class="rank-badge">"#"{rank}</span>
                                            <span class="player-name">{name}</span>
                                            <span class="trophy-count">"🏆 "{trophies}</span>
                                            <span class="completion-pct">
                                                {format!("{:.1}%", pct)}
                                            </span>
                                        </a>
                                    }
                                })
                                .collect_view()}

                            // ── Rangs 7–10 : floutés ─────────────────────────────
                            <div class="blur-teaser">
                                <div class="blur-rows">
                                    {blurred
                                        .into_iter()
                                        .map(|e| {
                                            let rank = e["rank"].as_i64().unwrap_or(0);
                                            let name = entry_name(&e);
                                            let trophies = e["total_achievements"]
                                                .as_i64()
                                                .unwrap_or(0);
                                            view! {
                                                <div class="table-row blurred-row">
                                                    <span class="rank-badge">"#"{rank}</span>
                                                    <span class="player-name">{name}</span>
                                                    <span class="trophy-count">
                                                        "🏆 "
                                                        {trophies}
                                                    </span>
                                                    <span class="completion-pct">"??%"</span>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                                <div class="blur-overlay">
                                    <p>"Inscrivez-vous pour voir la suite"</p>
                                    <a href="/register" class="btn-primary">
                                        "Créer un compte gratuit"
                                    </a>
                                </div>
                            </div>
                        </div>
                    }
                    .into_any()
                }}
            </div>
        </div>
    }
}
