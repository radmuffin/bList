use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use super::{check_permission_or_err, AppState, UserToken};
use crate::models::{ApiResponse, CreateListRequest, JoinListRequest, List, UpdateListRequest};

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

    match state.storage.count_user_lists(&user_token.0) {
        Ok(count) if count >= crate::db::MAX_LISTS_PER_USER => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(format!(
                    "Quota exceeded: Maximum {} lists allowed per account.",
                    crate::db::MAX_LISTS_PER_USER
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
    match state
        .storage
        .join_list(req.share_token.trim(), &user_token.0)
    {
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
