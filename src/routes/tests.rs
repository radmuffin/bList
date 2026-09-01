use super::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::db::{InMemoryStorage, SqliteRepository};
use crate::geocoder::Geocoder;
use crate::models::{
    CreateListRequest, CreatePinRequest, IngestRequest, JoinListRequest, ListPinsQuery,
    UpdateListRequest, UpdatePinRequest,
};
use crate::scraper::Scraper;

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
        visited: Some(false), ..Default::default()
    };
    let (status, Json(res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(pin_req)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(res.success);
    let created_pin = res.data.unwrap();
    assert_eq!(created_pin.title, "Colosseum");
    assert_eq!(created_pin.category, "History");
    assert!(!created_pin.visited);

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
        visited: None, ..Default::default()
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
        visited: None, ..Default::default()
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
    assert!(res.data.unwrap().visited);

    let (status, Json(res)) = toggle_visited(State(state.clone()), UserToken("test-token".to_string()), Path(created_pin.id)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!res.data.unwrap().visited);

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
            ..Default::default()
        };
        let _ = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(req)).await;
    }

    // Filter by category
    let query = ListPinsQuery {
        list_id: Some(1),
        category: Some("Sightseeing".to_string()),
        visited: None,
        search: None, ..Default::default()
    };
    let (status, Json(res)) = list_pins(State(state.clone()), UserToken("test-token".to_string()), Query(query)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res.data.unwrap().len(), 2);

    // Filter by visited
    let query = ListPinsQuery {
        list_id: Some(1),
        category: None,
        visited: Some(true),
        search: None, ..Default::default()
    };
    let (status, Json(res)) = list_pins(State(state.clone()), UserToken("test-token".to_string()), Query(query)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res.data.unwrap().len(), 2);

    // Search keyword
    let query = ListPinsQuery {
        list_id: Some(1),
        category: None,
        visited: None,
        search: Some("Sagrada".to_string()), ..Default::default()
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
        notes: None, ..Default::default()
    };
    let (status, Json(res)) = ingest_link(State(state.clone()), UserToken("test-token".to_string()), Json(empty_ingest)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res.error.unwrap(), "URL cannot be empty");

    // Empty URL in preview scrape
    let empty_preview = IngestRequest {
        url: "".to_string(),
        list_id: None,
        category: None,
        notes: None, ..Default::default()
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
            search: None, ..Default::default()
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
            search: None, ..Default::default()
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
            search: None, ..Default::default()
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
            notes: None, ..Default::default()
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
            notes: None, ..Default::default()
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
        visited: None, ..Default::default()
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
        visited: None, ..Default::default()
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
        visited: None, ..Default::default()
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
        visited: None, ..Default::default()
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
        visited: Some(false), ..Default::default()
    };
    let (status, Json(res)) = create_pin(State(state.clone()), UserToken("test-token".to_string()), Json(pin_req)).await;
    assert_eq!(status, StatusCode::CREATED);
    let pin = res.data.unwrap();

    let (status, Json(res)) = toggle_visited(State(state.clone()), UserToken("test-token".to_string()), Path(pin.id)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(res.data.unwrap().visited);
}

#[tokio::test]
async fn test_routes_multi_device_join_and_collaboration() {
    let state = setup_test_in_memory_state();

    // Device A creates a trip
    let (status, Json(res)) = create_list(
        State(state.clone()),
        UserToken("device-a".to_string()),
        Json(CreateListRequest {
            name: "Summer Roadtrip".to_string(),
            icon: Some("🚗".to_string()),
        }),
    ).await;
    assert_eq!(status, StatusCode::CREATED);
    let list_a = res.data.unwrap();

    // Device B joins the trip using the share token
    let (status, Json(res)) = join_list(
        State(state.clone()),
        UserToken("device-b".to_string()),
        Json(JoinListRequest {
            share_token: list_a.share_token.clone(),
        }),
    ).await;
    assert_eq!(status, StatusCode::OK);
    let joined_list = res.data.unwrap();
    assert_eq!(joined_list.name, "Summer Roadtrip");

    // Device B adds a pin to the shared list
    let (status, Json(res)) = create_pin(
        State(state.clone()),
        UserToken("device-b".to_string()),
        Json(CreatePinRequest {
            list_id: Some(list_a.id),
            title: "Grand Canyon".to_string(),
            description: None,
            latitude: 36.1069,
            longitude: -112.1129,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: Some(false), ..Default::default()
        }),
    ).await;
    assert_eq!(status, StatusCode::CREATED);
    let pin = res.data.unwrap();

    // Device A can read and update the pin
    let (status, Json(res)) = get_pin(
        State(state.clone()),
        UserToken("device-a".to_string()),
        Path(pin.id),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res.data.unwrap().title, "Grand Canyon");

    // Unauthorized Device C cannot access the pin
    let (status, _) = get_pin(
        State(state.clone()),
        UserToken("device-c".to_string()),
        Path(pin.id),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_routes_app_info() {
    let (status, Json(info)) = app_info().await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info["name"], "bList");
    assert_eq!(info["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(info["repository"], "https://github.com/radmuffin/bList");
    assert_eq!(info["issues_url"], "https://github.com/radmuffin/bList/issues");
    assert_eq!(info["license"], "MIT");
}

#[tokio::test]
async fn test_routes_user_token_extractor_header_and_query() {
    use axum::http::Request;

    let state = setup_test_in_memory_state();

    // 1. Header token
    let req1 = Request::builder()
        .header("x-user-token", "header-device-123")
        .body(())
        .unwrap();
    let (mut parts1, _) = req1.into_parts();
    let token1 = UserToken::from_request_parts(&mut parts1, &state).await.expect("extract header");
    assert_eq!(token1.0, "header-device-123");

    // 2. Query param token
    let req2 = Request::builder()
        .uri("/api/pins?user_token=query-device-456&list_id=1")
        .body(())
        .unwrap();
    let (mut parts2, _) = req2.into_parts();
    let token2 = UserToken::from_request_parts(&mut parts2, &state).await.expect("extract query");
    assert_eq!(token2.0, "query-device-456");

    // 3. Encoded query param token
    let req3 = Request::builder()
        .uri("/api/pins?user_token=device%20custom%20token")
        .body(())
        .unwrap();
    let (mut parts3, _) = req3.into_parts();
    let token3 = UserToken::from_request_parts(&mut parts3, &state).await.expect("extract encoded query");
    assert_eq!(token3.0, "device custom token");

    // 4. Missing token -> rejected with 400 Bad Request
    let req4 = Request::builder()
        .uri("/api/pins")
        .body(())
        .unwrap();
    let (mut parts4, _) = req4.into_parts();
    let err4 = UserToken::from_request_parts(&mut parts4, &state).await;
    assert!(err4.is_err());
    let (status, _) = err4.unwrap_err();
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 5. Empty token -> rejected with 400 Bad Request
    let req5 = Request::builder()
        .header("x-user-token", "   ")
        .body(())
        .unwrap();
    let (mut parts5, _) = req5.into_parts();
    let err5 = UserToken::from_request_parts(&mut parts5, &state).await;
    assert!(err5.is_err());
}

#[tokio::test]
async fn test_routes_pin_filtering_combined_matrix() {
    let state = setup_test_in_memory_state();
    let token = UserToken("tester-filter-matrix".to_string());

    // Create list 2
    let (_, Json(res_list)) = create_list(
        State(state.clone()),
        token.clone(),
        Json(CreateListRequest {
            name: "European Tour".to_string(),
            icon: Some("✈️".to_string()),
        }),
    ).await;
    let list2 = res_list.data.unwrap();

    // Pin 1: List 1, Cafe, Visited
    let _ = create_pin(
        State(state.clone()),
        token.clone(),
        Json(CreatePinRequest {
            list_id: Some(1),
            title: "Cafe de Flore".to_string(),
            description: Some("Historic Parisian coffee shop".to_string()),
            latitude: 48.8542,
            longitude: 2.3325,
            category: Some("Cafe".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Saint-Germain-des-Prés, Paris".to_string()),
            notes: Some("Famous hot chocolate".to_string()),
            visited: Some(true), ..Default::default()
        }),
    ).await;

    // Pin 2: List 1, Sightseeing, Bucket
    let _ = create_pin(
        State(state.clone()),
        token.clone(),
        Json(CreatePinRequest {
            list_id: Some(1),
            title: "Eiffel Tower".to_string(),
            description: Some("Iron lattice tower on Champ de Mars".to_string()),
            latitude: 48.8584,
            longitude: 2.2945,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Paris, France".to_string()),
            notes: Some("Visit at golden hour".to_string()),
            visited: Some(false), ..Default::default()
        }),
    ).await;

    // Pin 3: List 2, Cafe, Bucket
    let _ = create_pin(
        State(state.clone()),
        token.clone(),
        Json(CreatePinRequest {
            list_id: Some(list2.id),
            title: "Caffe Florian".to_string(),
            description: Some("Oldest coffeehouse in Venice".to_string()),
            latitude: 45.4337,
            longitude: 12.3381,
            category: Some("Cafe".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Piazza San Marco, Venice, Italy".to_string()),
            notes: Some("Live orchestral music outside".to_string()),
            visited: Some(false), ..Default::default()
        }),
    ).await;

    // Query 1: All pins for user
    let (status, Json(res)) = list_pins(
        State(state.clone()),
        token.clone(),
        Query(ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: None, ..Default::default()
        }),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res.data.unwrap().len(), 3);

    // Query 2: Filter by List 1 only
    let (_, Json(res)) = list_pins(
        State(state.clone()),
        token.clone(),
        Query(ListPinsQuery {
            list_id: Some(1),
            category: None,
            visited: None,
            search: None, ..Default::default()
        }),
    ).await;
    assert_eq!(res.data.unwrap().len(), 2);

    // Query 3: Filter by category Cafe across all lists
    let (_, Json(res)) = list_pins(
        State(state.clone()),
        token.clone(),
        Query(ListPinsQuery {
            list_id: None,
            category: Some("Cafe".to_string()),
            visited: None,
            search: None, ..Default::default()
        }),
    ).await;
    assert_eq!(res.data.unwrap().len(), 2);

    // Query 4: Filter by category Cafe in List 1
    let (_, Json(res)) = list_pins(
        State(state.clone()),
        token.clone(),
        Query(ListPinsQuery {
            list_id: Some(1),
            category: Some("Cafe".to_string()),
            visited: None,
            search: None, ..Default::default()
        }),
    ).await;
    let pins = res.data.unwrap();
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0].title, "Cafe de Flore");

    // Query 5: Filter visited = true
    let (_, Json(res)) = list_pins(
        State(state.clone()),
        token.clone(),
        Query(ListPinsQuery {
            list_id: None,
            category: None,
            visited: Some(true),
            search: None, ..Default::default()
        }),
    ).await;
    assert_eq!(res.data.unwrap().len(), 1);

    // Query 6: Search notes match "orchestral"
    let (_, Json(res)) = list_pins(
        State(state.clone()),
        token.clone(),
        Query(ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: Some("orchestral".to_string()), ..Default::default()
        }),
    ).await;
    let search_res = res.data.unwrap();
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].title, "Caffe Florian");

    // Query 7: Search no match
    let (_, Json(res)) = list_pins(
        State(state.clone()),
        token.clone(),
        Query(ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: Some("UnmatchedKeyword12345".to_string()), ..Default::default()
        }),
    ).await;
    assert_eq!(res.data.unwrap().len(), 0);
}

#[tokio::test]
async fn test_auto_onboarding_multi_user_isolation_and_pin_creation() {
    let state = setup_test_sqlite_state();
    let user_a = UserToken("user-a-token-111".to_string());
    let user_b = UserToken("user-b-token-222".to_string());

    // User A fetches lists -> gets auto-onboarded default list
    let (status_a, Json(res_a)) = list_lists(State(state.clone()), user_a.clone()).await;
    assert_eq!(status_a, StatusCode::OK);
    let lists_a = res_a.data.unwrap();
    assert_eq!(lists_a.len(), 1);
    let list_a_id = lists_a[0].id;

    // User B fetches lists -> gets their own auto-onboarded default list
    let (status_b, Json(res_b)) = list_lists(State(state.clone()), user_b.clone()).await;
    assert_eq!(status_b, StatusCode::OK);
    let lists_b = res_b.data.unwrap();
    assert_eq!(lists_b.len(), 1);
    let list_b_id = lists_b[0].id;

    // Both users should have valid, distinct lists
    assert_ne!(list_a_id, list_b_id);

    // User A creates a pin without explicitly specifying list_id
    let (status_pin_a, Json(res_pin_a)) = create_pin(
        State(state.clone()),
        user_a.clone(),
        Json(CreatePinRequest {
            title: "User A Tower".to_string(),
            description: None,
            latitude: 48.8584,
            longitude: 2.2945,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: Some(false),
            list_id: None, ..Default::default()
        }),
    ).await;
    assert_eq!(status_pin_a, StatusCode::CREATED);
    assert_eq!(res_pin_a.data.unwrap().list_id, list_a_id);

    // User B creates a pin without explicitly specifying list_id
    let (status_pin_b, Json(res_pin_b)) = create_pin(
        State(state.clone()),
        user_b.clone(),
        Json(CreatePinRequest {
            title: "User B Garden".to_string(),
            description: None,
            latitude: 35.6586,
            longitude: 139.7454,
            category: Some("Nature".to_string()),
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: Some(false),
            list_id: None, ..Default::default()
        }),
    ).await;
    assert_eq!(status_pin_b, StatusCode::CREATED);
    assert_eq!(res_pin_b.data.unwrap().list_id, list_b_id);

    // User A listing pins only sees their own pin
    let (_, Json(pins_a_res)) = list_pins(
        State(state.clone()),
        user_a.clone(),
        Query(ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: None, ..Default::default()
        }),
    ).await;
    let pins_a = pins_a_res.data.unwrap();
    assert_eq!(pins_a.len(), 1);
    assert_eq!(pins_a[0].title, "User A Tower");

    // User B listing pins only sees their own pin
    let (_, Json(pins_b_res)) = list_pins(
        State(state.clone()),
        user_b.clone(),
        Query(ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: None, ..Default::default()
        }),
    ).await;
    let pins_b = pins_b_res.data.unwrap();
    assert_eq!(pins_b.len(), 1);
    assert_eq!(pins_b[0].title, "User B Garden");
}

#[tokio::test]
async fn test_routes_import_places_and_batch_processing() {
    let storage = Arc::new(InMemoryStorage::new());
    let geocoder = Arc::new(Geocoder::new());
    let scraper = Arc::new(Scraper::with_geocoder(geocoder.clone()));
    let state = AppState { storage, scraper, geocoder };
    let user = UserToken("import-user-token".to_string());

    let json_data = r##"{
        "features": [
            {
                "geometry": { "coordinates": [139.7004, 35.6595] },
                "properties": {
                    "Title": "Shibuya Sky",
                    "Location": { "Address": "Tokyo, Japan" },
                    "Tags": "#view #tokyo",
                    "Priority": true
                }
            },
            {
                "geometry": { "coordinates": [139.7745, 35.7148] },
                "properties": {
                    "Title": "Ueno Park",
                    "Location": { "Address": "Ueno, Tokyo" },
                    "Tags": "#park #sakura"
                }
            }
        ]
    }"##;

    let payload = crate::models::ImportPayload {
        list_id: None,
        new_list_name: Some("Tokyo 2026 Trip".to_string()),
        default_category: Some("Sightseeing".to_string()),
        items: None,
        raw_data: Some(json_data.to_string()),
        format: Some("takeout_json".to_string()),
    };

    let (status, Json(res)) = import_places(State(state.clone()), user.clone(), Json(payload)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(res.success);
    let summary = res.data.unwrap();
    assert_eq!(summary.total_processed, 2);
    assert_eq!(summary.imported_count, 2);
    assert_eq!(summary.list_name, "Tokyo 2026 Trip");

    // Verify tags filter works
    let (_, Json(tag_res)) = list_pins(
        State(state.clone()),
        user.clone(),
        Query(ListPinsQuery {
            tag: Some("tokyo".to_string()),
            ..Default::default()
        }),
    ).await;
    let tag_pins = tag_res.data.unwrap();
    assert_eq!(tag_pins.len(), 1);
    assert_eq!(tag_pins[0].title, "Shibuya Sky");
}

#[tokio::test]
async fn test_routes_duplicate_pin_rejection() {
    let state = setup_test_sqlite_state();
    let user = UserToken("dup-test-user".to_string());

    let req1 = CreatePinRequest {
        list_id: Some(1),
        title: "Eiffel Tower".to_string(),
        description: Some("Famous landmark".to_string()),
        latitude: 48.8584,
        longitude: 2.2945,
        category: Some("Sightseeing".to_string()),
        source_url: Some("https://maps.google.com/?cid=eiffel".to_string()),
        ..Default::default()
    };

    let (status1, Json(res1)) = create_pin(State(state.clone()), user.clone(), Json(req1.clone())).await;
    assert_eq!(status1, StatusCode::CREATED);
    assert!(res1.success);

    // Try creating duplicate by same coordinates and title
    let (status2, Json(res2)) = create_pin(State(state.clone()), user.clone(), Json(req1)).await;
    assert_eq!(status2, StatusCode::CONFLICT);
    assert!(!res2.success);
    assert!(res2.error.unwrap().contains("already saved"));

    // Try creating duplicate by same source_url with slightly different title
    let req3 = CreatePinRequest {
        list_id: Some(1),
        title: "Tour Eiffel".to_string(),
        description: None,
        latitude: 48.0,
        longitude: 2.0,
        category: Some("Sightseeing".to_string()),
        source_url: Some("https://maps.google.com/?cid=eiffel".to_string()),
        ..Default::default()
    };
    let (status3, Json(res3)) = create_pin(State(state.clone()), user.clone(), Json(req3)).await;
    assert_eq!(status3, StatusCode::CONFLICT);
    assert!(!res3.success);
}

#[tokio::test]
async fn test_routes_ingest_blist_link() {
    let state = setup_test_sqlite_state();
    let user = UserToken("blist-share-user".to_string());

    let ingest_req = crate::models::IngestRequest {
        url: "https://blist.fly.dev/?lat=35.6586&lng=139.7454&title=Tokyo%20Tower&address=Tokyo,%20Japan".to_string(),
        list_id: Some(1),
        category: Some("Sightseeing".to_string()),
        emoji: Some("🗼".to_string()),
        tags: Some("#tokyo".to_string()),
        priority: Some(true),
        day_group: Some(1),
        notes: Some("Shared from bList".to_string()),
        opening_hours: None,
    };

    let (status, Json(res)) = ingest_link(State(state.clone()), user.clone(), Json(ingest_req.clone())).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(res.success);
    let pin = res.data.unwrap();
    assert_eq!(pin.title, "Tokyo Tower");
    assert_eq!(pin.latitude, 35.6586);
    assert_eq!(pin.longitude, 139.7454);
    assert!(pin.priority);

    // Ingesting the same bList link again should be rejected as a duplicate
    let (status_dup, Json(res_dup)) = ingest_link(State(state.clone()), user.clone(), Json(ingest_req)).await;
    assert_eq!(status_dup, StatusCode::CONFLICT);
    assert!(!res_dup.success);
    assert!(res_dup.error.unwrap().contains("already saved"));
}

