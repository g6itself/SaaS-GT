use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment, WildcardSegment,
};

use crate::pages::{
    dashboard::DashboardPage,
    game_detail::GameDetailPage,
    landing::LandingPage,
    login::LoginPage,
    platform_connections::PlatformConnectionsPage,
    profile::ProfilePage,
    register::RegisterPage,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/achievement-tracker.css"/>
        <Title text="Achievement Tracker"/>

        <Router>
            <main>
                <Routes fallback=move || "Page non trouvee.">
                    <Route path=StaticSegment("") view=LandingPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <Route path=StaticSegment("register") view=RegisterPage/>
                    <Route path=StaticSegment("dashboard") view=DashboardPage/>
                    <Route path=(StaticSegment("games"), WildcardSegment("id")) view=GameDetailPage/>
                    <Route path=StaticSegment("platforms") view=PlatformConnectionsPage/>
                    <Route path=StaticSegment("profile") view=ProfilePage/>
                    <Route path=WildcardSegment("any") view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        let resp = leptos::context::expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! {
        <h1>"404 - Page non trouvee"</h1>
        <p>"La page que vous recherchez n'existe pas."</p>
        <a href="/">"Retour a l'accueil"</a>
    }
}
