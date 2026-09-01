use axum::{
    debug_handler,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use super::{check_permission_or_err, AppState, UserToken};
use crate::models::{ApiResponse, GeoLocation, ListPinsQuery, Pin};

#[debug_handler]
pub async fn app_info() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "name": "bList",
            "version": env!("CARGO_PKG_VERSION"),
            "repository": "https://github.com/radmuffin/bList",
            "issues_url": "https://github.com/radmuffin/bList/issues",
            "license": "MIT"
        })),
    )
}

#[derive(serde::Deserialize)]
pub struct GeocodeQuery {
    pub q: String,
}

#[debug_handler]
pub async fn geocode(
    State(state): State<AppState>,
    Query(query): Query<GeocodeQuery>,
) -> (StatusCode, Json<ApiResponse<GeoLocation>>) {
    match state.geocoder.geocode(&query.q).await {
        Ok(Some(geo)) => (StatusCode::OK, Json(ApiResponse::ok(geo))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err("Location not found")),
        ),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(e))),
    }
}

pub async fn export_geojson(
    State(state): State<AppState>,
    user_token: UserToken,
    Query(query): Query<ListPinsQuery>,
) -> impl IntoResponse {
    if let Some(list_id) = query.list_id {
        if let Err(err) =
            check_permission_or_err::<serde_json::Value>(&state.storage, &user_token.0, list_id)
        {
            return err.into_response();
        }
    }
    let pins = match state.storage.list_pins(&query, &user_token.0) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Database error: {}", e) })),
            )
                .into_response();
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
                    "emoji": pin.emoji,
                    "tags": pin.tags,
                    "priority": pin.priority,
                    "day_group": pin.day_group,
                    "custom_order": pin.custom_order,
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
