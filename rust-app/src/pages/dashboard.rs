use leptos::prelude::*;

use crate::pages::components::navbar::Navbar;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let user = RwSignal::new(Option::<serde_json::Value>::None);
    let loading = RwSignal::new(true);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            let Some(window) = leptos::web_sys::window() else {
                loading.set(false);
                return;
            };
            let Ok(Some(storage)) = window.local_storage() else {
                loading.set(false);
                return;
            };
            let token = match storage.get_item("auth_token").ok().flatten() {
                Some(t) => t,
                None => {
                    let _ = window.location().set_href("/login");
                    return;
                }
            };

            let client = reqwest::Client::new();
            match client
                .get("/api/auth/me")
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        user.set(Some(body));
                    }
                }
                _ => {
                    let _ = window.location().set_href("/login");
                    return;
                }
            }
            loading.set(false);
        });
    });

    view! {
        <Navbar/>
        <div class="dashboard">
            {move || {
                if loading.get() {
                    view! {
                        <div style="text-align:center;padding:64px;color:var(--text-secondary)">
                            "Chargement..."
                        </div>
                    }
                    .into_any()
                } else {
                    let greeting = user
                        .get()
                        .and_then(|u| {
                            u["display_name"]
                                .as_str()
                                .or(u["username"].as_str())
                                .map(|n| format!("Bienvenue, {} !", n))
                        })
                        .unwrap_or_else(|| "Tableau de bord".into());

                    view! {
                        <h2>{greeting}</h2>

                        <section class="stats-overview">
                            <div class="stat-card">
                                <span class="stat-value">"--"</span>
                                <span class="stat-label">"Achievements debloques"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-value">"--%"</span>
                                <span class="stat-label">"Completion globale"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-value">"--"</span>
                                <span class="stat-label">"Jeux"</span>
                            </div>
                        </section>

                        <section class="platform-stats">
                            <h3>"Par plateforme"</h3>
                            <div class="platform-grid">
                                <div class="platform-stat">
                                    <span class="platform-name">"Steam"</span>
                                    <span class="platform-progress">"-- / --"</span>
                                </div>
                                <div class="platform-stat">
                                    <span class="platform-name">"GOG"</span>
                                    <span class="platform-progress">"-- / --"</span>
                                </div>
                                <div class="platform-stat">
                                    <span class="platform-name">"Epic Games"</span>
                                    <span class="platform-progress">"-- / --"</span>
                                </div>
                            </div>
                        </section>

                        <section class="recent-achievements">
                            <h3>"Derniers achievements debloques"</h3>
                            <p class="empty-state">
                                "Connectez une plateforme et synchronisez pour voir vos achievements."
                            </p>
                        </section>
                    }
                    .into_any()
                }
            }}
        </div>
    }
}
