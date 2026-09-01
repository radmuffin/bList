use super::*;
use crate::models::{
    CreateListRequest, CreatePinRequest, ListPinsQuery, UpdateListRequest, UpdatePinRequest,
    UpdateUserProfileRequest,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct TestDbGuard {
    pub path: String,
}

impl TestDbGuard {
    pub fn new(prefix: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = format!("test_{}_{}_{}.db", prefix, timestamp, count);
        Self::cleanup_files(&path);
        TestDbGuard { path }
    }

    fn cleanup_files(path: &str) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
        let _ = std::fs::remove_file(format!("{}-journal", path));
    }
}

impl Drop for TestDbGuard {
    fn drop(&mut self) {
        Self::cleanup_files(&self.path);
    }
}

#[test]
fn test_pragmas_configured() {
    let guard = TestDbGuard::new("pragmas");
    let conn = init_db(&guard.path).expect("init db");

    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).expect("journal_mode");
    assert_eq!(journal_mode.to_lowercase(), "wal");

    let synchronous: i32 = conn.query_row("PRAGMA synchronous", [], |r| r.get(0)).expect("synchronous");
    assert_eq!(synchronous, 1); // 1 = NORMAL

    let foreign_keys: i32 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).expect("foreign_keys");
    assert_eq!(foreign_keys, 1); // 1 = ON

    let busy_timeout: i64 = conn.query_row("PRAGMA busy_timeout", [], |r| r.get(0)).expect("busy_timeout");
    assert!(busy_timeout >= 5000);
}

#[test]
fn test_error_mapping() {
    let err = rusqlite::Error::QueryReturnedNoRows;
    assert_eq!(map_rusqlite_error(&err), "Requested record not found.");
    assert_eq!(map_status_code(&err), StatusCode::NOT_FOUND);

    let guard = TestDbGuard::new("constraint");
    let conn = init_db(&guard.path).expect("init db");
    // Attempting to insert a pin with non-existent list_id when foreign keys are ON
    let invalid_pin_res = conn.execute(
        "INSERT INTO pins (list_id, title, latitude, longitude, created_at) VALUES (99999, 'Test', 0.0, 0.0, '2026-01-01')",
        [],
    );
    assert!(invalid_pin_res.is_err());
    let db_err = invalid_pin_res.unwrap_err();
    let mapped = map_rusqlite_error(&db_err);
    assert!(mapped.contains("constraint violation") || mapped.contains("FOREIGN KEY"));
    assert_eq!(map_status_code(&db_err), StatusCode::BAD_REQUEST);
}

#[test]
fn test_init_and_seed_default_list() {
    let conn = init_db(":memory:").expect("init in-memory db");
    let lists = list_lists(&conn, None).expect("list lists");
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].id, 1);
    assert_eq!(lists[0].name, "My Bucket List");
    assert_eq!(lists[0].icon, "📍");
}

#[test]
fn test_sqlite_repository_trait_crud() {
    let repo = SqliteRepository::open(":memory:").expect("open repo");
    assert!(repo.raw_connection().lock().is_ok());

    // Create list
    let req = CreateListRequest {
        name: "Japan 2026".to_string(),
        icon: Some("🗾".to_string()),
    };
    let created = repo.create_list(&req, "test-user").expect("create list");
    assert_eq!(created.name, "Japan 2026");

    let fetched = repo.get_list(created.id).expect("get list").expect("some");
    assert_eq!(fetched.name, "Japan 2026");

    // Update list
    let update_req = UpdateListRequest {
        name: Some("Tokyo 2026".to_string()),
        icon: Some("🗼".to_string()),
    };
    let updated = repo
        .update_list(created.id, &update_req)
        .expect("update list")
        .expect("updated");
    assert_eq!(updated.name, "Tokyo 2026");

    let pin_req = CreatePinRequest {
        list_id: Some(created.id),
        title: "Tokyo Tower".to_string(),
        description: None,
        latitude: 35.6586,
        longitude: 139.7454,
        category: Some("Sightseeing".to_string()),
        source_url: None,
        image_url: None,
        address: Some("Minato City, Tokyo".to_string()),
        notes: None,
        visited: Some(false), ..Default::default()
    };
    let pin = repo.create_pin(&pin_req).expect("create pin");
    assert_eq!(pin.list_id, created.id);

    let pins = repo
        .list_pins(&ListPinsQuery {
            list_id: Some(created.id),
            category: None,
            visited: None,
            search: None, ..Default::default()
        }, "test-user")
        .expect("list pins");
    assert_eq!(pins.len(), 1);

    let cats = repo.get_categories(Some(created.id), "test-user").expect("categories");
    assert_eq!(cats, vec!["Sightseeing".to_string()]);

    let deleted = repo.delete_list(created.id).expect("delete list");
    assert!(deleted);
}

#[test]
fn test_in_memory_storage_engine() {
    let repo = InMemoryStorage::new();
    let lists = repo.list_lists("test-user").unwrap();
    assert_eq!(lists.len(), 1);

    let req = CreateListRequest {
        name: "Iceland".to_string(),
        icon: Some("🇮🇸".to_string()),
    };
    let list = repo.create_list(&req, "test-user").unwrap();
    assert_eq!(list.name, "Iceland");

    let pin_req = CreatePinRequest {
        list_id: Some(list.id),
        title: "Blue Lagoon".to_string(),
        description: Some("Geothermal spa".to_string()),
        latitude: 63.8804,
        longitude: -22.4495,
        category: Some("Relaxation".to_string()),
        source_url: None,
        image_url: None,
        address: Some("Grindavik".to_string()),
        notes: None,
        visited: Some(false), ..Default::default()
    };
    let pin = repo.create_pin(&pin_req).unwrap();
    assert_eq!(pin.title, "Blue Lagoon");

    let toggled = repo.toggle_visited(pin.id).unwrap().unwrap();
    assert!(toggled.visited);

    let pins = repo
        .list_pins(&ListPinsQuery {
            list_id: Some(list.id),
            category: None,
            visited: Some(true),
            search: Some("Lagoon".to_string()), ..Default::default()
        }, "test-user")
        .unwrap();
    assert_eq!(pins.len(), 1);
}

#[test]
fn test_sqlite_from_arc() {
    let conn = init_db(":memory:").unwrap();
    let arc_conn = Arc::new(Mutex::new(conn));
    let repo = SqliteRepository::from_arc(arc_conn);
    let lists = repo.list_lists("test-user").unwrap();
    assert_eq!(lists.len(), 1);
}

#[test]
fn test_list_crud_operations() {
    let conn = init_db(":memory:").expect("init db");

    // Create list
    let req = CreateListRequest {
        name: "Japan 2026".to_string(),
        icon: Some("🗾".to_string()),
    };
    let created = create_list(&conn, &req, None).expect("create list");
    assert_eq!(created.name, "Japan 2026");
    assert_eq!(created.icon, "🗾");

    // Get list
    let fetched = get_list(&conn, created.id).expect("get list").expect("found list");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "Japan 2026");

    // Update list
    let update_req = UpdateListRequest {
        name: Some("Tokyo 2026".to_string()),
        icon: Some("🗼".to_string()),
    };
    let updated = update_list(&conn, created.id, &update_req).expect("update list").expect("updated");
    assert_eq!(updated.name, "Tokyo 2026");
    assert_eq!(updated.icon, "🗼");

    // Update list with partial fields (no icon change)
    let update_req2 = UpdateListRequest {
        name: Some("Tokyo & Kyoto 2026".to_string()),
        icon: None,
    };
    let updated2 = update_list(&conn, created.id, &update_req2).expect("update list").expect("updated");
    assert_eq!(updated2.name, "Tokyo & Kyoto 2026");
    assert_eq!(updated2.icon, "🗼");

    // Delete list
    let deleted = delete_list(&conn, created.id).expect("delete list");
    assert!(deleted);
    assert!(get_list(&conn, created.id).expect("get deleted list").is_none());
}

#[test]
fn test_list_deletion_cascades_pins() {
    let conn = init_db(":memory:").expect("init db");

    let list = create_list(
        &conn,
        &CreateListRequest {
            name: "Road Trip".to_string(),
            icon: Some("🚗".to_string()),
        },
        None,
    )
    .expect("create list");

    let pin_req = CreatePinRequest {
        list_id: Some(list.id),
        title: "Route 66 Motel".to_string(),
        description: None,
        latitude: 35.0,
        longitude: -100.0,
        category: Some("Hotel & Stay".to_string()),
        source_url: None,
        image_url: None,
        address: None,
        notes: None,
        visited: Some(false), ..Default::default()
    };
    create_pin(&conn, &pin_req).expect("create pin");

    let pins_before = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: Some(list.id),
            category: None,
            visited: None,
            search: None, ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(pins_before.len(), 1);

    delete_list(&conn, list.id).expect("delete list");

    let pins_after = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: Some(list.id),
            category: None,
            visited: None,
            search: None, ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(pins_after.len(), 0);
}

#[test]
fn test_concurrent_reads_and_writes() {
    let guard = TestDbGuard::new("concurrency");
    let conn = init_db(&guard.path).expect("init db");

    let num_threads = 4;
    let mut handles = Vec::new();

    for i in 0..num_threads {
        let path = guard.path.clone();
        let handle = std::thread::spawn(move || {
            let thread_conn = Connection::open(&path).expect("open thread db");
            configure_pragmas(&thread_conn).expect("configure pragmas");

            for j in 0..10 {
                let req = CreatePinRequest {
                    list_id: Some(1),
                    title: format!("Thread {} Pin {}", i, j),
                    description: Some("Concurrent pin".to_string()),
                    latitude: 35.0 + (i as f64) * 0.1,
                    longitude: 139.0 + (j as f64) * 0.1,
                    category: Some("Sightseeing".to_string()),
                    source_url: None,
                    image_url: None,
                    address: None,
                    notes: None,
                    visited: Some(false),
                    ..Default::default()
                };
                create_pin(&thread_conn, &req).expect("create concurrent pin");
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("thread join");
    }

    let pins = list_pins(&conn, &ListPinsQuery {
        list_id: Some(1),
        category: None,
        visited: None,
        search: None, ..Default::default()
    }, None).expect("list pins");

    assert_eq!(pins.len(), num_threads * 10);
}

#[test]
fn test_pin_crud_and_toggle_visited() {
    let conn = init_db(":memory:").expect("init db");

    // Create pin
    let pin_req = CreatePinRequest {
        list_id: Some(1),
        title: "Ramen Street".to_string(),
        description: Some("Tasty noodles".to_string()),
        latitude: 35.6812,
        longitude: 139.7671,
        category: Some("Food & Drink".to_string()),
        source_url: Some("https://example.com/ramen".to_string()),
        image_url: Some("https://example.com/ramen.jpg".to_string()),
        address: Some("Tokyo Station".to_string()),
        notes: Some("Try the tsukemen".to_string()),
        visited: Some(false), ..Default::default()
    };
    let created_pin = create_pin(&conn, &pin_req).expect("create pin");
    assert_eq!(created_pin.title, "Ramen Street");
    assert!(!created_pin.visited);
    assert_eq!(created_pin.category, "Food & Drink");

    // Get pin
    let fetched = get_pin(&conn, created_pin.id).expect("get pin").expect("found pin");
    assert_eq!(fetched.id, created_pin.id);
    assert_eq!(fetched.title, "Ramen Street");
    assert_eq!(fetched.address, Some("Tokyo Station".to_string()));

    // Update pin
    let update_req = UpdatePinRequest {
        list_id: None,
        title: Some("Tokyo Ramen Street (Updated)".to_string()),
        description: None,
        latitude: None,
        longitude: None,
        category: Some("Food & Drink".to_string()),
        source_url: None,
        image_url: None,
        address: None,
        notes: Some("Special miso ramen".to_string()),
        visited: None, ..Default::default()
    };
    let updated = update_pin(&conn, created_pin.id, &update_req)
        .expect("update pin")
        .expect("updated pin");
    assert_eq!(updated.title, "Tokyo Ramen Street (Updated)");
    assert_eq!(updated.notes, Some("Special miso ramen".to_string()));
    assert_eq!(updated.description, Some("Tasty noodles".to_string()));

    // Toggle visited
    let toggled1 = toggle_visited(&conn, created_pin.id).expect("toggle").expect("toggled pin");
    assert!(toggled1.visited);
    let toggled2 = toggle_visited(&conn, created_pin.id).expect("toggle").expect("toggled pin");
    assert!(!toggled2.visited);

    // Delete pin
    let deleted = delete_pin(&conn, created_pin.id).expect("delete pin");
    assert!(deleted);
    assert!(get_pin(&conn, created_pin.id).expect("get deleted pin").is_none());
}

#[test]
fn test_pin_filtering_and_search() {
    let conn = init_db(":memory:").expect("init db");

    // Insert multiple pins
    let pins_data = vec![
        ("Eiffel Tower", "Sightseeing", 48.8584, 2.2945, false, "Champ de Mars, Paris", "Iconic tower"),
        ("Louvre Museum", "Sightseeing", 48.8606, 2.3376, true, "Rue de Rivoli, Paris", "Mona Lisa"),
        ("Le Comptoir", "Food & Drink", 48.8519, 2.3387, false, "Carrefour de l'Odeon, Paris", "French bistro"),
        ("Cafe de Flore", "Cafe", 48.8540, 2.3325, true, "Boulevard Saint-Germain, Paris", "Historic cafe"),
    ];

    for (title, cat, lat, lon, visited, address, notes) in pins_data {
        create_pin(
            &conn,
            &CreatePinRequest {
                list_id: Some(1),
                title: title.to_string(),
                description: Some(format!("Desc for {}", title)),
                latitude: lat,
                longitude: lon,
                category: Some(cat.to_string()),
                source_url: None,
                image_url: None,
                address: Some(address.to_string()),
                notes: Some(notes.to_string()),
                visited: Some(visited),
                ..Default::default()
            },
        )
        .expect("create pin");
    }

    // Test category filter
    let sightseeing = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: Some("Sightseeing".to_string()),
            visited: None,
            search: None, ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(sightseeing.len(), 2);

    // Test category 'All' returns all 4
    let all_cat = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: Some("All".to_string()),
            visited: None,
            search: None, ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(all_cat.len(), 4);

    // Test visited filter
    let visited_pins = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: None,
            visited: Some(true),
            search: None, ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(visited_pins.len(), 2);

    let unvisited_pins = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: None,
            visited: Some(false),
            search: None, ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(unvisited_pins.len(), 2);

    // Test search query (title match)
    let search_eiffel = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: Some("Eiffel".to_string()), ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(search_eiffel.len(), 1);
    assert_eq!(search_eiffel[0].title, "Eiffel Tower");

    // Test search query (address match)
    let search_odeon = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: Some("Odeon".to_string()), ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(search_odeon.len(), 1);
    assert_eq!(search_odeon[0].title, "Le Comptoir");

    // Test search query (notes match)
    let search_mona = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: Some("Mona Lisa".to_string()), ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(search_mona.len(), 1);
    assert_eq!(search_mona[0].title, "Louvre Museum");

    // Test search query (no match)
    let search_none = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: None,
            visited: None,
            search: Some("NonExistentKeywordXYZ".to_string()), ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(search_none.len(), 0);

    // Test combined filter: category Cafe + visited true
    let combined = list_pins(
        &conn,
        &ListPinsQuery {
            list_id: None,
            category: Some("Cafe".to_string()),
            visited: Some(true),
            search: None, ..Default::default()
        },
        None,
    )
    .expect("list pins");
    assert_eq!(combined.len(), 1);
    assert_eq!(combined[0].title, "Cafe de Flore");
}

#[test]
fn test_get_categories() {
    let conn = init_db(":memory:").expect("init db");

    // Initial empty categories
    let cats = get_categories(&conn, None, None).expect("get categories");
    assert_eq!(cats.len(), 0);

    // Add pins in different lists and categories
    create_pin(
        &conn,
        &CreatePinRequest {
            list_id: Some(1),
            title: "Place 1".to_string(),
            description: None,
            latitude: 10.0,
            longitude: 10.0,
            category: Some("Nature & Outdoors".to_string()),
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None, ..Default::default()
        },
    )
    .expect("pin 1");

    let list2 = create_list(
        &conn,
        &CreateListRequest {
            name: "Trip 2".to_string(),
            icon: None,
        },
        None,
    )
    .expect("list 2");

    create_pin(
        &conn,
        &CreatePinRequest {
            list_id: Some(list2.id),
            title: "Place 2".to_string(),
            description: None,
            latitude: 20.0,
            longitude: 20.0,
            category: Some("Food & Drink".to_string()),
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None, ..Default::default()
        },
    )
    .expect("pin 2");

    let all_cats = get_categories(&conn, None, None).expect("get all categories");
    assert_eq!(all_cats, vec!["Food & Drink", "Nature & Outdoors"]);

    let list1_cats = get_categories(&conn, Some(1), None).expect("get list 1 categories");
    assert_eq!(list1_cats, vec!["Nature & Outdoors"]);

    let list2_cats = get_categories(&conn, Some(list2.id), None).expect("get list 2 categories");
    assert_eq!(list2_cats, vec!["Food & Drink"]);
}

#[test]
fn test_nonexistent_entity_lookups_and_operations() {
    let conn = init_db(":memory:").expect("init db");

    assert!(get_list(&conn, 9999).expect("get list").is_none());
    assert!(!delete_list(&conn, 9999).expect("delete list"));
    assert!(update_list(
        &conn,
        9999,
        &UpdateListRequest {
            name: Some("Name".to_string()),
            icon: None,
        }
    )
    .expect("update list")
    .is_none());

    assert!(get_pin(&conn, 9999).expect("get pin").is_none());
    assert!(!delete_pin(&conn, 9999).expect("delete pin"));
    assert!(update_pin(
        &conn,
        9999,
        &UpdatePinRequest {
            list_id: None,
            title: Some("Title".to_string()),
            description: None,
            latitude: None,
            longitude: None,
            category: None,
            source_url: None,
            image_url: None,
            address: None,
            notes: None,
            visited: None, ..Default::default()
        }
    )
    .expect("update pin")
    .is_none());
    assert!(toggle_visited(&conn, 9999).expect("toggle visited").is_none());
}

#[test]
fn test_multi_device_sync_and_collaboration() {
    let repo = SqliteRepository::open(":memory:").expect("open memory repo");

    let token_a = "device_token_alice_123";
    let token_b = "device_token_bob_456";
    let token_c = "device_token_charlie_789";

    // Auto-associate device A
    repo.auto_associate_device(token_a).expect("associate A");
    let lists_a = repo.list_lists(token_a).expect("list A");
    assert_eq!(lists_a.len(), 1);
    assert_eq!(lists_a[0].id, 1);

    // Device A creates a new trip collection
    let trip_a = repo
        .create_list(
            &CreateListRequest {
                name: "Kyoto Autumn Trip".to_string(),
                icon: Some("🍁".to_string()),
            },
            token_a,
        )
        .expect("create trip");

    assert_eq!(trip_a.name, "Kyoto Autumn Trip");
    assert_eq!(trip_a.icon, "🍁");
    assert!(!trip_a.share_token.is_empty());

    // Device A has permission, Device B and C do not yet
    assert!(repo.check_permission(token_a, trip_a.id).expect("check perm A"));
    assert!(!repo.check_permission(token_b, trip_a.id).expect("check perm B"));
    assert!(!repo.check_permission(token_c, trip_a.id).expect("check perm C"));

    // Device B joins list using the share token
    let joined = repo
        .join_list(&trip_a.share_token, token_b)
        .expect("join list")
        .expect("found list");
    assert_eq!(joined.id, trip_a.id);
    assert_eq!(joined.name, "Kyoto Autumn Trip");

    // Device B now has permission
    assert!(repo.check_permission(token_b, trip_a.id).expect("check perm B after join"));
    // Device C still has no permission
    assert!(!repo.check_permission(token_c, trip_a.id).expect("check perm C"));

    // Both Device A and Device B see the list in list_lists
    let lists_b = repo.list_lists(token_b).expect("list B");
    assert_eq!(lists_b.len(), 2); // Bob's default list + joined Kyoto trip
    assert!(lists_b.iter().any(|l| l.id == trip_a.id));

    // Device B adds a pin to the shared list
    let pin = repo
        .create_pin(&CreatePinRequest {
            list_id: Some(trip_a.id),
            title: "Fushimi Inari Taisha".to_string(),
            description: Some("Thousands of vermilion torii gates".to_string()),
            latitude: 34.9671,
            longitude: 135.7727,
            category: Some("Sightseeing".to_string()),
            source_url: None,
            image_url: None,
            address: Some("Kyoto, Japan".to_string()),
            notes: Some("Hike to the summit".to_string()),
            visited: Some(false), ..Default::default()
        })
        .expect("create pin B");

    // Device A fetches pins for the shared list and sees Device B's pin
    let pins_a = repo
        .list_pins(
            &ListPinsQuery {
                list_id: Some(trip_a.id),
                category: None,
                visited: None,
                search: None, ..Default::default()
            },
            token_a,
        )
        .expect("list pins A");
    assert_eq!(pins_a.len(), 1);
    assert_eq!(pins_a[0].id, pin.id);
    assert_eq!(pins_a[0].title, "Fushimi Inari Taisha");

    // Device A toggles visited status
    let updated_pin = repo.toggle_visited(pin.id).expect("toggle visited").expect("pin exists");
    assert!(updated_pin.visited);

    // Device B sees the updated visited status
    let pins_b = repo
        .list_pins(
            &ListPinsQuery {
                list_id: Some(trip_a.id),
                category: None,
                visited: None,
                search: None, ..Default::default()
            },
            token_b,
        )
        .expect("list pins B");
    assert_eq!(pins_b.len(), 1);
    assert!(pins_b[0].visited);
}

#[test]
fn test_quota_counts_and_pin_filtering_edge_cases() {
    let repo = SqliteRepository::open(":memory:").expect("open memory repo");
    let token = "user_quota_tester";

    assert_eq!(repo.count_user_lists(token).expect("count lists"), 1); // default list
    assert_eq!(repo.count_user_pins(token).expect("count user pins"), 0);

    let list2 = repo
        .create_list(
            &CreateListRequest {
                name: "Food Trip".to_string(),
                icon: Some("🍜".to_string()),
            },
            token,
        )
        .expect("create list 2");

    assert_eq!(repo.count_user_lists(token).expect("count lists"), 2);

    // Add pins
    repo.create_pin(&CreatePinRequest {
        list_id: Some(list2.id),
        title: "Ramen Street".to_string(),
        description: Some("Tokyo Station subterranean dining".to_string()),
        latitude: 35.6812,
        longitude: 139.7671,
        category: Some("Food & Drink".to_string()),
        source_url: None,
        image_url: None,
        address: Some("Tokyo Station, Tokyo, Japan".to_string()),
        notes: Some("Try Rokurinsha tsukemen".to_string()),
        visited: Some(true), ..Default::default()
    })
    .expect("pin 1");

    repo.create_pin(&CreatePinRequest {
        list_id: Some(list2.id),
        title: "Udon Shin".to_string(),
        description: Some("Freshly made handmade noodles".to_string()),
        latitude: 35.6865,
        longitude: 139.6975,
        category: Some("Food & Drink".to_string()),
        source_url: None,
        image_url: None,
        address: Some("Shinjuku, Tokyo, Japan".to_string()),
        notes: Some("Expect a queue".to_string()),
        visited: Some(false), ..Default::default()
    })
    .expect("pin 2");

    assert_eq!(repo.count_list_pins(list2.id).expect("count list 2 pins"), 2);
    assert_eq!(repo.count_user_pins(token).expect("count user pins"), 2);

    // Search by description case-insensitive
    let search_desc = repo
        .list_pins(
            &ListPinsQuery {
                list_id: Some(list2.id),
                category: None,
                visited: None,
                search: Some("SUBTERRANEAN".to_string()), ..Default::default()
            },
            token,
        )
        .expect("search desc");
    assert_eq!(search_desc.len(), 1);
    assert_eq!(search_desc[0].title, "Ramen Street");

    // Search by notes case-insensitive
    let search_notes = repo
        .list_pins(
            &ListPinsQuery {
                list_id: Some(list2.id),
                category: None,
                visited: None,
                search: Some("rokurinsha".to_string()), ..Default::default()
            },
            token,
        )
        .expect("search notes");
    assert_eq!(search_notes.len(), 1);
    assert_eq!(search_notes[0].title, "Ramen Street");

    // Filter visited = false (bucket list)
    let unvisited = repo
        .list_pins(
            &ListPinsQuery {
                list_id: Some(list2.id),
                category: None,
                visited: Some(false),
                search: None, ..Default::default()
            },
            token,
        )
        .expect("unvisited");
    assert_eq!(unvisited.len(), 1);
    assert_eq!(unvisited[0].title, "Udon Shin");
}

#[test]
fn test_find_duplicate_pin() {
    let conn = init_db(":memory:").expect("init db");
    let repo = SqliteRepository::new(conn);

    let pin1 = repo.create_pin(&CreatePinRequest {
        list_id: Some(1),
        title: "Tokyo Tower".to_string(),
        description: Some("Red tower".to_string()),
        latitude: 35.6586,
        longitude: 139.7454,
        category: Some("Sightseeing".to_string()),
        source_url: Some("https://maps.google.com/?cid=123".to_string()),
        ..Default::default()
    }).expect("create pin 1");

    // 1. Same source_url
    let dup_source = repo.find_duplicate_pin(
        1,
        "Another Name",
        35.0,
        139.0,
        Some("https://maps.google.com/?cid=123"),
        None,
    ).expect("find duplicate by source");
    assert!(dup_source.is_some());
    assert_eq!(dup_source.unwrap().id, pin1.id);

    // 2. Same coordinates
    let dup_coords = repo.find_duplicate_pin(
        1,
        "Tokyo Tower Copy",
        35.6586,
        139.7454,
        None,
        None,
    ).expect("find duplicate by coords");
    assert!(dup_coords.is_some());
    assert_eq!(dup_coords.unwrap().id, pin1.id);

    // 3. Same title case-insensitively and close coords
    let dup_title = repo.find_duplicate_pin(
        1,
        "  tokyo tower  ",
        35.6588,
        139.7456,
        None,
        None,
    ).expect("find duplicate by title and proximity");
    assert!(dup_title.is_some());
    assert_eq!(dup_title.unwrap().id, pin1.id);

    // 4. Exclude self ID during update
    let self_dup = repo.find_duplicate_pin(
        1,
        "Tokyo Tower",
        35.6586,
        139.7454,
        Some("https://maps.google.com/?cid=123"),
        Some(pin1.id),
    ).expect("exclude self");
    assert!(self_dup.is_none());

    // 5. Different list ID
    let diff_list = repo.find_duplicate_pin(
        2,
        "Tokyo Tower",
        35.6586,
        139.7454,
        Some("https://maps.google.com/?cid=123"),
        None,
    ).expect("diff list");
    assert!(diff_list.is_none());
}

#[test]
fn test_user_profile_and_collaborators() {
    let guard = TestDbGuard::new("user_profile");
    let repo = SqliteRepository::open(&guard.path).expect("open repo");

    let token_alice = "user-alice-123";
    let token_bob = "user-bob-456";

    // 1. Fetch default profile for new user
    let alice_init = repo.get_user_profile(token_alice).expect("get profile");
    assert_eq!(alice_init.name, "");
    assert_eq!(alice_init.avatar, "🧭");

    // 2. Update Alice profile
    let alice_updated = repo.update_user_profile(
        token_alice,
        &UpdateUserProfileRequest {
            name: Some("Alice Adventurer".to_string()),
            avatar: Some("🦊".to_string()),
            color: Some("#f97316".to_string()),
        },
    ).expect("update profile");
    assert_eq!(alice_updated.name, "Alice Adventurer");
    assert_eq!(alice_updated.avatar, "🦊");
    assert_eq!(alice_updated.color, "#f97316");

    // 3. Update Bob profile
    repo.update_user_profile(
        token_bob,
        &UpdateUserProfileRequest {
            name: Some("Bob Backpacker".to_string()),
            avatar: Some("🎒".to_string()),
            color: Some("#10b981".to_string()),
        },
    ).expect("update bob");

    // 4. Create a shared list owned by Alice
    let list = repo.create_list(
        &CreateListRequest {
            name: "Japan Trip 2026".to_string(),
            icon: Some("⛩️".to_string()),
        },
        token_alice,
    ).expect("create list");

    // 5. Bob joins Alice's list via share token
    let joined = repo.join_list(&list.share_token, token_bob).expect("join list");
    assert!(joined.is_some());

    // 6. Query collaborators for the list
    let collabs = repo.get_list_collaborators(list.id).expect("get collaborators");
    assert_eq!(collabs.len(), 2);

    let owner = collabs.iter().find(|c| c.is_owner).expect("owner exists");
    assert_eq!(owner.name, "Alice Adventurer");
    assert_eq!(owner.avatar, "🦊");

    let guest = collabs.iter().find(|c| !c.is_owner).expect("guest exists");
    assert_eq!(guest.name, "Bob Backpacker");
    assert_eq!(guest.avatar, "🎒");
}

