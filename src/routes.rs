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
    ApiResponse, CreateListRequest, CreatePinRequest, IngestRequest, List, ListPinsQuery, Pin,
    ScrapedMetadata, UpdateListRequest, UpdatePinRequest,
};
use crate::scraper::Scraper;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub scraper: Arc<Scraper>,
    pub geocoder: Arc<Geocoder>,
}

// ---------------------------------------------------------------------------
// List Handlers
// ---------------------------------------------------------------------------

#[debug_handler]
pub async fn list_lists(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<Vec<List>>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::list_lists(&conn) {
        Ok(lists) => (StatusCode::OK, Json(ApiResponse::ok(lists))),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
        ),
    }
}

#[debug_handler]
pub async fn get_list(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::get_list(&conn, id) {
        Ok(Some(list)) => (StatusCode::OK, Json(ApiResponse::ok(list))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("List #{} not found", id))),
        ),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
        ),
    }
}

#[debug_handler]
pub async fn create_list(
    State(state): State<AppState>,
    Json(req): Json<CreateListRequest>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("List name cannot be empty")),
        );
    }

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::create_list(&conn, &req) {
        Ok(list) => (StatusCode::CREATED, Json(ApiResponse::ok(list))),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
        ),
    }
}

#[debug_handler]
pub async fn update_list(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateListRequest>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err("List name cannot be empty")),
            );
        }
    }

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::update_list(&conn, id, &req) {
        Ok(Some(list)) => (StatusCode::OK, Json(ApiResponse::ok(list))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("List #{} not found", id))),
        ),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
        ),
    }
}

#[debug_handler]
pub async fn delete_list(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::delete_list(&conn, id) {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::ok(true))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("List #{} not found", id))),
        ),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
        ),
    }
}

// ---------------------------------------------------------------------------
// Pin Handlers
// ---------------------------------------------------------------------------

#[debug_handler]
pub async fn list_pins(
    State(state): State<AppState>,
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Pin>>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::list_pins(&conn, &query) {
        Ok(pins) => (StatusCode::OK, Json(ApiResponse::ok(pins))),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
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
                StatusCode::SERVICE_UNAVAILABLE,
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
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
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
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::create_pin(&conn, &req) {
        Ok(pin) => (StatusCode::CREATED, Json(ApiResponse::ok(pin))),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
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
                StatusCode::SERVICE_UNAVAILABLE,
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
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
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
                StatusCode::SERVICE_UNAVAILABLE,
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
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
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
                StatusCode::SERVICE_UNAVAILABLE,
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
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
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
        list_id: req.list_id.or(Some(1)),
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
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::create_pin(&conn, &create_req) {
        Ok(pin) => (StatusCode::CREATED, Json(ApiResponse::ok(pin))),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(format!("Failed to save ingested pin: {}", db::map_rusqlite_error(&e)))),
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
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<String>>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::get_categories(&conn, query.list_id) {
        Ok(cats) => (StatusCode::OK, Json(ApiResponse::ok(cats))),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
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

pub async fn export_geojson(
    State(state): State<AppState>,
    Query(query): Query<ListPinsQuery>,
) -> impl IntoResponse {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": format!("Database lock error: {}", e) })),
            );
        }
    };

    let pins = match db::list_pins(&conn, &query) {
        Ok(p) => p,
        Err(e) => {
            return (
                db::map_status_code(&e),
                Json(json!({ "error": db::map_rusqlite_error(&e) })),
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
                    "list_id": pin.list_id,
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
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Pin>>>) {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::err(format!("Database lock error: {}", e))),
            );
        }
    };

    match db::list_pins(&conn, &query) {
        Ok(pins) => (StatusCode::OK, Json(ApiResponse::ok(pins))),
        Err(e) => (
            db::map_status_code(&e),
            Json(ApiResponse::err(db::map_rusqlite_error(&e))),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_state() -> (AppState, db::TestDbGuard) {
        let guard = db::TestDbGuard::new("routes");
        let conn = db::init_db(&guard.path).expect("init db");
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
            scraper: Arc::new(Scraper::new()),
            geocoder: Arc::new(Geocoder::new()),
        };
        (state, guard)
    }

    #[tokio::test]
    async fn test_routes_list_and_pin_flow() {
        let (state, _guard) = setup_test_state();

        // 1. List lists
        let (status, Json(res)) = list_lists(State(state.clone())).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.success);
        let lists = res.data.unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "My Bucket List");

        // 2. Create list
        let req = CreateListRequest {
            name: "Paris 2026".to_string(),
            icon: Some("🥐".to_string()),
        };
        let (status, Json(res)) = create_list(State(state.clone()), Json(req)).await;
        assert_eq!(status, StatusCode::CREATED);
        let new_list = res.data.unwrap();
        assert_eq!(new_list.name, "Paris 2026");
        assert_eq!(new_list.icon, "🥐");

        // 3. Create pin in new list
        let pin_req = CreatePinRequest {
            list_id: Some(new_list.id),
            title: "Eiffel Tower".to_string(),
            description: Some("Famous landmark".to_string()),
            latitude: 48.8584,
            longitude: 2.2945,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Champ de Mars, Paris".to_string()),
            notes: None,
            visited: Some(false),
        };
        let (status, Json(res)) = create_pin(State(state.clone()), Json(pin_req)).await;
        assert_eq!(status, StatusCode::CREATED);
        let pin = res.data.unwrap();
        assert_eq!(pin.list_id, new_list.id);
        assert_eq!(pin.title, "Eiffel Tower");

        // 4. Query pins filtered by list_id
        let query = ListPinsQuery {
            list_id: Some(new_list.id),
            category: None,
            visited: None,
            search: None,
        };
        let (status, Json(res)) = list_pins(State(state.clone()), Query(query.clone())).await;
        assert_eq!(status, StatusCode::OK);
        let pins = res.data.unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].title, "Eiffel Tower");

        // 5. Export JSON filtered by list_id
        let (status, Json(res)) = export_json(State(state.clone()), Query(query.clone())).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 1);

        // 6. Update list
        let update_req = UpdateListRequest {
            name: Some("Paris & Lyon 2026".to_string()),
            icon: Some("🍷".to_string()),
        };
        let (status, Json(res)) = update_list(State(state.clone()), Path(new_list.id), Json(update_req)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().name, "Paris & Lyon 2026");

        // 7. Delete list
        let (status, Json(res)) = delete_list(State(state.clone()), Path(new_list.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.unwrap());
    }
}
