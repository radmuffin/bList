pub mod export;
pub mod ingest;
pub mod lists;
pub mod pins;
#[cfg(test)]
pub mod tests;
pub mod user;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use std::sync::Arc;

#[allow(unused_imports)]
pub use export::{app_info, export_geojson, export_json, geocode, health_check, GeocodeQuery};
pub use ingest::{import_places, ingest_link, preview_scrape};
pub use lists::{create_list, delete_list, get_list, join_list, list_lists, update_list};
pub use pins::{
    create_pin, delete_pin, get_categories, get_pin, list_pins, toggle_visited, update_pin,
};
pub use user::{get_list_collaborators, get_profile, update_profile};

use crate::db::StorageEngine;
use crate::geocoder::Geocoder;
use crate::models::{ApiResponse, Pin};
use crate::scraper::Scraper;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn StorageEngine>,
    pub scraper: Arc<Scraper>,
    pub geocoder: Arc<Geocoder>,
}

#[derive(Debug, Clone)]
pub struct UserToken(pub String);

#[axum::async_trait]
impl FromRequestParts<AppState> for UserToken {
    type Rejection = (StatusCode, Json<ApiResponse<()>>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("x-user-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                parts.uri.query().and_then(|q| {
                    q.split('&')
                        .find(|p| p.starts_with("user_token=") || p.starts_with("token="))
                        .and_then(|p| p.split('=').nth(1))
                        .map(|v| {
                            urlencoding::decode(v)
                                .unwrap_or(std::borrow::Cow::Borrowed(v))
                                .into_owned()
                        })
                })
            });

        let token = match token {
            Some(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::err("Missing or empty X-User-Token header")),
                ))
            }
        };

        if let Err(e) = state.storage.auto_associate_device(&token) {
            tracing::error!("Failed to auto-associate device: {}", e);
        }

        Ok(UserToken(token))
    }
}

pub(crate) fn check_permission_or_err<T>(
    storage: &Arc<dyn StorageEngine>,
    user_token: &str,
    list_id: i64,
) -> Result<(), (StatusCode, Json<ApiResponse<T>>)> {
    match storage.get_list(list_id) {
        Ok(Some(_)) => match storage.check_permission(user_token, list_id) {
            Ok(true) => Ok(()),
            Ok(false) => Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::err("Forbidden: Access denied to this list")),
            )),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database error: {}", e))),
            )),
        },
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

pub(crate) fn check_pin_permission_or_err<T>(
    storage: &Arc<dyn StorageEngine>,
    user_token: &str,
    pin_id: i64,
) -> Result<Pin, (StatusCode, Json<ApiResponse<T>>)> {
    let pin = match storage.get_pin(pin_id) {
        Ok(Some(pin)) => pin,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::err(format!("Pin #{} not found", pin_id))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Database error: {}", e))),
            ))
        }
    };
    check_permission_or_err(storage, user_token, pin.list_id)?;
    Ok(pin)
}
