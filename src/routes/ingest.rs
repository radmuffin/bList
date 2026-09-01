use axum::{debug_handler, extract::State, http::StatusCode, Json};

use super::{check_permission_or_err, AppState, UserToken};
use crate::models::{
    ApiResponse, CreateListRequest, CreatePinRequest, ImportPayload, ImportSummary, IngestRequest,
    Pin, ScrapedMetadata,
};

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

    let category = req
        .category
        .unwrap_or_else(|| match meta.source_type.as_str() {
            "instagram" | "tiktok" => "Social".to_string(),
            "google_maps" | "apple_maps" | "tripadvisor" | "yelp" | "alltrails" => {
                "Place".to_string()
            }
            _ => "General".to_string(),
        });

    let create_req = CreatePinRequest {
        list_id: Some(list_id),
        title: meta.title,
        description: meta.description,
        latitude: lat,
        longitude: lon,
        category: Some(category),
        emoji: req.emoji,
        tags: req.tags,
        priority: req.priority,
        day_group: req.day_group,
        custom_order: None,
        opening_hours: req.opening_hours.or(meta.opening_hours),
        source_url: Some(meta.source_url),
        image_url: meta.image_url,
        address: meta.address,
        notes: req.notes,
        visited: Some(false),
    };

    if let Ok(Some(existing)) = state.storage.find_duplicate_pin(
        list_id,
        &create_req.title,
        create_req.latitude,
        create_req.longitude,
        create_req.source_url.as_deref(),
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

    match state.storage.create_pin(&create_req) {
        Ok(pin) => (StatusCode::CREATED, Json(ApiResponse::ok(pin))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!(
                "Failed to save ingested pin: {}",
                e
            ))),
        ),
    }
}

#[debug_handler]
pub async fn import_places(
    State(state): State<AppState>,
    user_token: UserToken,
    Json(payload): Json<ImportPayload>,
) -> (StatusCode, Json<ApiResponse<ImportSummary>>) {
    let list_id = if let Some(ref new_name) = payload.new_list_name {
        if !new_name.trim().is_empty() {
            if state.storage.count_user_lists(&user_token.0).unwrap_or(0)
                >= crate::db::MAX_LISTS_PER_USER
            {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::err(format!(
                        "Quota exceeded: Maximum {} lists allowed per account.",
                        crate::db::MAX_LISTS_PER_USER
                    ))),
                );
            }
            let create_list_req = CreateListRequest {
                name: new_name.trim().to_string(),
                icon: Some("📁".to_string()),
            };
            match state.storage.create_list(&create_list_req, &user_token.0) {
                Ok(new_list) => new_list.id,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::err(format!("Failed to create list: {}", e))),
                    );
                }
            }
        } else {
            1
        }
    } else {
        match payload.list_id {
            Some(id) if id > 0 => {
                if let Err(err) = check_permission_or_err(&state.storage, &user_token.0, id) {
                    return err;
                }
                id
            }
            _ => {
                let user_lists = state.storage.list_lists(&user_token.0).unwrap_or_default();
                if let Some(first) = user_lists.first() {
                    first.id
                } else {
                    1
                }
            }
        }
    };

    let list_name = state
        .storage
        .get_list(list_id)
        .ok()
        .flatten()
        .map(|l| l.name)
        .unwrap_or_else(|| "My Bucket List".to_string());

    let mut raw_items = if let Some(items) = payload.items {
        items
    } else if let Some(ref raw) = payload.raw_data {
        match crate::importer::parse_import_data(raw, payload.format.as_deref()) {
            Ok(parsed) => parsed,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::err(format!(
                        "Failed to parse import data: {}",
                        e
                    ))),
                );
            }
        }
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("No items or raw_data provided for import")),
        );
    };

    if raw_items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("Import payload contains 0 items")),
        );
    }

    let default_cat = payload.default_category.as_deref().unwrap_or("General");
    let total_processed = raw_items.len();
    let mut warnings = Vec::new();
    let mut valid_create_requests = Vec::new();

    for item in raw_items.iter_mut() {
        if item.latitude.is_none() || item.longitude.is_none() {
            let query = item.address.as_ref().or(Some(&item.title));
            if let Some(q) = query {
                if !q.trim().is_empty() {
                    if let Ok(Some(geo)) = state.geocoder.geocode(q).await {
                        item.latitude = Some(geo.latitude);
                        item.longitude = Some(geo.longitude);
                    }
                }
            }
        }

        if let (Some(lat), Some(lon)) = (item.latitude, item.longitude) {
            if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                valid_create_requests.push(CreatePinRequest {
                    list_id: Some(list_id),
                    title: item.title.trim().to_string(),
                    description: item.description.clone(),
                    latitude: lat,
                    longitude: lon,
                    category: item
                        .category
                        .clone()
                        .or_else(|| Some(default_cat.to_string())),
                    emoji: item.emoji.clone(),
                    tags: item.tags.clone(),
                    priority: item.priority,
                    day_group: item.day_group,
                    custom_order: None,
                    opening_hours: item.opening_hours.clone(),
                    source_url: item.source_url.clone(),
                    image_url: item.image_url.clone(),
                    address: item.address.clone(),
                    notes: item.notes.clone(),
                    visited: item.visited,
                });
            } else {
                warnings.push(format!(
                    "Skipped '{}': Coordinates ({}, {}) out of bounds",
                    item.title, lat, lon
                ));
            }
        } else {
            warnings.push(format!(
                "Skipped '{}': Missing GPS coordinates and geocoding failed",
                item.title
            ));
        }
    }

    // Deduplicate against existing pins in the database and within batch
    let mut deduplicated_requests = Vec::new();
    for req in valid_create_requests {
        let is_dup_in_batch = deduplicated_requests.iter().any(|d: &CreatePinRequest| {
            (d.source_url.is_some() && d.source_url == req.source_url)
                || ((d.latitude - req.latitude).abs() < 0.0001
                    && (d.longitude - req.longitude).abs() < 0.0001)
                || (d.title.trim().eq_ignore_ascii_case(req.title.trim())
                    && (d.latitude - req.latitude).abs() < 0.001
                    && (d.longitude - req.longitude).abs() < 0.001)
        });
        if is_dup_in_batch {
            warnings.push(format!("Skipped duplicate '{}' in import data", req.title));
            continue;
        }

        if let Ok(Some(existing)) = state.storage.find_duplicate_pin(
            list_id,
            &req.title,
            req.latitude,
            req.longitude,
            req.source_url.as_deref(),
            None,
        ) {
            warnings.push(format!(
                "Skipped '{}': Already saved in this list",
                existing.title
            ));
            continue;
        }

        deduplicated_requests.push(req);
    }

    let current_list_count = state.storage.count_list_pins(list_id).unwrap_or(0);
    let space_left_in_list = crate::db::MAX_PINS_PER_LIST.saturating_sub(current_list_count);

    let current_user_pin_count = state.storage.count_user_pins(&user_token.0).unwrap_or(0);
    let space_left_in_user = crate::db::MAX_PINS_PER_USER.saturating_sub(current_user_pin_count);

    let allowed_count = space_left_in_list.min(space_left_in_user);
    if deduplicated_requests.len() > allowed_count {
        warnings.push(format!(
            "Quota limit reached. Only the first {} places were imported (List limit: {}, Account limit: {}).",
            allowed_count, crate::db::MAX_PINS_PER_LIST, crate::db::MAX_PINS_PER_USER
        ));
        deduplicated_requests.truncate(allowed_count);
    }

    let created_pins = match state
        .storage
        .create_pins_batch(list_id, &deduplicated_requests)
    {
        Ok(pins) => pins,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!("Batch insert failed: {}", e))),
            );
        }
    };

    let imported_count = created_pins.len();
    let skipped_count = total_processed.saturating_sub(imported_count);

    let summary = ImportSummary {
        list_id,
        list_name,
        total_processed,
        imported_count,
        skipped_count,
        warnings,
        created_pins,
    };

    (StatusCode::OK, Json(ApiResponse::ok(summary)))
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
