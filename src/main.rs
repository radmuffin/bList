mod db;
mod geocoder;
mod models;
mod routes;
mod scraper;

use axum::{
    routing::{get, patch, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    let conn = db::init_db(&db_path)?;
    println!("📦 Connected to SQLite database at: {}", db_path);

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        scraper: Arc::new(Scraper::new()),
        geocoder: Arc::new(Geocoder::new()),
    };

    // API Routes
    let api_router = Router::new()
        .route("/pins", get(routes::list_pins).post(routes::create_pin))
        .route(
            "/pins/:id",
            get(routes::get_pin)
                .put(routes::update_pin)
                .delete(routes::delete_pin),
        )
        .route("/pins/:id/visited", patch(routes::toggle_visited))
        .route("/pins/ingest", post(routes::ingest_link))
        .route("/scrape/preview", post(routes::preview_scrape))
        .route("/categories", get(routes::get_categories))
        .route("/geocode", get(routes::geocode))
        .route("/export/geojson", get(routes::export_geojson))
        .route("/export/json", get(routes::export_json));

    // Static Assets & Fallback
    let static_service =
        ServeDir::new("static").fallback(ServeFile::new("static/index.html"));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(static_service)
        .layer(CorsLayer::permissive())
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
