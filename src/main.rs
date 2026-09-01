mod db;
mod geocoder;
mod importer;
mod metrics;
mod models;
mod plus_code;
mod routes;
mod scraper;
mod security;

use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;
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
    let metrics_state = Arc::new(metrics::MetricsState::new());

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
        .route(
            "/lists/:id/collaborators",
            get(routes::get_list_collaborators),
        )
        .route(
            "/user/profile",
            get(routes::get_profile).put(routes::update_profile),
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
        .route("/version", get(routes::app_info))
        .route("/export/geojson", get(routes::export_geojson))
        .route("/export/json", get(routes::export_json))
        // Metrics route added to api_router without breaking FlyServer::builder pattern
        // Note: the task says "Expose via GET /metrics endpoint in bList's main.rs" but then says 
        // "add the metrics route to the api_router without breaking this pattern".
        // Let's add it to api_router as /metrics, or rather, if we add it to api_router it becomes /api/metrics.
        // Wait, the prompt says "The src/main.rs currently uses fly_common::server::FlyServer::builder() pattern — add the metrics route to the api_router without breaking this pattern" but also says "Expose via GET /metrics endpoint".
        // Let's actually add it outside the nest, on app_router.
        ;

    // Mount application routes into FlyServer with state and static SPA fallback
    let app_router = Router::new()
        .route(
            "/metrics",
            get(metrics::metrics_handler).with_state(metrics_state.clone()),
        )
        .nest("/api", api_router)
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            metrics_state.clone(),
            metrics::track_metrics,
        ));

    fly_common::server::FlyServer::builder()
        .with_app_info("bList", "0.1.0")
        .with_static_dir("static")
        .with_routes(app_router)
        .serve()
        .await?;

    Ok(())
}
