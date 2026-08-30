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

    fn setup_test_state() -> AppState {
        let conn = db::init_db(":memory:").expect("init in-memory db");
        AppState {
            db: Arc::new(Mutex::new(conn)),
            scraper: Arc::new(Scraper::new()),
            geocoder: Arc::new(Geocoder::new()),
        }
    }

    #[tokio::test]
    async fn test_routes_list_crud_and_validation() {
        let state = setup_test_state();

        // 1. Initial list check (default seeded list)
        let (status, Json(res)) = list_lists(State(state.clone())).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.success);
        let lists = res.data.unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "My Bucket List");

        // 2. Create list validation: empty name
        let empty_req = CreateListRequest {
            name: "   ".to_string(),
            icon: None,
        };
        let (status, Json(res)) = create_list(State(state.clone()), Json(empty_req)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!res.success);
        assert_eq!(res.error.unwrap(), "List name cannot be empty");

        // 3. Create list success
        let req = CreateListRequest {
            name: "Tokyo Trip 2026".to_string(),
            icon: Some("🗼".to_string()),
        };
        let (status, Json(res)) = create_list(State(state.clone()), Json(req)).await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(res.success);
        let new_list = res.data.unwrap();
        assert_eq!(new_list.name, "Tokyo Trip 2026");
        assert_eq!(new_list.icon, "🗼");

        // 4. Get list success
        let (status, Json(res)) = get_list(State(state.clone()), Path(new_list.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().name, "Tokyo Trip 2026");

        // 5. Get list 404 not found
        let (status, Json(res)) = get_list(State(state.clone()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!res.success);

        // 6. Update list validation: empty name
        let invalid_update = UpdateListRequest {
            name: Some("  ".to_string()),
            icon: None,
        };
        let (status, Json(res)) = update_list(State(state.clone()), Path(new_list.id), Json(invalid_update)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!res.success);

        // 7. Update list 404 not found
        let valid_update = UpdateListRequest {
            name: Some("Updated Non-Existent".to_string()),
            icon: None,
        };
        let (status, Json(res)) = update_list(State(state.clone()), Path(99999), Json(valid_update)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!res.success);

        // 8. Update list success
        let update_req = UpdateListRequest {
            name: Some("Tokyo & Kyoto 2026".to_string()),
            icon: Some("🗾".to_string()),
        };
        let (status, Json(res)) = update_list(State(state.clone()), Path(new_list.id), Json(update_req)).await;
        assert_eq!(status, StatusCode::OK);
        let updated = res.data.unwrap();
        assert_eq!(updated.name, "Tokyo & Kyoto 2026");
        assert_eq!(updated.icon, "🗾");

        // 9. Delete list 404 not found
        let (status, Json(res)) = delete_list(State(state.clone()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!res.success);

        // 10. Delete list success
        let (status, Json(res)) = delete_list(State(state.clone()), Path(new_list.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.unwrap());
    }

    #[tokio::test]
    async fn test_routes_pin_crud_and_validation() {
        let state = setup_test_state();

        // 1. Create pin validation: empty title
        let empty_pin_req = CreatePinRequest {
            list_id: Some(1),
            title: "  ".to_string(),
            description: None,
            latitude: 35.6586,
            longitude: 139.7454,
            category: None,
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None,
        };
        let (status, Json(res)) = create_pin(State(state.clone()), Json(empty_pin_req)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(res.error.unwrap(), "Title cannot be empty");

        // 2. Create pin success
        let pin_req = CreatePinRequest {
            list_id: Some(1),
            title: "Tokyo Tower".to_string(),
            description: Some("Famous tower in Tokyo".to_string()),
            latitude: 35.6586,
            longitude: 139.7454,
            category: Some("Sightseeing".to_string()),
            source_url: Some("https://example.com/tokyo-tower".to_string()),
            image_url: Some("https://example.com/tokyo-tower.jpg".to_string()),
            address: Some("Minato City, Tokyo".to_string()),
            notes: Some("Great sunset view".to_string()),
            visited: Some(false),
        };
        let (status, Json(res)) = create_pin(State(state.clone()), Json(pin_req)).await;
        assert_eq!(status, StatusCode::CREATED);
        let pin = res.data.unwrap();
        assert_eq!(pin.title, "Tokyo Tower");
        assert_eq!(pin.category, "Sightseeing");
        assert_eq!(pin.visited, false);

        // 3. Get pin success
        let (status, Json(res)) = get_pin(State(state.clone()), Path(pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().id, pin.id);

        // 4. Get pin 404 not found
        let (status, Json(res)) = get_pin(State(state.clone()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!res.success);

        // 5. Update pin success
        let update_pin_req = UpdatePinRequest {
            list_id: None,
            title: Some("Tokyo Tower Observatory".to_string()),
            description: None,
            latitude: None,
            longitude: None,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: None,
            notes: Some("Night view is amazing".to_string()),
            visited: None,
        };
        let (status, Json(update_res)) = update_pin(State(state.clone()), Path(pin.id), Json(update_pin_req)).await;
        assert_eq!(status, StatusCode::OK);
        let updated = update_res.data.unwrap();
        assert_eq!(updated.title, "Tokyo Tower Observatory");
        assert_eq!(updated.notes, Some("Night view is amazing".to_string()));

        // 6. Update pin 404 not found
        let (status, _res) = update_pin(
            State(state.clone()),
            Path(99999),
            Json(UpdatePinRequest {
                list_id: None,
                title: Some("NonExistent".to_string()),
                description: None,
                latitude: None,
                longitude: None,
                category: None,
                source_url: None,
                image_url: None,
                address: None,
                notes: None,
                visited: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // 7. Toggle visited success
        let (status, Json(res)) = toggle_visited(State(state.clone()), Path(pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().visited, true);

        let (status, Json(res)) = toggle_visited(State(state.clone()), Path(pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().visited, false);

        // 8. Toggle visited 404 not found
        let (status, _res) = toggle_visited(State(state.clone()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // 9. Delete pin success
        let (status, Json(del_res)) = delete_pin(State(state.clone()), Path(pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(del_res.data.unwrap());

        // 10. Delete pin 404 not found
        let (status, _res) = delete_pin(State(state.clone()), Path(pin.id)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_routes_pin_query_filters_and_search() {
        let state = setup_test_state();

        // Seed 3 pins
        let pin1_req = CreatePinRequest {
            list_id: Some(1),
            title: "Sushi Dai".to_string(),
            description: Some("Fresh tuna and sushi sets".to_string()),
            latitude: 35.65,
            longitude: 139.75,
            category: Some("Food & Drink".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Toyosu Market, Tokyo".to_string()),
            notes: Some("Arrive early morning".to_string()),
            visited: Some(false),
        };
        let _ = create_pin(State(state.clone()), Json(pin1_req)).await;

        let pin2_req = CreatePinRequest {
            list_id: Some(1),
            title: "Senso-ji Temple".to_string(),
            description: Some("Ancient Buddhist temple in Asakusa".to_string()),
            latitude: 35.7148,
            longitude: 139.7967,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Asakusa, Taito City, Tokyo".to_string()),
            notes: Some("Walk through Kaminarimon gate".to_string()),
            visited: Some(true),
        };
        let _ = create_pin(State(state.clone()), Json(pin2_req)).await;

        let pin3_req = CreatePinRequest {
            list_id: Some(1),
            title: "Fuglen Tokyo".to_string(),
            description: Some("Scandinavian coffee bar".to_string()),
            latitude: 35.6644,
            longitude: 139.6917,
            category: Some("Cafe".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Shibuya City, Tokyo".to_string()),
            notes: Some("Great pour-over coffee".to_string()),
            visited: Some(true),
        };
        let _ = create_pin(State(state.clone()), Json(pin3_req)).await;

        // Test category filter: Food & Drink
        let (status, Json(res)) = list_pins(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: Some("Food & Drink".to_string()),
                visited: None,
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let pins = res.data.unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].title, "Sushi Dai");

        // Test category filter: All
        let (status, Json(res)) = list_pins(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: Some("All".to_string()),
                visited: None,
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 3);

        // Test visited filter: true
        let (status, Json(res)) = list_pins(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: Some(true),
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 2);

        // Test visited filter: false
        let (status, Json(res)) = list_pins(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: Some(false),
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 1);

        // Test search filter: notes search ("pour-over")
        let (status, Json(res)) = list_pins(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: None,
                search: Some("pour-over".to_string()),
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let pins = res.data.unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].title, "Fuglen Tokyo");

        // Test search filter: address search ("Shibuya")
        let (status, Json(res)) = list_pins(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: None,
                search: Some("Shibuya".to_string()),
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 1);

        // Test search filter: no match
        let (status, Json(res)) = list_pins(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: None,
                search: Some("NonExistentQuery".to_string()),
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 0);

        // Test get_categories route
        let (status, Json(res)) = get_categories(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: None,
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let cats = res.data.unwrap();
        assert_eq!(cats, vec!["Cafe", "Food & Drink", "Sightseeing"]);
    }

    #[tokio::test]
    async fn test_routes_validation_and_exports() {
        let state = setup_test_state();

        // 1. Ingest link validation: empty url
        let (status, Json(res)) = ingest_link(
            State(state.clone()),
            Json(IngestRequest {
                url: "  ".to_string(),
                list_id: None,
                category: None,
                notes: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(res.error.unwrap(), "URL cannot be empty");

        // 2. Preview scrape validation: empty url
        let (status, Json(res)) = preview_scrape(
            State(state.clone()),
            Json(IngestRequest {
                url: "".to_string(),
                list_id: None,
                category: None,
                notes: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(res.error.unwrap(), "URL cannot be empty");

        // 3. Create a pin for export verification
        let _ = create_pin(
            State(state.clone()),
            Json(CreatePinRequest {
                list_id: Some(1),
                title: "Golden Gate Bridge".to_string(),
                description: Some("Suspension bridge".to_string()),
                latitude: 37.8199,
                longitude: -122.4783,
                category: Some("Sightseeing".to_string()),
                source_url: None,
                image_url: None,
                address: Some("San Francisco, CA".to_string()),
                notes: None,
                visited: Some(true),
            }),
        )
        .await;

        // 4. Export JSON
        let (status, Json(res)) = export_json(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: None,
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let pins = res.data.unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].title, "Golden Gate Bridge");

        // 5. Export GeoJSON
        let response = export_geojson(
            State(state.clone()),
            Query(ListPinsQuery {
                list_id: None,
                category: None,
                visited: None,
                search: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // 6. Geocode empty query
        let (status, Json(res)) = geocode(
            State(state.clone()),
            Query(GeocodeQuery {
                q: "   ".to_string(),
            }),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(res.error.unwrap(), "Location not found");
    }
}
