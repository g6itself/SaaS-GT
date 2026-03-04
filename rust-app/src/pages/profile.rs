use leptos::prelude::*;

use crate::pages::components::navbar::Navbar;

#[component]
pub fn ProfilePage() -> impl IntoView {
    view! {
        <Navbar/>
        <div class="profile-page">
            <h2>"Mon profil"</h2>

            <form class="profile-form">
                <div class="form-group">
                    <label>"Nom d'affichage"</label>
                    <input type="text" placeholder="Votre nom d'affichage"/>
                </div>

                <div class="form-group">
                    <label>"Email"</label>
                    <input type="email" disabled=true placeholder="email@exemple.com"/>
                </div>

                <button type="submit" class="btn btn-primary">"Sauvegarder"</button>
            </form>

            <hr/>

            <div class="danger-zone">
                <h3>"Zone dangereuse"</h3>
                <button class="btn btn-danger">"Supprimer mon compte"</button>
            </div>
        </div>
    }
}
