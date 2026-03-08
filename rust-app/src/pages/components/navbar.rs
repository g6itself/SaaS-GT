use leptos::prelude::*;

#[component]
pub fn Navbar() -> impl IntoView {
    let username = RwSignal::new(String::new());

    Effect::new(move |_| {
        if let Some(window) = leptos::web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(u)) = storage.get_item("auth_user") {
                    username.set(u);
                }
            }
        }
    });

    let logout = move |_: leptos::ev::MouseEvent| {
        if let Some(window) = leptos::web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.remove_item("auth_token");
                let _ = storage.remove_item("auth_user");
            }
            let _ = window.location().set_href("/");
        }
    };

    view! {
        <nav class="navbar">
            <div class="navbar-brand">
                <a href="/dashboard">"Achievement Tracker"</a>
            </div>
            <div class="navbar-links">
                <a href="/dashboard">"Dashboard"</a>
                <a href="/leaderboard">"Classement"</a>
                <a href="/platforms">"Plateformes"</a>
                <a href="/profile">"Profil"</a>
                {move || {
                    let u = username.get();
                    if !u.is_empty() {
                        view! { <span class="nav-username">{u}</span> }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
                <button class="btn-logout" on:click=logout>"Deconnexion"</button>
            </div>
        </nav>
    }
}
