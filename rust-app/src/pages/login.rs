use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        error.set(None);

        #[cfg(target_arch = "wasm32")]
        {
        let email_val = email.get();
        let password_val = password.get();
        leptos::task::spawn_local(async move {
            let client = reqwest::Client::new();
            let resp = client
                .post("/api/auth/login")
                .json(&serde_json::json!({
                    "email": email_val,
                    "password": password_val,
                }))
                .send()
                .await;

            loading.set(false);

            match resp {
                Ok(r) if r.status().is_success() => {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        if let Some(token) = body["token"].as_str() {
                            let Some(window) = leptos::web_sys::window() else {
                                error.set(Some("Environnement navigateur indisponible".into()));
                                return;
                            };
                            let Ok(Some(storage)) = window.local_storage() else {
                                error.set(Some("Stockage local indisponible".into()));
                                return;
                            };
                            let _ = storage.set_item("auth_token", token);
                            if let Some(username) = body["user"]["username"].as_str() {
                                let _ = storage.set_item("auth_user", username);
                            }
                            let _ = window.location().set_href("/dashboard");
                        } else {
                            error.set(Some("Reponse serveur invalide".into()));
                        }
                    } else {
                        error.set(Some("Reponse serveur invalide".into()));
                    }
                }
                Ok(r) => {
                    let msg = r
                        .json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|b| b["error"].as_str().map(String::from))
                        .unwrap_or_else(|| "Erreur de connexion".into());
                    error.set(Some(msg));
                }
                Err(_) => {
                    error.set(Some("Erreur reseau, veuillez reessayer".into()));
                }
            }
        });
        } // cfg(target_arch = "wasm32")
    };

    view! {
        <div class="auth-page">
            <h2>"Connexion"</h2>

            {move || error.get().map(|e| view! { <div class="error-message">{e}</div> })}

            <form on:submit=on_submit>
                <div class="form-group">
                    <label for="email">"Email"</label>
                    <input
                        type="email"
                        id="email"
                        required=true
                        on:input=move |ev| email.set(event_target_value(&ev))
                        prop:value=move || email.get()
                    />
                </div>

                <div class="form-group">
                    <label for="password">"Mot de passe"</label>
                    <input
                        type="password"
                        id="password"
                        required=true
                        on:input=move |ev| password.set(event_target_value(&ev))
                        prop:value=move || password.get()
                    />
                </div>

                <button type="submit" class="btn btn-primary" disabled=move || loading.get()>
                    {move || if loading.get() { "Connexion..." } else { "Se connecter" }}
                </button>
            </form>

            <p class="auth-link">
                "Pas encore de compte ? "
                <a href="/register">"Creer un compte"</a>
            </p>
        </div>
    }
}
