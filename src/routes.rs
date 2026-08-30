use axum::{
    debug_handler,
    extract::{Path, Query, State, FromRequestParts},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum::http::request::Parts;
use serde_json::json;
use std::sync::Arc;

use crate::db::StorageEngine;
use crate::geocoder::Geocoder;
use crate::models::{
    ApiResponse, CreateListRequest, CreatePinRequest, IngestRequest, List, ListPinsQuery, Pin,
    ScrapedMetadata, UpdateListRequest, UpdatePinRequest, JoinListRequest,
};
use crate::scraper::Scraper;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn StorageEngine>,
    pub scraper: Arc<Scraper>,
    pub geocoder: Arc<Geocoder>,
}

pub struct UserToken(pub String);

#[axum::async_trait]
impl FromRequestParts<AppState> for UserToken {
    type Rejection = (StatusCode, Json<ApiResponse<()>>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts.headers
            .get("x-user-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                parts.uri.query().and_then(|q| {
                    q.split('&')
                        .find(|p| p.starts_with("user_token="))
                        .and_then(|p| p.split('=').nth(1))
                        .map(|v| urlencoding::decode(v).unwrap_or(std::borrow::Cow::Borrowed(v)).into_owned())
                })
            });

        let token = match token {
            Some(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err("Missing or empty X-User-Token header")),
            )),
        };

        if let Err(e) = state.storage.auto_associate_device(&token) {
            tracing::error!("Failed to auto-associate device: {}", e);
        }

        Ok(UserToken(token))
    }
}

fn check_permission_or_err<T>(
    storage: &Arc<dyn StorageEngine>,
    user_token: &str,
    list_id: i64,
) -> Result<(), (StatusCode, Json<ApiResponse<T>>)> {
    match storage.get_list(list_id) {
        Ok(Some(_)) => {
            match storage.check_permission(user_token, list_id) {
                Ok(true) => Ok(()),
                Ok(false) => Err((
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::err("Forbidden: Access denied to this list")),
                )),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::err(format!("Database error: {}", e))),
                )),
            }
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("List #{} not found", list_id))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Database error: {}", e))),
        )),
    }
}

fn check_pin_permission_or_err<T>(
    storage: &Arc<dyn StorageEngine>,
    user_token: &str,
    pin_id: i64,
) -> Result<Pin, (StatusCode, Json<ApiResponse<T>>)> {
    match storage.get_pin(pin_id) {
        Ok(Some(pin)) => {
            match storage.check_permission(user_token, pin.list_id) {
                Ok(true) => Ok(pin),
                Ok(false) => Err((
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::err("Forbidden: Access denied to this list")),
                )),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::err(format!("Database error: {}", e))),
                )),
            }
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err(format!("Pin #{} not found", pin_id))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Database error: {}", e))),
        )),
    }
}

// ---------------------------------------------------------------------------
// List Handlers
// ---------------------------------------------------------------------------

#[debug_handler]
pub async fn list_lists(
    State(state): State<AppState>,
    user_token: UserToken,
) -> (StatusCode, Json<ApiResponse<Vec<List>>>) {
    match state.storage.list_lists(&user_token.0) {
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
    user_token: UserToken,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, id) {
        return err;
    }
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
    user_token: UserToken,
    Json(req): Json<CreateListRequest>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("List name cannot be empty")),
        );
    }

    match state.storage.create_list(&req, &user_token.0) {
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
    user_token: UserToken,
    Path(id): Path<i64>,
    Json(req): Json<UpdateListRequest>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, id) {
        return err;
    }
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
    user_token: UserToken,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, id) {
        return err;
    }
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

#[debug_handler]
pub async fn join_list(
    State(state): State<AppState>,
    user_token: UserToken,
    Json(req): Json<JoinListRequest>,
) -> (StatusCode, Json<ApiResponse<List>>) {
    if req.share_token.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("Share token cannot be empty")),
        );
    }
    match state.storage.join_list(req.share_token.trim(), &user_token.0) {
        Ok(Some(list)) => (StatusCode::OK, Json(ApiResponse::ok(list))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err("List not found for the given share token")),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to join list: {}", e))),
        ),
    }
}

// ---------------------------------------------------------------------------
// Pin Handlers
// ---------------------------------------------------------------------------

#[debug_handler]
pub async fn list_pins(
    State(state): State<AppState>,
    user_token: UserToken,
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Pin>>>) {
    if let Some(list_id) = query.list_id {
        if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, list_id) {
            return err;
        }
    }
    match state.storage.list_pins(&query, &user_token.0) {
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
    user_token: UserToken,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    match check_pin_permission_or_err(&state.storage, &user_token.0, id) {
        Ok(pin) => (StatusCode::OK, Json(ApiResponse::ok(pin))),
        Err(err) => err,
    }
}

#[debug_handler]
pub async fn create_pin(
    State(state): State<AppState>,
    user_token: UserToken,
    Json(req): Json<CreatePinRequest>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    if req.title.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("Title cannot be empty")),
        );
    }

    if !req.latitude.is_finite()
        || !req.longitude.is_finite()
        || !(-90.0..=90.0).contains(&req.latitude)
        || !(-180.0..=180.0).contains(&req.longitude)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err(
                "Invalid GPS coordinates: latitude must be [-90, 90] and longitude [-180, 180]",
            )),
        );
    }

    let list_id = req.list_id.unwrap_or(1);
    if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, list_id) {
        return err;
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
    user_token: UserToken,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePinRequest>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    let pin = match check_pin_permission_or_err(&state.storage, &user_token.0, id) {
        Ok(p) => p,
        Err(err) => return err,
    };

    if let Some(new_list_id) = req.list_id {
        if new_list_id != pin.list_id {
            if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, new_list_id) {
                return err;
            }
        }
    }

    if let Some(lat) = req.latitude {
        if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(
                    "Invalid latitude: must be between -90 and 90",
                )),
            );
        }
    }

    if let Some(lon) = req.longitude {
        if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(
                    "Invalid longitude: must be between -180 and 180",
                )),
            );
        }
    }

    match state.storage.update_pin(id, &req) {
        Ok(Some(updated)) => (StatusCode::OK, Json(ApiResponse::ok(updated))),
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
    user_token: UserToken,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    if let Err(err) = check_pin_permission_or_err::<Pin>(&state.storage, &user_token.0, id) {
        return err;
    }
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
    user_token: UserToken,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    if let Err(err) = check_pin_permission_or_err::<bool>(&state.storage, &user_token.0, id) {
        return err;
    }
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
    user_token: UserToken,
    Json(req): Json<IngestRequest>,
) -> (StatusCode, Json<ApiResponse<Pin>>) {
    let raw_url = req.url.trim();
    if raw_url.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("URL cannot be empty")),
        );
    }

    let list_id = req.list_id.unwrap_or(1);
    if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, list_id) {
        return err;
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
        list_id: Some(list_id),
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
    user_token: UserToken,
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<String>>>) {
    if let Some(list_id) = query.list_id {
        if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, list_id) {
            return err;
        }
    }
    match state.storage.get_categories(query.list_id, &user_token.0) {
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
    user_token: UserToken,
    Query(query): Query<ListPinsQuery>,
) -> impl IntoResponse {
    if let Some(list_id) = query.list_id {
        if let Err(err) = check_permission_or_err::<serde_json::Value>(&state.storage, &user_token.0, list_id) {
            return err.into_response();
        }
    }
    let pins = match state.storage.list_pins(&query, &user_token.0) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Database error: {}", e) })),
            ).into_response();
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

    (StatusCode::OK, Json(geojson)).into_response()
}

#[debug_handler]
pub async fn export_json(
    State(state): State<AppState>,
    user_token: UserToken,
    Query(query): Query<ListPinsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Pin>>>) {
    if let Some(list_id) = query.list_id {
        if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, list_id) {
            return err;
        }
    }
    match state.storage.list_pins(&query, &user_token.0) {
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

    fn setup_test_sqlite_state() -> AppState {
        let storage = Arc::new(SqliteRepository::open(":memory:").expect("init sqlite repo"));
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
    async fn test_routes_list_crud_and_validation() {
        let state = setup_test_sqlite_state();

        // List initial seeded lists
        let (status, Json(res)) = list_lists(State(state.clone()), UserToken("test-token".to_string())).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.success);
        let lists = res.data.unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "My Bucket List");

        // Create new list
        let create_req = CreateListRequest {
            name: "Euro Summer".to_string(),
            icon: Some("🏖️".to_string()),
        };
        let (status, Json(res)) = create_list(State(state.clone()), UserToken("test-token".to_string()), Json(create_req)).await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(res.success);
        let created_list = res.data.unwrap();
        assert_eq!(created_list.name, "Euro Summer");
        assert_eq!(created_list.icon, "🏖️");

        // Reject empty list name
        let empty_req = CreateListRequest {
            name: "   ".to_string(),
            icon: None,
        };
        let (status, Json(res)) = create_list(State(state.clone()), UserToken("test-token".to_string()), Json(empty_req)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!res.success);
        assert_eq!(res.error.unwrap(), "List name cannot be empty");

        // Get created list
        let (status, Json(res)) = get_list(State(state.clone()), UserToken("test-token".to_string()), Path(created_list.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().name, "Euro Summer");

        // Get non-existent list
        let (status, Json(res)) = get_list(State(state.clone()), UserToken("test-token".to_string()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!res.success);

        // Update list
        let update_req = UpdateListRequest {
            name: Some("Euro Trip 2026".to_string()),
            icon: Some("✈️".to_string()),
        };
        let (status, Json(res)) =
            update_list(State(state.clone()), UserToken("test-token".to_string()), Path(created_list.id), Json(update_req)).await;
        assert_eq!(status, StatusCode::OK);
        let updated_list = res.data.unwrap();
        assert_eq!(updated_list.name, "Euro Trip 2026");
        assert_eq!(updated_list.icon, "✈️");

        // Update list with empty name should fail
        let invalid_update = UpdateListRequest {
            name: Some("  ".to_string()),
            icon: None,
        };
        let (status, Json(res)) =
            update_list(State(state.clone()), UserToken("test-token".to_string()), Path(created_list.id), Json(invalid_update)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(res.error.unwrap(), "List name cannot be empty");

        // Delete list
        let (status, Json(res)) = delete_list(State(state.clone()), UserToken("test-token".to_string()), Path(created_list.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.unwrap());

        // Delete non-existent list
        let (status, Json(res)) = delete_list(State(state.clone()), UserToken("test-token".to_string()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!res.success);
    }

    #[tokio::test]
    async fn test_routes_pin_crud_and_validation() {
        let state = setup_test_sqlite_state();

        // Create pin
        let pin_req = CreatePinRequest {
            list_id: Some(1),
            title: "Colosseum".to_string(),
            description: Some("Ancient Roman amphitheatre".to_string()),
            latitude: 41.8902,
            longitude: 12.4922,
            category: Some("History".to_string()),
            source_url: Some("https://example.com/colosseum".to_string()),
            image_url: Some("https://example.com/colosseum.jpg".to_string()),
            address: Some("Piazza del Colosseo, 1, Roma".to_string()),
            notes: Some("Book tickets early".to_string()),
            visited: Some(false),
        };
        let (status, Json(res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(pin_req)).await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(res.success);
        let created_pin = res.data.unwrap();
        assert_eq!(created_pin.title, "Colosseum");
        assert_eq!(created_pin.category, "History");
        assert_eq!(created_pin.visited, false);

        // Reject empty title
        let empty_title_req = CreatePinRequest {
            list_id: Some(1),
            title: "   ".to_string(),
            description: None,
            latitude: 41.8902,
            longitude: 12.4922,
            category: None,
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None,
        };
        let (status, Json(res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(empty_title_req)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(res.error.unwrap(), "Title cannot be empty");

        // Get pin
        let (status, Json(res)) = get_pin(State(state.clone()), UserToken("test-token".to_string()), Path(created_pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().title, "Colosseum");

        // Get non-existent pin
        let (status, Json(_res)) = get_pin(State(state.clone()), UserToken("test-token".to_string()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // Update pin
        let update_req = UpdatePinRequest {
            list_id: None,
            title: Some("Flavian Amphitheatre (Colosseum)".to_string()),
            description: None,
            latitude: None,
            longitude: None,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: None,
            notes: Some("Night tour booked".to_string()),
            visited: None,
        };
        let (status, Json(res)) =
            update_pin(State(state.clone()), UserToken("test-token".to_string()), Path(created_pin.id), Json(update_req)).await;
        assert_eq!(status, StatusCode::OK);
        let updated_pin = res.data.unwrap();
        assert_eq!(updated_pin.title, "Flavian Amphitheatre (Colosseum)");
        assert_eq!(updated_pin.category, "Sightseeing");
        assert_eq!(updated_pin.notes, Some("Night tour booked".to_string()));

        // Toggle visited
        let (status, Json(res)) = toggle_visited(State(state.clone()), UserToken("test-token".to_string()), Path(created_pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().visited, true);

        let (status, Json(res)) = toggle_visited(State(state.clone()), UserToken("test-token".to_string()), Path(created_pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().visited, false);

        // Delete pin
        let (status, Json(res)) = delete_pin(State(state.clone()), UserToken("test-token".to_string()), Path(created_pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.unwrap());

        // Delete non-existent pin
        let (status, Json(_res)) = delete_pin(State(state.clone()), UserToken("test-token".to_string()), Path(99999)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_routes_pin_query_filters_and_search() {
        let state = setup_test_sqlite_state();

        let pins = vec![
            ("Sagrada Familia", "Sightseeing", 41.4036, 2.1743, true),
            ("Park Guell", "Sightseeing", 41.4145, 2.1527, false),
            ("El Xampanyet", "Food & Drink", 41.3847, 2.1818, true),
            ("Bar del Pla", "Food & Drink", 41.3854, 2.1794, false),
        ];

        for (title, cat, lat, lon, visited) in pins {
            let req = CreatePinRequest {
                list_id: Some(1),
                title: title.to_string(),
                description: Some(format!("Info for {}", title)),
                latitude: lat,
                longitude: lon,
                category: Some(cat.to_string()),
                source_url: None,
                image_url: None,
                address: Some("Barcelona, Spain".to_string()),
                notes: None,
                visited: Some(visited),
            };
            let _ = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(req)).await;
        }

        // Filter by category
        let query = ListPinsQuery {
            list_id: Some(1),
            category: Some("Sightseeing".to_string()),
            visited: None,
            search: None,
        };
        let (status, Json(res)) = list_pins(State(state.clone()), UserToken("test-token".to_string()), Query(query)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 2);

        // Filter by visited
        let query = ListPinsQuery {
            list_id: Some(1),
            category: None,
            visited: Some(true),
            search: None,
        };
        let (status, Json(res)) = list_pins(State(state.clone()), UserToken("test-token".to_string()), Query(query)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res.data.unwrap().len(), 2);

        // Search keyword
        let query = ListPinsQuery {
            list_id: Some(1),
            category: None,
            visited: None,
            search: Some("Sagrada".to_string()),
        };
        let (status, Json(res)) = list_pins(State(state.clone()), UserToken("test-token".to_string()), Query(query)).await;
        assert_eq!(status, StatusCode::OK);
        let found = res.data.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Sagrada Familia");
    }

    #[tokio::test]
    async fn test_routes_validation_and_exports() {
        let state = setup_test_sqlite_state();

        // Empty URL in ingest link
        let empty_ingest = IngestRequest {
            url: "   ".to_string(),
            list_id: Some(1),
            category: None,
            notes: None,
        };
        let (status, Json(res)) = ingest_link(State(state.clone()), UserToken("test-token".to_string()), Json(empty_ingest)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(res.error.unwrap(), "URL cannot be empty");

        // Empty URL in preview scrape
        let empty_preview = IngestRequest {
            url: "".to_string(),
            list_id: None,
            category: None,
            notes: None,
        };
        let (status, Json(res)) = preview_scrape(State(state.clone()), Json(empty_preview)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(res.error.unwrap(), "URL cannot be empty");

        // Get categories
        let (status, Json(res)) = get_categories(
            State(state.clone()),
            UserToken("test-token".to_string()),
            Query(ListPinsQuery {
                list_id: Some(1),
                category: None,
                visited: None,
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.is_some());

        // Export JSON
        let (status, Json(res)) = export_json(
            State(state.clone()),
            UserToken("test-token".to_string()),
            Query(ListPinsQuery {
                list_id: Some(1),
                category: None,
                visited: None,
                search: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.success);

        // Export GeoJSON
        let response = export_geojson(
            State(state.clone()),
            UserToken("test-token".to_string()),
            Query(ListPinsQuery {
                list_id: Some(1),
                category: None,
                visited: None,
                search: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_routes_ssrf_protection_in_ingest_and_preview() {
        let state = setup_test_sqlite_state();

        let malicious_urls = vec![
            "http://127.0.0.1/admin",
            "http://localhost:8080/secret",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.1/internal",
            "http://192.168.1.1/router",
            "file:///etc/passwd",
            "javascript:alert(1)",
        ];

        for url in malicious_urls {
            let ingest_req = IngestRequest {
                url: url.to_string(),
                list_id: Some(1),
                category: None,
                notes: None,
            };
            let (status, Json(res)) = ingest_link(State(state.clone()), UserToken("test-token".to_string()), Json(ingest_req)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "Ingest should block SSRF URL: {}", url);
            assert!(!res.success);
            assert!(
                res.error.as_ref().unwrap().contains("SSRF")
                    || res.error.as_ref().unwrap().contains("scheme")
                    || res.error.as_ref().unwrap().contains("Failed to scrape")
            );

            let preview_req = IngestRequest {
                url: url.to_string(),
                list_id: None,
                category: None,
                notes: None,
            };
            let (status, Json(res)) = preview_scrape(State(state.clone()), Json(preview_req)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "Preview should block SSRF URL: {}", url);
            assert!(!res.success);
        }
    }

    #[tokio::test]
    async fn test_routes_invalid_coordinates_rejected() {
        let state = setup_test_sqlite_state();

        let invalid_pin_req = CreatePinRequest {
            list_id: Some(1),
            title: "Impossible Place".to_string(),
            description: None,
            latitude: 120.0, // Invalid latitude (> 90)
            longitude: 50.0,
            category: None,
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None,
        };
        let (status, Json(res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(invalid_pin_req)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!res.success);
        assert!(res.error.unwrap().contains("Invalid GPS coordinates"));

        let nan_pin_req = CreatePinRequest {
            list_id: Some(1),
            title: "NaN Place".to_string(),
            description: None,
            latitude: f64::NAN,
            longitude: 50.0,
            category: None,
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None,
        };
        let (status, Json(_res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(nan_pin_req)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // Update pin with invalid coords
        let valid_req = CreatePinRequest {
            list_id: Some(1),
            title: "Valid Place".to_string(),
            description: None,
            latitude: 40.0,
            longitude: 40.0,
            category: None,
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None,
        };
        let (_, Json(res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(valid_req)).await;
        let pin = res.data.unwrap();

        let invalid_update = UpdatePinRequest {
            list_id: None,
            title: None,
            description: None,
            latitude: Some(-95.0),
            longitude: None,
            category: None,
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None,
        };
        let (status, Json(res)) = update_pin(State(state.clone()), UserToken("test-token".to_string()), Path(pin.id), Json(invalid_update)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(res.error.unwrap().contains("Invalid latitude"));
    }

    #[tokio::test]
    async fn test_routes_in_memory_backend() {
        let state = setup_test_in_memory_state();

        let req = CreateListRequest {
            name: "Kyoto Trip".to_string(),
            icon: Some("⛩️".to_string()),
        };
        let (status, Json(res)) = create_list(State(state.clone()), UserToken("test-token".to_string()), Json(req)).await;
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
        let (status, Json(res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(pin_req)).await;
        assert_eq!(status, StatusCode::CREATED);
        let pin = res.data.unwrap();

        let (status, Json(res)) = toggle_visited(State(state.clone()), UserToken("test-token".to_string()), Path(pin.id)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(res.data.unwrap().visited);
    }
}
