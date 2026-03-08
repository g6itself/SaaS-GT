use leptos::prelude::*;

#[derive(Clone, PartialEq, Debug)]
enum UsernameStatus {
    Idle,
    Checking,
    Available,
    Taken,
}

fn validate_email(email: &str) -> bool {
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    true
}

#[cfg(target_arch = "wasm32")]
fn encode_uri_component(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b => {
                encoded.push('%');
                encoded.push_str(&format!("{:02X}", b));
            }
        }
    }
    encoded
}

#[component]
pub fn RegisterPage() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let confirm_pw = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);
    let email_touched = RwSignal::new(false);
    let confirm_touched = RwSignal::new(false);
    let username_status = RwSignal::new(UsernameStatus::Idle);

    let email_valid = Memo::new(move |_| validate_email(&email.get()));
    let pw_len_ok = Memo::new(move |_| password.get().len() >= 12);
    let pw_spec_ok =
        Memo::new(move |_| password.get().chars().any(|c| !c.is_alphanumeric()));
    let pw_valid = Memo::new(move |_| pw_len_ok.get() && pw_spec_ok.get());
    let pw_match =
        Memo::new(move |_| !confirm_pw.get().is_empty() && password.get() == confirm_pw.get());
    let can_submit = Memo::new(move |_| {
        email_valid.get()
            && pw_valid.get()
            && pw_match.get()
            && username.get().trim().len() >= 3
            && username_status.get() == UsernameStatus::Available
            && !loading.get()
    });

    let check_username_avail = move || {
        let uname = username.get();
        let trimmed = uname.trim();
        if trimmed.len() < 3 {
            username_status.set(UsernameStatus::Idle);
            return;
        }
        username_status.set(UsernameStatus::Checking);
        #[cfg(target_arch = "wasm32")]
        {
        let encoded = encode_uri_component(trimmed);
        leptos::task::spawn_local(async move {
            let url = format!("/api/auth/check-username?username={}", encoded);
            let client = reqwest::Client::new();
            match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        let available = body["available"].as_bool().unwrap_or(false);
                        username_status
                            .set(if available { UsernameStatus::Available } else { UsernameStatus::Taken });
                    } else {
                        username_status.set(UsernameStatus::Idle);
                    }
                }
                _ => username_status.set(UsernameStatus::Idle),
            }
        });
        } // cfg(target_arch = "wasm32")
    };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if !can_submit.get() {
            return;
        }
        loading.set(true);
        error.set(None);

        #[cfg(target_arch = "wasm32")]
        {
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
                            let Some(window) = leptos::web_sys::window() else {
                                error.set(Some("Environnement navigateur indisponible".into()));
                                return;
                            };
                            let Ok(Some(storage)) = window.local_storage() else {
                                error.set(Some("Stockage local indisponible".into()));
                                return;
                            };
                            let _ = storage.set_item("auth_token", token);
                            if let Some(uname) = body["user"]["username"].as_str() {
                                let _ = storage.set_item("auth_user", uname);
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
                        .unwrap_or_else(|| "Erreur d'inscription".into());
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
            <h2>"Creer un compte"</h2>

            {move || error.get().map(|e| view! { <div class="error-message">{e}</div> })}

            <form on:submit=on_submit>
                // Email
                <div class="form-group">
                    <label for="reg-email">"Email"</label>
                    <input
                        type="text"
                        id="reg-email"
                        autocomplete="email"
                        class:valid=move || email_touched.get() && email_valid.get()
                        class:invalid=move || email_touched.get() && !email_valid.get()
                        on:input=move |ev| {
                            email_touched.set(true);
                            email.set(event_target_value(&ev));
                        }
                        prop:value=move || email.get()
                    />
                    {move || {
                        if email_touched.get() && !email_valid.get() {
                            view! { <span class="field-error">"Adresse email invalide"</span> }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>

                // Username
                <div class="form-group">
                    <label for="reg-username">"Nom d'utilisateur"</label>
                    <input
                        type="text"
                        id="reg-username"
                        autocomplete="username"
                        maxlength="30"
                        class:valid=move || username_status.get() == UsernameStatus::Available
                        class:invalid=move || username_status.get() == UsernameStatus::Taken
                        on:input=move |ev| {
                            username.set(event_target_value(&ev));
                            username_status.set(UsernameStatus::Idle);
                        }
                        on:blur=move |_| check_username_avail()
                        prop:value=move || username.get()
                    />
                    {move || match username_status.get() {
                        UsernameStatus::Checking => view! { <span class="field-hint">"Verification..."</span> }.into_any(),
                        UsernameStatus::Available => view! { <span class="field-success">"Disponible ✓"</span> }.into_any(),
                        UsernameStatus::Taken => view! { <span class="field-error">"Ce pseudo est deja pris"</span> }.into_any(),
                        UsernameStatus::Idle => {
                            if username.get().trim().len() > 0 && username.get().trim().len() < 3 {
                                view! { <span class="field-error">"Minimum 3 caracteres"</span> }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        }
                    }}
                </div>

                // Password
                <div class="form-group">
                    <label for="reg-password">"Mot de passe"</label>
                    <input
                        type="password"
                        id="reg-password"
                        autocomplete="new-password"
                        class:valid=move || pw_valid.get()
                        class:invalid=move || !password.get().is_empty() && !pw_valid.get()
                        on:input=move |ev| password.set(event_target_value(&ev))
                        prop:value=move || password.get()
                    />
                    {move || {
                        if !password.get().is_empty() {
                            view! {
                                <ul class="password-rules">
                                    <li class:rule-ok=move || pw_len_ok.get() class:rule-fail=move || !pw_len_ok.get()>
                                        "12 caracteres minimum"
                                    </li>
                                    <li class:rule-ok=move || pw_spec_ok.get() class:rule-fail=move || !pw_spec_ok.get()>
                                        "Au moins un caractere special"
                                    </li>
                                </ul>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>

                // Confirm password
                <div class="form-group">
                    <label for="reg-confirm">"Confirmer le mot de passe"</label>
                    <input
                        type="password"
                        id="reg-confirm"
                        autocomplete="new-password"
                        class:valid=move || confirm_touched.get() && pw_match.get()
                        class:invalid=move || confirm_touched.get() && !confirm_pw.get().is_empty() && !pw_match.get()
                        on:input=move |ev| {
                            confirm_touched.set(true);
                            confirm_pw.set(event_target_value(&ev));
                        }
                        prop:value=move || confirm_pw.get()
                    />
                    {move || {
                        if confirm_touched.get() && !confirm_pw.get().is_empty() && !pw_match.get() {
                            view! { <span class="field-error">"Les mots de passe ne correspondent pas"</span> }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>

                <button
                    type="submit"
                    class="btn btn-primary"
                    style="width: 100%; margin-top: 8px;"
                    disabled=move || !can_submit.get()
                >
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
