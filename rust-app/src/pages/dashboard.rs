use leptos::prelude::*;

use crate::pages::components::navbar::Navbar;

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <Navbar/>
        <div class="dashboard">
            <h2>"Tableau de bord"</h2>

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
                <p class="empty-state">"Connectez une plateforme et synchronisez pour voir vos achievements."</p>
            </section>
        </div>
    }
}
