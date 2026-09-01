use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use super::{check_permission_or_err, AppState, UserToken};
use crate::models::{ApiResponse, Collaborator, UpdateUserProfileRequest, UserProfile};

#[debug_handler]
pub async fn get_profile(
    State(state): State<AppState>,
    user_token: UserToken,
) -> (StatusCode, Json<ApiResponse<UserProfile>>) {
    match state.storage.get_user_profile(&user_token.0) {
        Ok(profile) => (StatusCode::OK, Json(ApiResponse::ok(profile))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!(
                "Failed to retrieve user profile: {}",
                e
            ))),
        ),
    }
}

#[debug_handler]
pub async fn update_profile(
    State(state): State<AppState>,
    user_token: UserToken,
    Json(req): Json<UpdateUserProfileRequest>,
) -> (StatusCode, Json<ApiResponse<UserProfile>>) {
    match state.storage.update_user_profile(&user_token.0, &req) {
        Ok(profile) => (StatusCode::OK, Json(ApiResponse::ok(profile))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to update profile: {}", e))),
        ),
    }
}

#[debug_handler]
pub async fn get_list_collaborators(
    State(state): State<AppState>,
    user_token: UserToken,
    Path(id): Path<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<Collaborator>>>) {
    if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, id) {
        return err;
    }

    match state.storage.get_list_collaborators(id) {
        Ok(collaborators) => (StatusCode::OK, Json(ApiResponse::ok(collaborators))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!(
                "Failed to get collaborators: {}",
                e
            ))),
        ),
    }
}
