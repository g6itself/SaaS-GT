use leptos::prelude::*;

#[component]
pub fn RegisterPage() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        error.set(None);

        let email_val = email.get();
        let username_val = username.get();
        let password_val = password.get();

        leptos::task::spawn_local(async move {
            let client = reqwest::Client::new();
            let resp = client
                .post("/api/auth/register")
                .json(&serde_json::json!({
                    "email": email_val,
                    "username": username_val,
                    "password": password_val,
                }))
                .send()
                .await;

            loading.set(false);

            match resp {
                Ok(r) if r.status().is_success() => {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        if let Some(token) = body["token"].as_str() {
                            let window = web_sys::window().unwrap();
                            let storage = window.local_storage().unwrap().unwrap();
                            let _ = storage.set_item("auth_token", token);
                            let _ = window.location().set_href("/dashboard");
                        }
                    }
                }
                Ok(r) => {
                    let body = r.json::<serde_json::Value>().await.ok();
                    let msg = body
                        .and_then(|b| b["error"].as_str().map(String::from))
                        .unwrap_or_else(|| "Erreur d'inscription".into());
                    error.set(Some(msg));
                }
                Err(e) => {
                    error.set(Some(format!("Erreur reseau: {}", e)));
                }
            }
        });
    };

    view! {
        <div class="auth-page">
            <h2>"Creer un compte"</h2>

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
                    <label for="username">"Nom d'utilisateur"</label>
                    <input
                        type="text"
                        id="username"
                        required=true
                        on:input=move |ev| username.set(event_target_value(&ev))
                        prop:value=move || username.get()
                    />
                </div>

                <div class="form-group">
                    <label for="password">"Mot de passe"</label>
                    <input
                        type="password"
                        id="password"
                        required=true
                        minlength="8"
                        on:input=move |ev| password.set(event_target_value(&ev))
                        prop:value=move || password.get()
                    />
                </div>

                <button type="submit" class="btn btn-primary" disabled=move || loading.get()>
                    {move || if loading.get() { "Inscription..." } else { "S'inscrire" }}
                </button>
            </form>

            <p class="auth-link">
                "Deja un compte ? "
                <a href="/login">"Se connecter"</a>
            </p>
        </div>
    }
}
