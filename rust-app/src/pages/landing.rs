use leptos::prelude::*;

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div class="landing">
            <header class="landing-header">
                <h1>"Achievement Tracker"</h1>
                <p class="subtitle">"Centralisez vos trophees Steam, GOG et Epic Games en un seul endroit"</p>
            </header>

            <section class="features">
                <div class="feature-card">
                    <h3>"Multi-plateforme"</h3>
                    <p>"Connectez vos comptes Steam, GOG et Epic Games pour voir tous vos achievements."</p>
                </div>
                <div class="feature-card">
                    <h3>"Tableau de bord"</h3>
                    <p>"Visualisez votre progression globale et par jeu en un coup d'oeil."</p>
                </div>
                <div class="feature-card">
                    <h3>"Synchronisation"</h3>
                    <p>"Synchronisez automatiquement vos achievements depuis chaque plateforme."</p>
                </div>
            </section>

            <div class="cta">
                <a href="/register" class="btn btn-primary">"Creer un compte"</a>
                <a href="/login" class="btn btn-secondary">"Se connecter"</a>
            </div>
        </div>
    }
}
