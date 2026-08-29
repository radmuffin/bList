use chrono::Utc;
use rusqlite::{params, Connection, Result};
use crate::models::{CreatePinRequest, ListPinsQuery, Pin, UpdatePinRequest};

pub fn init_db(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Enable WAL mode for concurrency
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS pins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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

        CREATE INDEX IF NOT EXISTS idx_pins_category ON pins(category);
        CREATE INDEX IF NOT EXISTS idx_pins_visited ON pins(visited);
        CREATE INDEX IF NOT EXISTS idx_pins_coords ON pins(latitude, longitude);
        "#,
    )?;

    Ok(conn)
}

pub fn list_pins(conn: &Connection, query: &ListPinsQuery) -> Result<Vec<Pin>> {
    let mut sql = String::from(
        "SELECT id, title, description, latitude, longitude, category, source_url, image_url, address, notes, visited, created_at FROM pins WHERE 1=1"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

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
        let visited_int: i32 = row.get(10)?;
        Ok(Pin {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            latitude: row.get(3)?,
            longitude: row.get(4)?,
            category: row.get(5)?,
            source_url: row.get(6)?,
            image_url: row.get(7)?,
            address: row.get(8)?,
            notes: row.get(9)?,
            visited: visited_int != 0,
            created_at: row.get(11)?,
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
        "SELECT id, title, description, latitude, longitude, category, source_url, image_url, address, notes, visited, created_at FROM pins WHERE id = ?",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let visited_int: i32 = row.get(10)?;
        Ok(Some(Pin {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            latitude: row.get(3)?,
            longitude: row.get(4)?,
            category: row.get(5)?,
            source_url: row.get(6)?,
            image_url: row.get(7)?,
            address: row.get(8)?,
            notes: row.get(9)?,
            visited: visited_int != 0,
            created_at: row.get(11)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create_pin(conn: &Connection, req: &CreatePinRequest) -> Result<Pin> {
    let created_at = Utc::now().to_rfc3339();
    let category = req.category.clone().unwrap_or_else(|| "General".to_string());
    let visited_int = if req.visited.unwrap_or(false) { 1 } else { 0 };

    conn.execute(
        r#"
        INSERT INTO pins (title, description, latitude, longitude, category, source_url, image_url, address, notes, visited, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
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
        SET title = ?, description = ?, latitude = ?, longitude = ?, category = ?, source_url = ?, image_url = ?, address = ?, notes = ?, visited = ?
        WHERE id = ?
        "#,
        params![
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

pub fn get_categories(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT category FROM pins WHERE category IS NOT NULL AND category != '' ORDER BY category ASC")?;
    let cat_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut categories = Vec::new();
    for cat in cat_iter {
        categories.push(cat?);
    }
    Ok(categories)
}
