use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use crate::models::{
    CreateListRequest, CreatePinRequest, List, ListPinsQuery, Pin, UpdateListRequest,
    UpdatePinRequest,
};

// ---------------------------------------------------------------------------
// Storage Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum StorageError {
    Database(String),
    #[allow(dead_code)]
    NotFound(String),
    #[allow(dead_code)]
    Validation(String),
    Lock(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Database(msg) => write!(f, "Database error: {}", msg),
            StorageError::NotFound(msg) => write!(f, "Not found: {}", msg),
            StorageError::Validation(msg) => write!(f, "Validation error: {}", msg),
            StorageError::Lock(msg) => write!(f, "Storage lock error: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Database(e.to_string())
    }
}

impl From<String> for StorageError {
    fn from(msg: String) -> Self {
        StorageError::Database(msg)
    }
}

impl From<&str> for StorageError {
    fn from(msg: &str) -> Self {
        StorageError::Database(msg.to_string())
    }
}

// ---------------------------------------------------------------------------
// Repository Traits
// ---------------------------------------------------------------------------

/// Clean interface for bucket list CRUD operations.
pub trait ListRepository: Send + Sync {
    fn list_lists(&self) -> Result<Vec<List>, StorageError>;
    fn get_list(&self, id: i64) -> Result<Option<List>, StorageError>;
    fn create_list(&self, req: &CreateListRequest) -> Result<List, StorageError>;
    fn update_list(&self, id: i64, req: &UpdateListRequest) -> Result<Option<List>, StorageError>;
    fn delete_list(&self, id: i64) -> Result<bool, StorageError>;
}

/// Clean interface for map pin CRUD, querying, and category operations.
pub trait PinRepository: Send + Sync {
    fn list_pins(&self, query: &ListPinsQuery) -> Result<Vec<Pin>, StorageError>;
    fn get_pin(&self, id: i64) -> Result<Option<Pin>, StorageError>;
    fn create_pin(&self, req: &CreatePinRequest) -> Result<Pin, StorageError>;
    fn update_pin(&self, id: i64, req: &UpdatePinRequest) -> Result<Option<Pin>, StorageError>;
    fn toggle_visited(&self, id: i64) -> Result<Option<Pin>, StorageError>;
    fn delete_pin(&self, id: i64) -> Result<bool, StorageError>;
    fn get_categories(&self, list_id: Option<i64>) -> Result<Vec<String>, StorageError>;
}

/// Unified storage engine interface combining list and pin repositories.
pub trait StorageEngine: ListRepository + PinRepository + Send + Sync {}

impl<T: ListRepository + PinRepository + Send + Sync> StorageEngine for T {}

// ---------------------------------------------------------------------------
// Rusqlite SQLite Storage Implementation
// ---------------------------------------------------------------------------

pub struct SqliteRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    #[allow(dead_code)]
    pub fn from_arc(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn open(db_path: &str) -> Result<Self, StorageError> {
        let conn = init_db(db_path)?;
        Ok(Self::new(conn))
    }

    #[allow(dead_code)]
    pub fn raw_connection(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }
}

impl ListRepository for SqliteRepository {
    fn list_lists(&self) -> Result<Vec<List>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        list_lists(&conn).map_err(Into::into)
    }

    fn get_list(&self, id: i64) -> Result<Option<List>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        get_list(&conn, id).map_err(Into::into)
    }

    fn create_list(&self, req: &CreateListRequest) -> Result<List, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        create_list(&conn, req).map_err(Into::into)
    }

    fn update_list(&self, id: i64, req: &UpdateListRequest) -> Result<Option<List>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        update_list(&conn, id, req).map_err(Into::into)
    }

    fn delete_list(&self, id: i64) -> Result<bool, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        delete_list(&conn, id).map_err(Into::into)
    }
}

impl PinRepository for SqliteRepository {
    fn list_pins(&self, query: &ListPinsQuery) -> Result<Vec<Pin>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        list_pins(&conn, query).map_err(Into::into)
    }

    fn get_pin(&self, id: i64) -> Result<Option<Pin>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        get_pin(&conn, id).map_err(Into::into)
    }

    fn create_pin(&self, req: &CreatePinRequest) -> Result<Pin, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        create_pin(&conn, req).map_err(Into::into)
    }

    fn update_pin(&self, id: i64, req: &UpdatePinRequest) -> Result<Option<Pin>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        update_pin(&conn, id, req).map_err(Into::into)
    }

    fn toggle_visited(&self, id: i64) -> Result<Option<Pin>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        toggle_visited(&conn, id).map_err(Into::into)
    }

    fn delete_pin(&self, id: i64) -> Result<bool, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        delete_pin(&conn, id).map_err(Into::into)
    }

    fn get_categories(&self, list_id: Option<i64>) -> Result<Vec<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        get_categories(&conn, list_id).map_err(Into::into)
    }
}

// ---------------------------------------------------------------------------
// In-Memory Storage Engine (For Unit Testing & Ephemeral Deployments)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Default)]
pub struct InMemoryStorage {
    lists: RwLock<HashMap<i64, List>>,
    pins: RwLock<HashMap<i64, Pin>>,
    next_list_id: RwLock<i64>,
    next_pin_id: RwLock<i64>,
}

impl InMemoryStorage {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let storage = Self::default();
        let default_list = List {
            id: 1,
            name: "My Bucket List".to_string(),
            icon: "📍".to_string(),
            created_at: Utc::now().to_rfc3339(),
        };
        storage.lists.write().unwrap().insert(1, default_list);
        *storage.next_list_id.write().unwrap() = 2;
        *storage.next_pin_id.write().unwrap() = 1;
        storage
    }
}

impl ListRepository for InMemoryStorage {
    fn list_lists(&self) -> Result<Vec<List>, StorageError> {
        let lists = self.lists.read().unwrap();
        let mut result: Vec<List> = lists.values().cloned().collect();
        result.sort_by_key(|l| l.id);
        Ok(result)
    }

    fn get_list(&self, id: i64) -> Result<Option<List>, StorageError> {
        let lists = self.lists.read().unwrap();
        Ok(lists.get(&id).cloned())
    }

    fn create_list(&self, req: &CreateListRequest) -> Result<List, StorageError> {
        let mut next_id = self.next_list_id.write().unwrap();
        let id = *next_id;
        *next_id += 1;

        let icon = match &req.icon {
            Some(i) if !i.trim().is_empty() => i.trim().to_string(),
            _ => "📍".to_string(),
        };

        let list = List {
            id,
            name: req.name.trim().to_string(),
            icon,
            created_at: Utc::now().to_rfc3339(),
        };

        self.lists.write().unwrap().insert(id, list.clone());
        Ok(list)
    }

    fn update_list(&self, id: i64, req: &UpdateListRequest) -> Result<Option<List>, StorageError> {
        let mut lists = self.lists.write().unwrap();
        if let Some(existing) = lists.get_mut(&id) {
            if let Some(ref name) = req.name {
                if !name.trim().is_empty() {
                    existing.name = name.trim().to_string();
                }
            }
            if let Some(ref icon) = req.icon {
                if !icon.trim().is_empty() {
                    existing.icon = icon.trim().to_string();
                }
            }
            Ok(Some(existing.clone()))
        } else {
            Ok(None)
        }
    }

    fn delete_list(&self, id: i64) -> Result<bool, StorageError> {
        let mut lists = self.lists.write().unwrap();
        let removed = lists.remove(&id).is_some();
        if removed {
            let mut pins = self.pins.write().unwrap();
            pins.retain(|_, p| p.list_id != id);
        }
        Ok(removed)
    }
}

impl PinRepository for InMemoryStorage {
    fn list_pins(&self, query: &ListPinsQuery) -> Result<Vec<Pin>, StorageError> {
        let pins = self.pins.read().unwrap();
        let mut result: Vec<Pin> = pins
            .values()
            .filter(|p| {
                if let Some(lid) = query.list_id {
                    if p.list_id != lid {
                        return false;
                    }
                }
                if let Some(ref cat) = query.category {
                    if !cat.is_empty() && cat != "All" && &p.category != cat {
                        return false;
                    }
                }
                if let Some(visited) = query.visited {
                    if p.visited != visited {
                        return false;
                    }
                }
                if let Some(ref search) = query.search {
                    let s = search.trim().to_lowercase();
                    if !s.is_empty() {
                        let title_m = p.title.to_lowercase().contains(&s);
                        let desc_m = p.description.as_deref().unwrap_or("").to_lowercase().contains(&s);
                        let addr_m = p.address.as_deref().unwrap_or("").to_lowercase().contains(&s);
                        let notes_m = p.notes.as_deref().unwrap_or("").to_lowercase().contains(&s);
                        if !title_m && !desc_m && !addr_m && !notes_m {
                            return false;
                        }
                    }
                }
                true
            })
            .cloned()
            .collect();

        result.sort_by_key(|a| std::cmp::Reverse(a.id));
        Ok(result)
    }

    fn get_pin(&self, id: i64) -> Result<Option<Pin>, StorageError> {
        let pins = self.pins.read().unwrap();
        Ok(pins.get(&id).cloned())
    }

    fn create_pin(&self, req: &CreatePinRequest) -> Result<Pin, StorageError> {
        let mut next_id = self.next_pin_id.write().unwrap();
        let id = *next_id;
        *next_id += 1;

        let pin = Pin {
            id,
            list_id: req.list_id.unwrap_or(1),
            title: req.title.clone(),
            description: req.description.clone(),
            latitude: req.latitude,
            longitude: req.longitude,
            category: req.category.clone().unwrap_or_else(|| "General".to_string()),
            source_url: req.source_url.clone(),
            image_url: req.image_url.clone(),
            address: req.address.clone(),
            notes: req.notes.clone(),
            visited: req.visited.unwrap_or(false),
            created_at: Utc::now().to_rfc3339(),
        };

        self.pins.write().unwrap().insert(id, pin.clone());
        Ok(pin)
    }

    fn update_pin(&self, id: i64, req: &UpdatePinRequest) -> Result<Option<Pin>, StorageError> {
        let mut pins = self.pins.write().unwrap();
        if let Some(pin) = pins.get_mut(&id) {
            if let Some(lid) = req.list_id {
                pin.list_id = lid;
            }
            if let Some(ref t) = req.title {
                pin.title = t.clone();
            }
            if req.description.is_some() {
                pin.description = req.description.clone();
            }
            if let Some(lat) = req.latitude {
                pin.latitude = lat;
            }
            if let Some(lon) = req.longitude {
                pin.longitude = lon;
            }
            if let Some(ref cat) = req.category {
                pin.category = cat.clone();
            }
            if req.source_url.is_some() {
                pin.source_url = req.source_url.clone();
            }
            if req.image_url.is_some() {
                pin.image_url = req.image_url.clone();
            }
            if req.address.is_some() {
                pin.address = req.address.clone();
            }
            if req.notes.is_some() {
                pin.notes = req.notes.clone();
            }
            if let Some(v) = req.visited {
                pin.visited = v;
            }
            Ok(Some(pin.clone()))
        } else {
            Ok(None)
        }
    }

    fn toggle_visited(&self, id: i64) -> Result<Option<Pin>, StorageError> {
        let mut pins = self.pins.write().unwrap();
        if let Some(pin) = pins.get_mut(&id) {
            pin.visited = !pin.visited;
            Ok(Some(pin.clone()))
        } else {
            Ok(None)
        }
    }

    fn delete_pin(&self, id: i64) -> Result<bool, StorageError> {
        let mut pins = self.pins.write().unwrap();
        Ok(pins.remove(&id).is_some())
    }

    fn get_categories(&self, list_id: Option<i64>) -> Result<Vec<String>, StorageError> {
        let pins = self.pins.read().unwrap();
        let mut set = std::collections::BTreeSet::new();
        for pin in pins.values() {
            if let Some(lid) = list_id {
                if pin.list_id != lid {
                    continue;
                }
            }
            if !pin.category.trim().is_empty() {
                set.insert(pin.category.clone());
            }
        }
        Ok(set.into_iter().collect())
    }
}

// ---------------------------------------------------------------------------
// Rusqlite Direct Database Helpers (Preserving 100% Backward Compatibility)
// ---------------------------------------------------------------------------

pub fn init_db(db_path: &str) -> rusqlite::Result<Connection> {
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

pub fn list_lists(conn: &Connection) -> rusqlite::Result<Vec<List>> {
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

pub fn get_list(conn: &Connection, id: i64) -> rusqlite::Result<Option<List>> {
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

pub fn create_list(conn: &Connection, req: &CreateListRequest) -> rusqlite::Result<List> {
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

pub fn update_list(
    conn: &Connection,
    id: i64,
    req: &UpdateListRequest,
) -> rusqlite::Result<Option<List>> {
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

pub fn delete_list(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    conn.execute("DELETE FROM pins WHERE list_id = ?", params![id])?;
    let rows_affected = conn.execute("DELETE FROM lists WHERE id = ?", params![id])?;
    Ok(rows_affected > 0)
}

pub fn list_pins(conn: &Connection, query: &ListPinsQuery) -> rusqlite::Result<Vec<Pin>> {
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
            sql.push_str(
                " AND (title LIKE ? OR address LIKE ? OR notes LIKE ? OR description LIKE ?)",
            );
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

pub fn get_pin(conn: &Connection, id: i64) -> rusqlite::Result<Option<Pin>> {
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

pub fn create_pin(conn: &Connection, req: &CreatePinRequest) -> rusqlite::Result<Pin> {
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

pub fn update_pin(
    conn: &Connection,
    id: i64,
    req: &UpdatePinRequest,
) -> rusqlite::Result<Option<Pin>> {
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

pub fn toggle_visited(conn: &Connection, id: i64) -> rusqlite::Result<Option<Pin>> {
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

pub fn delete_pin(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    let rows_affected = conn.execute("DELETE FROM pins WHERE id = ?", params![id])?;
    Ok(rows_affected > 0)
}

pub fn get_categories(conn: &Connection, list_id: Option<i64>) -> rusqlite::Result<Vec<String>> {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    fn test_sqlite_repository_trait_crud() {
        let db_file = "test_sqlite_repo.db";
        let _ = std::fs::remove_file(db_file);
        let repo = SqliteRepository::open(db_file).expect("open repo");
        assert!(repo.raw_connection().lock().is_ok());

        let req = CreateListRequest {
            name: "Japan 2026".to_string(),
            icon: Some("🗾".to_string()),
        };
        let created = repo.create_list(&req).expect("create list");
        assert_eq!(created.name, "Japan 2026");

        let fetched = repo.get_list(created.id).expect("get list").expect("some");
        assert_eq!(fetched.name, "Japan 2026");

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
            visited: Some(false),
        };
        let pin = repo.create_pin(&pin_req).expect("create pin");
        assert_eq!(pin.list_id, created.id);

        let pins = repo
            .list_pins(&ListPinsQuery {
                list_id: Some(created.id),
                category: None,
                visited: None,
                search: None,
            })
            .expect("list pins");
        assert_eq!(pins.len(), 1);

        let cats = repo.get_categories(Some(created.id)).expect("categories");
        assert_eq!(cats, vec!["Sightseeing".to_string()]);

        let deleted = repo.delete_list(created.id).expect("delete list");
        assert!(deleted);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_in_memory_storage_engine() {
        let repo = InMemoryStorage::new();
        let lists = repo.list_lists().unwrap();
        assert_eq!(lists.len(), 1);

        let req = CreateListRequest {
            name: "Iceland".to_string(),
            icon: Some("🇮🇸".to_string()),
        };
        let list = repo.create_list(&req).unwrap();
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
            visited: Some(false),
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
                search: Some("Lagoon".to_string()),
            })
            .unwrap();
        assert_eq!(pins.len(), 1);
    }

    #[test]
    fn test_sqlite_from_arc() {
        let db_file = "test_from_arc.db";
        let _ = std::fs::remove_file(db_file);
        let conn = init_db(db_file).unwrap();
        let arc_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteRepository::from_arc(arc_conn);
        let lists = repo.list_lists().unwrap();
        assert_eq!(lists.len(), 1);
        let _ = std::fs::remove_file(db_file);
    }
}
