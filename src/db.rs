use chrono::Utc;
use rusqlite::{params, Connection, Result};
use crate::models::{
    CreateListRequest, CreatePinRequest, List, ListPinsQuery, Pin, UpdateListRequest,
    UpdatePinRequest,
};

pub fn init_db(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Enable WAL mode for concurrency
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;

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
            list_id INTEGER NOT NULL DEFAULT 1,
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
            "ALTER TABLE pins ADD COLUMN list_id INTEGER NOT NULL DEFAULT 1",
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
    // Delete pins associated with this list
    conn.execute("DELETE FROM pins WHERE list_id = ?", params![id])?;
    let rows_affected = conn.execute("DELETE FROM lists WHERE id = ?", params![id])?;
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
mod tests {
    use super::*;
    use crate::models::{CreateListRequest, CreatePinRequest, ListPinsQuery, UpdateListRequest};

    #[test]
    fn test_init_and_seed_default_list() {
        let db_file = "test_seed.db";
        let _ = std::fs::remove_file(db_file);
        let conn = init_db(db_file).expect("init db");
        let lists = list_lists(&conn).expect("list lists");
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].id, 1);
        assert_eq!(lists[0].name, "My Bucket List");
        assert_eq!(lists[0].icon, "📍");
        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_list_crud_and_pin_filtering() {
        let db_file = "test_crud.db";
        let _ = std::fs::remove_file(db_file);
        let conn = init_db(db_file).expect("init db");

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

        let _ = std::fs::remove_file(db_file);
    }
}
