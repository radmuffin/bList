mod db;
mod geocoder;
mod importer;
mod models;
mod plus_code;
mod routes;
mod scraper;
mod security;

use axum::{
    routing::{get, patch, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::SqliteRepository;
use crate::geocoder::Geocoder;
use crate::routes::AppState;
use crate::scraper::Scraper;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "map_bucket_list=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("🗺️  Initializing Map Bucket List...");

    // Initialize Database
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "pins.db".to_string());
    let storage = Arc::new(SqliteRepository::open(&db_path)?);
    println!("📦 Connected to SQLite database at: {}", db_path);

    let geocoder = Arc::new(Geocoder::new());
    let scraper = Arc::new(Scraper::with_geocoder(geocoder.clone()));

    let state = AppState {
        storage,
        scraper,
        geocoder,
    };

    // API Routes
    let api_router = Router::new()
        .route("/lists", get(routes::list_lists).post(routes::create_list))
        .route("/lists/join", post(routes::join_list))
        .route(
            "/lists/:id",
            get(routes::get_list)
                .put(routes::update_list)
                .delete(routes::delete_list),
        )
        .route("/pins", get(routes::list_pins).post(routes::create_pin))
        .route(
            "/pins/:id",
            get(routes::get_pin)
                .put(routes::update_pin)
                .delete(routes::delete_pin),
        )
        .route("/pins/:id/visited", patch(routes::toggle_visited))
        .route("/pins/save", post(routes::ingest_link))
        .route("/pins/ingest", post(routes::ingest_link))
        .route("/import", post(routes::import_places))
        .route("/scrape/preview", post(routes::preview_scrape))
        .route("/categories", get(routes::get_categories))
        .route("/geocode", get(routes::geocode))
        .route("/health", get(routes::health_check))
        .route("/info", get(routes::app_info))
        .route("/version", get(routes::app_info))
        .route("/export/geojson", get(routes::export_geojson))
        .route("/export/json", get(routes::export_json));

    // Static Assets & Fallback
    let static_service =
        ServeDir::new("static").fallback(ServeFile::new("static/index.html"));

    let app = Router::new()
        .route("/health", get(routes::health_check))
        .nest("/api", api_router)
        .fallback_service(static_service)
        .layer(axum::middleware::from_fn(fly_common::security::set_security_headers))
        .layer(fly_common::security::standard_cors_layer())
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Server listening on http://localhost:{}", port);
    println!("📍 Open http://localhost:{} in your browser to view the interactive map!", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
