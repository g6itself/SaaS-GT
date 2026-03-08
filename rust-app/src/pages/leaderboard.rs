use leptos::prelude::*;

#[component]
pub fn LeaderboardPage() -> impl IntoView {
    // Données mockées — à remplacer par un appel API Rust
    let top3 = vec![
        ("ShadowHunter", 1247, 87.3_f64, "🥇"),
        ("TrophyMaster", 1189, 82.1_f64, "🥈"),
        ("AchievR_Pro",  1056, 79.8_f64, "🥉"),
    ];
    let visible = vec![
        (4,  "CompletionistX",  998, 76.4_f64),
        (5,  "NightWolf_99",    934, 71.2_f64),
        (6,  "SpeedRunner_VII", 887, 68.9_f64),
    ];
    let blurred = vec![
        (7,  "PhantomAce",      821),
        (8,  "LegendSeeker",    776),
        (9,  "TrophyVault",     734),
        (10, "AchievHunter",    698),
    ];

    view! {
        <div class="page-wrapper">
            <div class="leaderboard-container">
                <div class="leaderboard-header">
                    <h1 class="page-title">"Classement mondial"</h1>
                    <p class="page-subtitle">"Les meilleurs chasseurs de trophées de la plateforme"</p>
                </div>

                // ── Podium Top 3 ────────────────────────────────────────────────
                <div class="podium">
                    // #2 — Argent
                    <div class="podium-card podium-silver">
                        <div class="podium-crown">"🥈"</div>
                        <div class="podium-rank">"#2"</div>
                        <div class="podium-avatar silver-glow">"🎮"</div>
                        <div class="podium-name">{top3[1].0}</div>
                        <div class="podium-trophies">{top3[1].1}" trophées"</div>
                        <div class="podium-pct">{format!("{:.1}%", top3[1].2)}" complétion"</div>
                        <div class="podium-bar" style="height:100px;"></div>
                    </div>
                    // #1 — Or
                    <div class="podium-card podium-gold">
                        <div class="podium-crown">"👑"</div>
                        <div class="podium-rank">"#1"</div>
                        <div class="podium-avatar gold-glow">"🏆"</div>
                        <div class="podium-name">{top3[0].0}</div>
                        <div class="podium-trophies">{top3[0].1}" trophées"</div>
                        <div class="podium-pct">{format!("{:.1}%", top3[0].2)}" complétion"</div>
                        <div class="podium-bar" style="height:140px;"></div>
                    </div>
                    // #3 — Bronze
                    <div class="podium-card podium-bronze">
                        <div class="podium-crown">"🥉"</div>
                        <div class="podium-rank">"#3"</div>
                        <div class="podium-avatar bronze-glow">"⚡"</div>
                        <div class="podium-name">{top3[2].0}</div>
                        <div class="podium-trophies">{top3[2].1}" trophées"</div>
                        <div class="podium-pct">{format!("{:.1}%", top3[2].2)}" complétion"</div>
                        <div class="podium-bar" style="height:70px;"></div>
                    </div>
                </div>

                // ── Tableau rangs 4-6 (visible) ─────────────────────────────────
                <div class="leaderboard-table">
                    <div class="table-header">
                        <span>"Rang"</span>
                        <span>"Joueur"</span>
                        <span>"Trophées"</span>
                        <span>"Complétion"</span>
                    </div>
                    {visible.iter().map(|(rank, name, trophies, pct)| {
                        let rank = *rank;
                        let trophies = *trophies;
                        let pct = *pct;
                        view! {
                            <div class="table-row">
                                <span class="rank-badge">"#"{rank}</span>
                                <span class="player-name">{*name}</span>
                                <span class="trophy-count">"🏆 "{trophies}</span>
                                <span class="completion-pct">{format!("{:.1}%", pct)}</span>
                            </div>
                        }
                    }).collect_view()}

                    // ── Rangs 7-10 : floutés ────────────────────────────────────
                    <div class="blur-teaser">
                        <div class="blur-rows">
                            {blurred.iter().map(|(rank, name, trophies)| {
                                let rank = *rank;
                                let trophies = *trophies;
                                view! {
                                    <div class="table-row blurred-row">
                                        <span class="rank-badge">"#"{rank}</span>
                                        <span class="player-name">{*name}</span>
                                        <span class="trophy-count">"🏆 "{trophies}</span>
                                        <span class="completion-pct">"??%"</span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                        <div class="blur-overlay">
                            <p>"Inscrivez-vous pour voir la suite"</p>
                            <a href="/register" class="btn-primary">"Créer un compte gratuit"</a>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
