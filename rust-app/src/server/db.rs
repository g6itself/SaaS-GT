use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL doit etre definie");

    tracing::info!("Connexion a PostgreSQL...");

    PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Impossible de se connecter a la base de donnees")
}
