use leptos::prelude::*;

#[component]
pub fn Navbar() -> impl IntoView {
    view! {
        <nav class="navbar">
            <div class="navbar-brand">
                <a href="/dashboard">"Achievement Tracker"</a>
            </div>
            <div class="navbar-links">
                <a href="/dashboard">"Dashboard"</a>
                <a href="/platforms">"Plateformes"</a>
                <a href="/profile">"Profil"</a>
                <a href="/" class="btn-logout">"Deconnexion"</a>
            </div>
        </nav>
    }
}
