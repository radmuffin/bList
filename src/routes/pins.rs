use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use super::{check_permission_or_err, check_pin_permission_or_err, AppState, UserToken};
use crate::models::{ApiResponse, CreatePinRequest, ListPinsQuery, Pin, UpdatePinRequest};

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

    let list_id = match req.list_id {
        Some(id) if id > 0 => {
            if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, id) {
                return err;
            }
            id
        }
        _ => {
            let user_lists = state.storage.list_lists(&user_token.0).unwrap_or_default();
            if let Some(first_list) = user_lists.first() {
                first_list.id
            } else {
                1
            }
        }
    };
    let mut resolved_req = req;
    resolved_req.list_id = Some(list_id);

    match state.storage.count_list_pins(list_id) {
        Ok(count) if count >= crate::db::MAX_PINS_PER_LIST => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(format!(
                    "Quota exceeded: Maximum {} pins allowed per list.",
                    crate::db::MAX_PINS_PER_LIST
                ))),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database error: {}", e))),
            );
        }
        _ => {}
    }

    match state.storage.count_user_pins(&user_token.0) {
        Ok(count) if count >= crate::db::MAX_PINS_PER_USER => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(format!(
                    "Quota exceeded: Maximum {} total pins allowed per account.",
                    crate::db::MAX_PINS_PER_USER
                ))),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database error: {}", e))),
            );
        }
        _ => {}
    }

    if let Ok(Some(existing)) = state.storage.find_duplicate_pin(
        list_id,
        &resolved_req.title,
        resolved_req.latitude,
        resolved_req.longitude,
        resolved_req.source_url.as_deref(),
        None,
    ) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::err(format!(
                "Place '{}' is already saved in this list.",
                existing.title
            ))),
        );
    }

    match state.storage.create_pin(&resolved_req) {
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

    let target_list_id = req.list_id.unwrap_or(pin.list_id);
    if let Some(new_list_id) = req.list_id {
        if new_list_id != pin.list_id {
            if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, new_list_id) {
                return err;
            }
            if state.storage.count_list_pins(new_list_id).unwrap_or(0) >= crate::db::MAX_PINS_PER_LIST {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::err(format!(
                        "Quota exceeded: Target list #{} already has maximum {} places.",
                        new_list_id, crate::db::MAX_PINS_PER_LIST
                    ))),
                );
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

    let check_title = req.title.as_deref().unwrap_or(&pin.title);
    let check_lat = req.latitude.unwrap_or(pin.latitude);
    let check_lon = req.longitude.unwrap_or(pin.longitude);
    let check_source = req.source_url.as_deref().or(pin.source_url.as_deref());

    if let Ok(Some(existing)) = state.storage.find_duplicate_pin(
        target_list_id,
        check_title,
        check_lat,
        check_lon,
        check_source,
        Some(id),
    ) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::err(format!(
                "Place '{}' is already saved in this list.",
                existing.title
            ))),
        );
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
