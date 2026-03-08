use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(serde::Serialize, sqlx::FromRow)]
struct RetroConsole {
    id: i32,
    name: String,
    manufacturer: Option<String>,
}

#[derive(sqlx::FromRow)]
struct RetroGameRow {
    id: i32,
    name: String,
    description: Option<String>,
    release_date: Option<chrono::NaiveDate>,
}

#[derive(sqlx::FromRow)]
struct CoverRow {
    game_id: i32,
    region: String,
    url: String,
}

#[derive(serde::Serialize)]
struct CoverInfo {
    region: String,
    url: String,
}

#[derive(serde::Serialize)]
struct RetroGameWithCovers {
    id: i32,
    name: String,
    description: Option<String>,
    release_date: Option<chrono::NaiveDate>,
    covers: Vec<CoverInfo>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/retro-consoles")
            .route("", web::get().to(list_retro_consoles))
            .route("/{id}/games", web::get().to(list_console_games)),
    );
}

async fn list_retro_consoles(pool: web::Data<PgPool>) -> HttpResponse {
    let result = sqlx::query_as::<_, RetroConsole>(
        "SELECT id, name, manufacturer FROM retro_consoles ORDER BY name",
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(consoles) => HttpResponse::Ok().json(consoles),
        Err(e) => {
            eprintln!("Error fetching retro consoles: {e}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur serveur"}))
        }
    }
}

async fn list_console_games(
    pool: web::Data<PgPool>,
    path: web::Path<i32>,
) -> HttpResponse {
    let console_id = path.into_inner();

    let games = sqlx::query_as::<_, RetroGameRow>(
        "SELECT id, name, description, release_date
         FROM retro_games
         WHERE console_id = $1
         ORDER BY name",
    )
    .bind(console_id)
    .fetch_all(pool.get_ref())
    .await;

    let games = match games {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error fetching retro games: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur serveur"}));
        }
    };

    if games.is_empty() {
        return HttpResponse::Ok().json(Vec::<RetroGameWithCovers>::new());
    }

    let game_ids: Vec<i32> = games.iter().map(|g| g.id).collect();

    let covers = sqlx::query_as::<_, CoverRow>(
        "SELECT game_id, region, url
         FROM retro_game_covers
         WHERE game_id = ANY($1)
         ORDER BY game_id, region",
    )
    .bind(&game_ids)
    .fetch_all(pool.get_ref())
    .await;

    let covers = match covers {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error fetching covers: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur serveur"}));
        }
    };

    let mut cover_map: HashMap<i32, Vec<CoverInfo>> = HashMap::new();
    for c in covers {
        cover_map.entry(c.game_id).or_default().push(CoverInfo {
            region: c.region,
            url: c.url,
        });
    }

    let result: Vec<RetroGameWithCovers> = games
        .into_iter()
        .map(|g| {
            let covers = cover_map.remove(&g.id).unwrap_or_default();
            RetroGameWithCovers {
                id: g.id,
                name: g.name,
                description: g.description,
                release_date: g.release_date,
                covers,
            }
        })
        .collect();

    HttpResponse::Ok().json(result)
}