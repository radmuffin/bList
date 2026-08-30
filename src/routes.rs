use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::db::StorageEngine;
use crate::geocoder::Geocoder;
use crate::models::{
    ApiResponse, CreateListRequest, CreatePinRequest, IngestRequest, List, ListPinsQuery, Pin,
    ScrapedMetadata, UpdateListRequest, UpdatePinRequest,
};
use crate::scraper::Scraper;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn StorageEngine>,
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
    match state.storage.list_lists() {
        Ok(lists) => (StatusCode::OK, Json(ApiResponse::ok(lists))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to fetch lists: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn get_list(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    match state.storage.get_list(id) {
        Ok(Some(list)) => (StatusCode::OK, Json(ApiResponse::ok(list))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("List #{} not found", id))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Database query failed: {}", e))),
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

    match state.storage.create_list(&req) {
        Ok(list) => (StatusCode::CREATED, Json(ApiResponse::ok(list))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to create list: {}", e))),
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

    match state.storage.update_list(id, &req) {
        Ok(Some(list)) => (StatusCode::OK, Json(ApiResponse::ok(list))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("List #{} not found", id))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to update list: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn delete_list(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    match state.storage.delete_list(id) {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::ok(true))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("List #{} not found", id))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to delete list: {}", e))),
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
    match state.storage.list_pins(&query) {
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
    match state.storage.get_pin(id) {
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

    match state.storage.create_pin(&req) {
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
    match state.storage.update_pin(id, &req) {
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
    match state.storage.toggle_visited(id) {
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
    match state.storage.delete_pin(id) {
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
        "instagram" | "tiktok" => "Social".to_string(),
        "google_maps" | "apple_maps" | "tripadvisor" | "yelp" | "alltrails" => "Place".to_string(),
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

    match state.storage.create_pin(&create_req) {
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
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<String>>>) {
    match state.storage.get_categories(query.list_id) {
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

pub async fn export_geojson(
    State(state): State<AppState>,
    Query(query): Query<ListPinsQuery>,
) -> impl IntoResponse {
    let pins = match state.storage.list_pins(&query) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Database error: {}", e) })),
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
    match state.storage.list_pins(&query) {
        Ok(pins) => (StatusCode::OK, Json(ApiResponse::ok(pins))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Export failed: {}", e))),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{InMemoryStorage, SqliteRepository};

    fn setup_test_sqlite_state(db_name: &str) -> AppState {
        let _ = std::fs::remove_file(db_name);
        let storage = Arc::new(SqliteRepository::open(db_name).expect("init sqlite repo"));
        let geocoder = Arc::new(Geocoder::new());
        let scraper = Arc::new(Scraper::with_geocoder(geocoder.clone()));
        AppState {
            storage,
            scraper,
            geocoder,
        }
    }

    fn setup_test_in_memory_state() -> AppState {
        let storage = Arc::new(InMemoryStorage::new());
        let geocoder = Arc::new(Geocoder::new());
        let scraper = Arc::new(Scraper::with_geocoder(geocoder.clone()));
        AppState {
            storage,
            scraper,
            geocoder,
        }
    }

    #[tokio::test]
    async fn test_routes_list_and_pin_flow_sqlite() {
        let db_name = "test_routes.db";
        let state = setup_test_sqlite_state(db_name);

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
        let (status, Json(res)) = export_json(State(state.clone()), Query(query)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 1);

        // 6. Update list
        let update_req = UpdateListRequest {
            name: Some("Paris & Lyon 2026".to_string()),
            icon: Some("🍷".to_string()),
        };
        let (status, Json(res)) =
            update_list(State(state.clone()), Path(new_list.id), Json(update_req)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().name, "Paris & Lyon 2026");

        // 7. Delete list
        let (status, Json(res)) = delete_list(State(state.clone()), Path(new_list.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.unwrap());

        let _ = std::fs::remove_file(db_name);
    }

    #[tokio::test]
    async fn test_routes_in_memory_backend() {
        let state = setup_test_in_memory_state();

        let req = CreateListRequest {
            name: "Kyoto Trip".to_string(),
            icon: Some("⛩️".to_string()),
        };
        let (status, Json(res)) = create_list(State(state.clone()), Json(req)).await;
        assert_eq!(status, StatusCode::CREATED);
        let list = res.data.unwrap();
        assert_eq!(list.name, "Kyoto Trip");

        let pin_req = CreatePinRequest {
            list_id: Some(list.id),
            title: "Fushimi Inari Taisha".to_string(),
            description: Some("Shrine gates".to_string()),
            latitude: 34.9671,
            longitude: 135.7727,
            category: Some("Culture".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Fushimi Ward, Kyoto".to_string()),
            notes: None,
            visited: Some(false),
        };
        let (status, Json(res)) = create_pin(State(state.clone()), Json(pin_req)).await;
        assert_eq!(status, StatusCode::CREATED);
        let pin = res.data.unwrap();

        let (status, Json(res)) = toggle_visited(State(state.clone()), Path(pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.unwrap().visited);
    }
}
