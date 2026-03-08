use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::pages::components::navbar::Navbar;

#[component]
pub fn ProfilePage() -> impl IntoView {
    // ── Query param: ?u=USERNAME for viewing other users ─────────────────────
    let query = use_query_map();
    let target_user = Memo::new(move |_| {
        query.read().get("u").filter(|s: &String| !s.is_empty())
    });

    // ── Public profile state (when viewing another user) ──────────────────────
    let pub_data = RwSignal::new(Option::<serde_json::Value>::None);
    let pub_loading = RwSignal::new(false);
    let pub_error = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        let Some(_u) = target_user.get() else {
            return;
        };
        pub_loading.set(true);
        pub_data.set(None);
        pub_error.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            leptos::task::spawn_local(async move {
                let client = reqwest::Client::new();
                let url = format!("/api/users/{}", _u);
                match client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => {
                        if let Ok(data) = r.json::<serde_json::Value>().await {
                            pub_data.set(Some(data));
                        } else {
                            pub_error.set(Some("Réponse invalide".into()));
                        }
                    }
                    Ok(r) if r.status().as_u16() == 404 => {
                        pub_error.set(Some(format!("Joueur \"{}\" introuvable.", _u)));
                    }
                    _ => {
                        pub_error.set(Some("Impossible de charger le profil.".into()));
                    }
                }
                pub_loading.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        pub_loading.set(false);
    });

    // ── Own profile state ─────────────────────────────────────────────────────
    let display_name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let username_val = RwSignal::new(String::new());
    let loading = RwSignal::new(true);
    let saving = RwSignal::new(false);
    let save_msg = RwSignal::new(Option::<(String, bool)>::None);

    // Load own profile on mount (skipped when ?u= is set)
    Effect::new(move |_| {
        if target_user.get().is_some() {
            loading.set(false);
            return;
        }
        #[cfg(target_arch = "wasm32")]
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
                        email.set(body["email"].as_str().unwrap_or("").to_string());
                        username_val
                            .set(body["username"].as_str().unwrap_or("").to_string());
                        display_name.set(
                            body["display_name"].as_str().unwrap_or("").to_string(),
                        );
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

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        saving.set(true);
        save_msg.set(None);

        let dn = display_name.get();
        if dn.len() > 50 {
            save_msg.set(Some(("Le nom d'affichage ne peut pas depasser 50 caracteres".into(), false)));
            saving.set(false);
            return;
        }

        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            let Some(window) = leptos::web_sys::window() else {
                save_msg.set(Some(("Environnement navigateur indisponible".into(), false)));
                saving.set(false);
                return;
            };
            let Ok(Some(storage)) = window.local_storage() else {
                save_msg.set(Some(("Stockage local indisponible".into(), false)));
                saving.set(false);
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
                .put("/api/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .json(&serde_json::json!({ "display_name": dn }))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => {
                    save_msg.set(Some(("Profil mis a jour avec succes !".into(), true)));
                }
                Ok(r) => {
                    let msg = r
                        .json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|b| b["error"].as_str().map(String::from))
                        .unwrap_or_else(|| "Erreur lors de la sauvegarde".into());
                    save_msg.set(Some((msg, false)));
                }
                Err(_) => {
                    save_msg.set(Some(("Erreur reseau, veuillez reessayer".into(), false)));
                }
            }
            saving.set(false);
        });
    };

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
        <Navbar/>
        <div class="profile-page">
            {move || {
                if let Some(_) = target_user.get() {
                    // ── Public profile view ───────────────────────────────────
                    if pub_loading.get() {
                        return view! {
                            <p style="color:var(--text-secondary);text-align:center;padding:32px">
                                "Chargement..."
                            </p>
                        }
                        .into_any();
                    }
                    if let Some(err) = pub_error.get() {
                        return view! {
                            <div class="error-message">{err}</div>
                            <a href="/leaderboard" style="display:block;margin-top:16px">
                                "← Retour au classement"
                            </a>
                        }
                        .into_any();
                    }
                    if let Some(data) = pub_data.get() {
                        let username = data["username"].as_str().unwrap_or("").to_string();
                        let display = data["display_name"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .unwrap_or(data["username"].as_str().unwrap_or("?"))
                            .to_string();
                        let league = data["league"].as_str().unwrap_or("Orbit").to_string();
                        let title = data["active_title"]
                            .as_str()
                            .unwrap_or("Chasseur de Trophées")
                            .to_string();
                        let total_achievements =
                            data["total_achievements"].as_i64().unwrap_or(0);
                        let completion_avg = data["completion_avg"].as_f64().unwrap_or(0.0);
                        let rank = data["rank"].as_i64().unwrap_or(0);
                        let total_points = data["total_points"].as_i64().unwrap_or(0);

                        return view! {
                            <h2>{display}</h2>
                            <p class="field-hint" style="margin-bottom:8px">
                                "@"
                                {username}
                            </p>
                            <p class="field-hint" style="margin-bottom:24px">
                                "Titre : "
                                {title}
                                " · Ligue : "
                                {league}
                            </p>

                            <section class="stats-overview">
                                <div class="stat-card">
                                    <span class="stat-value">{total_achievements}</span>
                                    <span class="stat-label">"Achievements débloqués"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-value">
                                        {format!("{:.1}%", completion_avg)}
                                    </span>
                                    <span class="stat-label">"Complétion globale"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-value">
                                        {if rank > 0 { format!("#{}", rank) } else { "—".into() }}
                                    </span>
                                    <span class="stat-label">"Classement"</span>
                                </div>
                                <div class="stat-card">
                                    <span class="stat-value">{total_points}</span>
                                    <span class="stat-label">"Points"</span>
                                </div>
                            </section>

                            <a
                                href="/leaderboard"
                                class="btn btn-secondary"
                                style="margin-top:24px;display:inline-block"
                            >
                                "← Retour au classement"
                            </a>
                        }
                        .into_any();
                    }
                    view! { <span></span> }.into_any()
                } else {
                    // ── Own profile view ──────────────────────────────────────
                    view! { <h2>"Mon profil"</h2> }.into_any()
                }
            }}

            {move || {
                if target_user.get().is_some() {
                    // Already rendered above
                    return view! { <span></span> }.into_any();
                }
                if loading.get() {
                    view! {
                        <p style="color:var(--text-secondary);text-align:center;padding:32px">
                            "Chargement..."
                        </p>
                    }
                    .into_any()
                } else {
                    view! {
                        <form class="profile-form" on:submit=on_save>
                            <div class="form-group">
                                <label>"Nom d'utilisateur"</label>
                                <input
                                    type="text"
                                    disabled=true
                                    prop:value=move || username_val.get()
                                />
                            </div>

                            <div class="form-group">
                                <label>"Email"</label>
                                <input
                                    type="email"
                                    disabled=true
                                    prop:value=move || email.get()
                                />
                            </div>

                            <div class="form-group">
                                <label>"Nom d'affichage"</label>
                                <input
                                    type="text"
                                    maxlength="50"
                                    placeholder="Votre nom d'affichage (optionnel)"
                                    on:input=move |ev| display_name.set(event_target_value(&ev))
                                    prop:value=move || display_name.get()
                                />
                                <span class="field-hint">
                                    "Affiché sur le classement à la place de votre pseudo (50 caractères max)"
                                </span>
                            </div>

                            {move || save_msg.get().map(|(msg, ok)| {
                                if ok {
                                    view! { <div class="field-success">{msg}</div> }.into_any()
                                } else {
                                    view! { <div class="error-message">{msg}</div> }.into_any()
                                }
                            })}

                            <button
                                type="submit"
                                class="btn btn-primary"
                                style="width:100%"
                                disabled=move || saving.get()
                            >
                                {move || if saving.get() { "Sauvegarde..." } else { "Sauvegarder" }}
                            </button>
                        </form>

                        <hr/>

                        <div class="danger-zone">
                            <h3>"Zone de danger"</h3>
                            <p style="color:var(--text-secondary);margin-bottom:16px;font-size:0.9rem">
                                "Vous deconnecter supprimera votre session."
                            </p>
                            <button class="btn btn-danger" on:click=logout>
                                "Se deconnecter"
                            </button>
                        </div>
                    }
                    .into_any()
                }
            }}
        </div>
    }
}
