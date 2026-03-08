#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_files::Files;
    use actix_web::*;
    use leptos::config::get_configuration;
    use leptos::prelude::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use leptos_meta::MetaTags;

    use achievement_tracker::app::*;
    use achievement_tracker::server::{api, db};

    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Connexion a la base de donnees
    let pool = db::create_pool().await;

    // Configuration Leptos
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;

    tracing::info!("Demarrage du serveur sur http://{}", &addr);

    HttpServer::new(move || {
        let routes = generate_route_list(App);
        let leptos_options = &conf.leptos_options;
        let site_root = leptos_options.site_root.clone().to_string();

        App::new()
            // En-tetes de securite
            .wrap(
                actix_web::middleware::DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-XSS-Protection", "1; mode=block"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"))
            )
            // Donnees partagees
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(leptos_options.to_owned()))
            // API REST
            .service(
                web::scope("/api")
                    .configure(api::auth::configure)
                    .configure(api::platforms::configure)
                    .configure(api::games::configure)
                    .configure(api::achievements::configure)
                    .configure(api::leaderboard::configure)
                    .configure(api::users::configure),
            )
            // Fichiers statiques Leptos
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            .service(Files::new("/assets", &site_root))
            .service(favicon)
            // Routes Leptos SSR
            .leptos_routes(routes, {
                let leptos_options = leptos_options.clone();
                move || {
                    view! {
                        <!DOCTYPE html>
                        <html lang="fr">
                            <head>
                                <meta charset="utf-8"/>
                                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                                <AutoReload options=leptos_options.clone()/>
                                <HydrationScripts options=leptos_options.clone()/>
                                <MetaTags/>
                            </head>
                            <body>
                                <App/>
                            </body>
                        </html>
                    }
                }
            })
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(feature = "ssr")]
#[actix_web::get("favicon.ico")]
async fn favicon(
    leptos_options: actix_web::web::Data<leptos::config::LeptosOptions>,
) -> actix_web::Result<actix_files::NamedFile> {
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    Ok(actix_files::NamedFile::open(format!(
        "{site_root}/favicon.ico"
    ))?)
}

#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {
    // pas de main client-side
}

#[cfg(all(not(feature = "ssr"), feature = "csr"))]
pub fn main() {
    use achievement_tracker::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
