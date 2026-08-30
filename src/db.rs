use chrono::Utc;
use rusqlite::{params, Connection, Result};
use axum::http::StatusCode;
use crate::models::{
    CreateListRequest, CreatePinRequest, List, ListPinsQuery, Pin, UpdateListRequest,
    UpdatePinRequest,
};

/// Configure database connection pragmas for high concurrency and data integrity:
/// - WAL mode for non-blocking reads during writes
/// - synchronous = NORMAL for optimal WAL write throughput
/// - busy_timeout = 5000ms for busy lock retry
/// - foreign_keys = ON for relational integrity
pub fn configure_pragmas(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
    Ok(())
}

/// Map rusqlite database errors to user-friendly, descriptive application error messages.
pub fn map_rusqlite_error(err: &rusqlite::Error) -> String {
    match err {
        rusqlite::Error::SqliteFailure(ffi_err, msg) => match ffi_err.code {
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => {
                "Database is currently busy or locked by another operation. Please retry shortly.".to_string()
            }
            rusqlite::ErrorCode::ConstraintViolation => {
                if let Some(detail) = msg {
                    format!("Database constraint violation: {}", detail)
                } else {
                    "A database constraint was violated (e.g. invalid foreign key or duplicate entry).".to_string()
                }
            }
            rusqlite::ErrorCode::CannotOpen => {
                "Unable to open or access the database file.".to_string()
            }
            rusqlite::ErrorCode::ReadOnly => {
                "Database is in read-only mode.".to_string()
            }
            _ => {
                if let Some(detail) = msg {
                    format!("Database error ({:?}): {}", ffi_err.code, detail)
                } else {
                    format!("Database error: {}", ffi_err)
                }
            }
        },
        rusqlite::Error::QueryReturnedNoRows => "Requested record not found.".to_string(),
        rusqlite::Error::ToSqlConversionFailure(e) => format!("Data encoding error: {}", e),
        rusqlite::Error::FromSqlConversionFailure(idx, ty, e) => {
            format!("Data conversion error at column {} (type {}): {}", idx, ty, e)
        }
        _ => format!("Database operation failed: {}", err),
    }
}

/// Map rusqlite errors to the most appropriate HTTP Status Code.
pub fn map_status_code(err: &rusqlite::Error) -> StatusCode {
    match err {
        rusqlite::Error::SqliteFailure(ffi_err, _) => match ffi_err.code {
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            rusqlite::ErrorCode::ConstraintViolation => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
        rusqlite::Error::QueryReturnedNoRows => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub fn init_db(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Apply robust concurrency pragmas
    configure_pragmas(&conn)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS lists (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            icon TEXT NOT NULL DEFAULT '📍',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            list_id INTEGER NOT NULL DEFAULT 1 REFERENCES lists(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            description TEXT,
            latitude REAL NOT NULL,
            longitude REAL NOT NULL,
            category TEXT NOT NULL DEFAULT 'General',
            source_url TEXT,
            image_url TEXT,
            address TEXT,
            notes TEXT,
            visited INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );
        "#,
    )?;

    // Migration check: verify list_id column exists in existing pins table
    let has_list_id = {
        let mut stmt = conn.prepare("PRAGMA table_info(pins)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut found = false;
        for col in columns {
            if col? == "list_id" {
                found = true;
                break;
            }
        }
        found
    };

    if !has_list_id {
        conn.execute(
            "ALTER TABLE pins ADD COLUMN list_id INTEGER NOT NULL DEFAULT 1 REFERENCES lists(id) ON DELETE CASCADE",
            [],
        )?;
    }

    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_pins_list_id ON pins(list_id);
        CREATE INDEX IF NOT EXISTS idx_pins_category ON pins(category);
        CREATE INDEX IF NOT EXISTS idx_pins_visited ON pins(visited);
        CREATE INDEX IF NOT EXISTS idx_pins_coords ON pins(latitude, longitude);
        "#,
    )?;

    // Seed default list if none exists
    let list_count: i64 = conn.query_row("SELECT COUNT(*) FROM lists", [], |r| r.get(0))?;
    if list_count == 0 {
        let created_at = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO lists (id, name, icon, created_at) VALUES (1, 'My Bucket List', '📍', ?)",
            params![created_at],
        )?;
    }

    Ok(conn)
}

pub fn list_lists(conn: &Connection) -> Result<Vec<List>> {
    let mut stmt = conn.prepare("SELECT id, name, icon, created_at FROM lists ORDER BY id ASC")?;
    let list_iter = stmt.query_map([], |row| {
        Ok(List {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;

    let mut lists = Vec::new();
    for list in list_iter {
        lists.push(list?);
    }
    Ok(lists)
}

pub fn get_list(conn: &Connection, id: i64) -> Result<Option<List>> {
    let mut stmt = conn.prepare("SELECT id, name, icon, created_at FROM lists WHERE id = ?")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(List {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            created_at: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create_list(conn: &Connection, req: &CreateListRequest) -> Result<List> {
    let created_at = Utc::now().to_rfc3339();
    let default_icon = "📍".to_string();
    let icon = match &req.icon {
        Some(i) if !i.trim().is_empty() => i.trim(),
        _ => &default_icon,
    };

    conn.execute(
        "INSERT INTO lists (name, icon, created_at) VALUES (?, ?, ?)",
        params![req.name.trim(), icon, created_at],
    )?;

    let id = conn.last_insert_rowid();

    Ok(List {
        id,
        name: req.name.trim().to_string(),
        icon: icon.to_string(),
        created_at,
    })
}

pub fn update_list(conn: &Connection, id: i64, req: &UpdateListRequest) -> Result<Option<List>> {
    let existing = match get_list(conn, id)? {
        Some(list) => list,
        None => return Ok(None),
    };

    let name = match &req.name {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => &existing.name,
    };
    let icon = match &req.icon {
        Some(i) if !i.trim().is_empty() => i.trim(),
        _ => &existing.icon,
    };

    conn.execute(
        "UPDATE lists SET name = ?, icon = ? WHERE id = ?",
        params![name, icon, id],
    )?;

    get_list(conn, id)
}

pub fn delete_list(conn: &Connection, id: i64) -> Result<bool> {
    // Atomic transaction for deleting pins and list
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM pins WHERE list_id = ?", params![id])?;
    let rows_affected = tx.execute("DELETE FROM lists WHERE id = ?", params![id])?;
    tx.commit()?;
    Ok(rows_affected > 0)
}

pub fn list_pins(conn: &Connection, query: &ListPinsQuery) -> Result<Vec<Pin>> {
    let mut sql = String::from(
        "SELECT id, list_id, title, description, latitude, longitude, category, source_url, image_url, address, notes, visited, created_at FROM pins WHERE 1=1"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(list_id) = query.list_id {
        sql.push_str(" AND list_id = ?");
        params_vec.push(Box::new(list_id));
    }

    if let Some(ref cat) = query.category {
        if !cat.is_empty() && cat != "All" {
            sql.push_str(" AND category = ?");
            params_vec.push(Box::new(cat.clone()));
        }
    }

    if let Some(vis) = query.visited {
        sql.push_str(" AND visited = ?");
        params_vec.push(Box::new(if vis { 1 } else { 0 }));
    }

    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            sql.push_str(" AND (title LIKE ? OR address LIKE ? OR notes LIKE ? OR description LIKE ?)");
            let pattern = format!("%{}%", search.trim());
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }
    }

    sql.push_str(" ORDER BY id DESC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;

    let pin_iter = stmt.query_map(params_slice.as_slice(), |row| {
        let visited_int: i32 = row.get(11)?;
        Ok(Pin {
            id: row.get(0)?,
            list_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            latitude: row.get(4)?,
            longitude: row.get(5)?,
            category: row.get(6)?,
            source_url: row.get(7)?,
            image_url: row.get(8)?,
            address: row.get(9)?,
            notes: row.get(10)?,
            visited: visited_int != 0,
            created_at: row.get(12)?,
        })
    })?;

    let mut pins = Vec::new();
    for pin in pin_iter {
        pins.push(pin?);
    }
    Ok(pins)
}

pub fn get_pin(conn: &Connection, id: i64) -> Result<Option<Pin>> {
    let mut stmt = conn.prepare(
        "SELECT id, list_id, title, description, latitude, longitude, category, source_url, image_url, address, notes, visited, created_at FROM pins WHERE id = ?",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let visited_int: i32 = row.get(11)?;
        Ok(Some(Pin {
            id: row.get(0)?,
            list_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            latitude: row.get(4)?,
            longitude: row.get(5)?,
            category: row.get(6)?,
            source_url: row.get(7)?,
            image_url: row.get(8)?,
            address: row.get(9)?,
            notes: row.get(10)?,
            visited: visited_int != 0,
            created_at: row.get(12)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create_pin(conn: &Connection, req: &CreatePinRequest) -> Result<Pin> {
    let created_at = Utc::now().to_rfc3339();
    let list_id = req.list_id.unwrap_or(1);
    let category = req.category.clone().unwrap_or_else(|| "General".to_string());
    let visited_int = if req.visited.unwrap_or(false) { 1 } else { 0 };

    conn.execute(
        r#"
        INSERT INTO pins (list_id, title, description, latitude, longitude, category, source_url, image_url, address, notes, visited, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            list_id,
            req.title,
            req.description,
            req.latitude,
            req.longitude,
            category,
            req.source_url,
            req.image_url,
            req.address,
            req.notes,
            visited_int,
            created_at
        ],
    )?;

    let id = conn.last_insert_rowid();

    Ok(Pin {
        id,
        list_id,
        title: req.title.clone(),
        description: req.description.clone(),
        latitude: req.latitude,
        longitude: req.longitude,
        category,
        source_url: req.source_url.clone(),
        image_url: req.image_url.clone(),
        address: req.address.clone(),
        notes: req.notes.clone(),
        visited: req.visited.unwrap_or(false),
        created_at,
    })
}

pub fn update_pin(conn: &Connection, id: i64, req: &UpdatePinRequest) -> Result<Option<Pin>> {
    let existing = match get_pin(conn, id)? {
        Some(pin) => pin,
        None => return Ok(None),
    };

    let list_id = req.list_id.unwrap_or(existing.list_id);
    let title = req.title.as_ref().unwrap_or(&existing.title);
    let description = match &req.description {
        Some(d) => Some(d.clone()),
        None => existing.description,
    };
    let latitude = req.latitude.unwrap_or(existing.latitude);
    let longitude = req.longitude.unwrap_or(existing.longitude);
    let category = req.category.as_ref().unwrap_or(&existing.category);
    let source_url = match &req.source_url {
        Some(u) => Some(u.clone()),
        None => existing.source_url,
    };
    let image_url = match &req.image_url {
        Some(i) => Some(i.clone()),
        None => existing.image_url,
    };
    let address = match &req.address {
        Some(a) => Some(a.clone()),
        None => existing.address,
    };
    let notes = match &req.notes {
        Some(n) => Some(n.clone()),
        None => existing.notes,
    };
    let visited = req.visited.unwrap_or(existing.visited);
    let visited_int = if visited { 1 } else { 0 };

    conn.execute(
        r#"
        UPDATE pins
        SET list_id = ?, title = ?, description = ?, latitude = ?, longitude = ?, category = ?, source_url = ?, image_url = ?, address = ?, notes = ?, visited = ?
        WHERE id = ?
        "#,
        params![
            list_id,
            title,
            description,
            latitude,
            longitude,
            category,
            source_url,
            image_url,
            address,
            notes,
            visited_int,
            id
        ],
    )?;

    get_pin(conn, id)
}

pub fn toggle_visited(conn: &Connection, id: i64) -> Result<Option<Pin>> {
    let existing = match get_pin(conn, id)? {
        Some(pin) => pin,
        None => return Ok(None),
    };

    let new_visited = !existing.visited;
    let visited_int = if new_visited { 1 } else { 0 };

    conn.execute(
        "UPDATE pins SET visited = ? WHERE id = ?",
        params![visited_int, id],
    )?;

    get_pin(conn, id)
}

pub fn delete_pin(conn: &Connection, id: i64) -> Result<bool> {
    let rows_affected = conn.execute("DELETE FROM pins WHERE id = ?", params![id])?;
    Ok(rows_affected > 0)
}

pub fn get_categories(conn: &Connection, list_id: Option<i64>) -> Result<Vec<String>> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = match list_id {
        Some(lid) => (
            "SELECT DISTINCT category FROM pins WHERE list_id = ? AND category IS NOT NULL AND category != '' ORDER BY category ASC".to_string(),
            vec![Box::new(lid)],
        ),
        None => (
            "SELECT DISTINCT category FROM pins WHERE category IS NOT NULL AND category != '' ORDER BY category ASC".to_string(),
            Vec::new(),
        ),
    };

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let cat_iter = stmt.query_map(params_slice.as_slice(), |row| row.get::<_, String>(0))?;

    let mut categories = Vec::new();
    for cat in cat_iter {
        categories.push(cat?);
    }
    Ok(categories)
}

#[cfg(test)]
pub struct TestDbGuard {
    pub path: String,
}

#[cfg(test)]
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

#[cfg(test)]
impl Drop for TestDbGuard {
    fn drop(&mut self) {
        Self::cleanup_files(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreateListRequest, CreatePinRequest, ListPinsQuery, UpdateListRequest};

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
        let guard = TestDbGuard::new("seed");
        let conn = init_db(&guard.path).expect("init db");
        let lists = list_lists(&conn).expect("list lists");
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].id, 1);
        assert_eq!(lists[0].name, "My Bucket List");
        assert_eq!(lists[0].icon, "📍");
    }

    #[test]
    fn test_list_crud_and_pin_filtering() {
        let guard = TestDbGuard::new("crud");
        let conn = init_db(&guard.path).expect("init db");

        let req = CreateListRequest {
            name: "Japan 2026".to_string(),
            icon: Some("🗾".to_string()),
        };
        let created = create_list(&conn, &req).expect("create list");
        assert_eq!(created.name, "Japan 2026");
        assert_eq!(created.icon, "🗾");

        let fetched = get_list(&conn, created.id).expect("get list").expect("some list");
        assert_eq!(fetched.name, "Japan 2026");

        let update_req = UpdateListRequest {
            name: Some("Tokyo 2026".to_string()),
            icon: Some("🗼".to_string()),
        };
        let updated = update_list(&conn, created.id, &update_req).expect("update list").expect("updated");
        assert_eq!(updated.name, "Tokyo 2026");
        assert_eq!(updated.icon, "🗼");

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
            visited: Some(false),
        };
        let pin = create_pin(&conn, &pin_req).expect("create pin");
        assert_eq!(pin.list_id, created.id);

        let pins_in_list = list_pins(&conn, &ListPinsQuery {
            list_id: Some(created.id),
            category: None,
            visited: None,
            search: None,
        }).expect("list pins");
        assert_eq!(pins_in_list.len(), 1);
        assert_eq!(pins_in_list[0].title, "Tokyo Tower");

        let deleted = delete_list(&conn, created.id).expect("delete list");
        assert!(deleted);

        let pins_after = list_pins(&conn, &ListPinsQuery {
            list_id: Some(created.id),
            category: None,
            visited: None,
            search: None,
        }).expect("list pins");
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
            search: None,
        }).expect("list pins");

        assert_eq!(pins.len(), num_threads * 10);
    }
}
