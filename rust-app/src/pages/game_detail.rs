use leptos::prelude::*;

use crate::pages::components::navbar::Navbar;

#[component]
pub fn GameDetailPage() -> impl IntoView {
    view! {
        <Navbar/>
        <div class="game-detail">
            <h2>"Detail du jeu"</h2>
            <p>"Chargement des achievements..."</p>
        </div>
    }
}
