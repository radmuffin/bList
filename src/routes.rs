use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rusqlite::Connection;
use serde_json::json;
use std::sync::{Arc, Mutex};

use crate::db;
use crate::geocoder::Geocoder;
use crate::models::{
    ApiResponse, CreatePinRequest, IngestRequest, ListPinsQuery, Pin, ScrapedMetadata,
    UpdatePinRequest,
};
use crate::scraper::Scraper;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub scraper: Arc<Scraper>,
    pub geocoder: Arc<Geocoder>,
}

#[debug_handler]
pub async fn list_pins(
    State(state): State<AppState>,
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Pin>>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::list_pins(&conn, &query) {
        Ok(pins) => (StatusCode::OK, Json(ApiResponse::ok(pins))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to fetch pins: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn get_pin(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::get_pin(&conn, id) {
        Ok(Some(pin)) => (StatusCode::OK, Json(ApiResponse::ok(pin))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("Pin #{} not found", id))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Database query failed: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn create_pin(
    State(state): State<AppState>,
    Json(req): Json<CreatePinRequest>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    if req.title.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("Title cannot be empty")),
        );
    }

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::create_pin(&conn, &req) {
        Ok(pin) => (StatusCode::CREATED, Json(ApiResponse::ok(pin))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to create pin: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn update_pin(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePinRequest>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::update_pin(&conn, id, &req) {
        Ok(Some(pin)) => (StatusCode::OK, Json(ApiResponse::ok(pin))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("Pin #{} not found", id))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to update pin: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn toggle_visited(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::toggle_visited(&conn, id) {
        Ok(Some(pin)) => (StatusCode::OK, Json(ApiResponse::ok(pin))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("Pin #{} not found", id))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to toggle visited: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn delete_pin(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::delete_pin(&conn, id) {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::ok(true))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("Pin #{} not found", id))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to delete pin: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn ingest_link(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    let raw_url = req.url.trim();
    if raw_url.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("URL cannot be empty")),
        );
    }

    let meta = match state.scraper.scrape_url(raw_url).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(format!("Failed to scrape URL: {}", e))),
            );
        }
    };

    let lat = match meta.latitude {
        Some(l) => l,
        None => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResponse::err(
                    "Could not determine GPS coordinates for this location automatically. Use manual placement or search.",
                )),
            );
        }
    };

    let lon = match meta.longitude {
        Some(l) => l,
        None => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResponse::err(
                    "Could not determine GPS coordinates for this location automatically. Use manual placement or search.",
                )),
            );
        }
    };

    let category = req.category.unwrap_or_else(|| match meta.source_type.as_str() {
        "instagram" => "Social".to_string(),
        "google_maps" | "apple_maps" => "Place".to_string(),
        _ => "General".to_string(),
    });

    let create_req = CreatePinRequest {
        title: meta.title,
        description: meta.description,
        latitude: lat,
        longitude: lon,
        category: Some(category),
        source_url: Some(meta.source_url),
        image_url: meta.image_url,
        address: meta.address,
        notes: req.notes,
        visited: Some(false),
    };

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::create_pin(&conn, &create_req) {
        Ok(pin) => (StatusCode::CREATED, Json(ApiResponse::ok(pin))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to save ingested pin: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn preview_scrape(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> (StatusCode, Json<ApiResponse<ScrapedMetadata>>) {
    let raw_url = req.url.trim();
    if raw_url.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("URL cannot be empty")),
        );
    }

    match state.scraper.scrape_url(raw_url).await {
        Ok(meta) => (StatusCode::OK, Json(ApiResponse::ok(meta))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err(format!("Scraping failed: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn get_categories(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<Vec<String>>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::get_categories(&conn) {
        Ok(cats) => (StatusCode::OK, Json(ApiResponse::ok(cats))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to get categories: {}", e))),
        ),
    }
}

#[derive(serde::Deserialize)]
pub struct GeocodeQuery {
    pub q: String,
}

#[debug_handler]
pub async fn geocode(
    State(state): State<AppState>,
    Query(query): Query<GeocodeQuery>,
) -> (StatusCode, Json<ApiResponse<crate::models::GeoLocation>>) {
    match state.geocoder.geocode(&query.q).await {
        Ok(Some(geo)) => (StatusCode::OK, Json(ApiResponse::ok(geo))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err("Location not found")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(e)),
        ),
    }
}

pub async fn export_geojson(State(state): State<AppState>) -> impl IntoResponse {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Database error: {}", e) })),
            );
        }
    };

    let empty_query = ListPinsQuery {
        category: None,
        visited: None,
        search: None,
    };

    let pins = match db::list_pins(&conn, &empty_query) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to fetch pins: {}", e) })),
            );
        }
    };

    let features: Vec<serde_json::Value> = pins
        .into_iter()
        .map(|pin| {
            json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [pin.longitude, pin.latitude]
                },
                "properties": {
                    "id": pin.id,
                    "title": pin.title,
                    "description": pin.description,
                    "category": pin.category,
                    "source_url": pin.source_url,
                    "image_url": pin.image_url,
                    "address": pin.address,
                    "notes": pin.notes,
                    "visited": pin.visited,
                    "created_at": pin.created_at
                }
            })
        })
        .collect();

    let geojson = json!({
        "type": "FeatureCollection",
        "features": features
    });

    (StatusCode::OK, Json(geojson))
}

#[debug_handler]
pub async fn export_json(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<Vec<Pin>>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database error: {}", e))),
            );
        }
    };

    let empty_query = ListPinsQuery {
        category: None,
        visited: None,
        search: None,
    };

    match db::list_pins(&conn, &empty_query) {
        Ok(pins) => (StatusCode::OK, Json(ApiResponse::ok(pins))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Export failed: {}", e))),
        ),
    }
}
