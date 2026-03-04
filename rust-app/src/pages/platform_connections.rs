use leptos::prelude::*;

use crate::pages::components::navbar::Navbar;

#[component]
pub fn PlatformConnectionsPage() -> impl IntoView {
    view! {
        <Navbar/>
        <div class="platforms-page">
            <h2>"Mes plateformes"</h2>

            <div class="platform-list">
                <div class="platform-card">
                    <div class="platform-info">
                        <h3>"Steam"</h3>
                        <p class="platform-status">"Non connecte"</p>
                    </div>
                    <button class="btn btn-primary">"Connecter"</button>
                </div>

                <div class="platform-card">
                    <div class="platform-info">
                        <h3>"GOG"</h3>
                        <p class="platform-status">"Non connecte"</p>
                        <span class="badge experimental">"Experimental"</span>
                    </div>
                    <button class="btn btn-primary">"Connecter"</button>
                </div>

                <div class="platform-card">
                    <div class="platform-info">
                        <h3>"Epic Games"</h3>
                        <p class="platform-status">"Non connecte"</p>
                        <span class="badge limited">"Acces limite"</span>
                    </div>
                    <button class="btn btn-primary" disabled=true>"Bientot disponible"</button>
                </div>
            </div>
        </div>
    }
}
