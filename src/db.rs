use chrono::Utc;
use rusqlite::{params, Connection, Result};
use axum::http::StatusCode;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

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
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "busy_timeout", 5000);
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
    Ok(())
}

/// Map rusqlite database errors to user-friendly, descriptive application error messages.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
// Quota & Storage Limits
// ---------------------------------------------------------------------------
pub const MAX_LISTS_PER_USER: usize = 50;
pub const MAX_PINS_PER_LIST: usize = 500;
pub const MAX_PINS_PER_USER: usize = 2500;

// ---------------------------------------------------------------------------
// Repository Traits
// ---------------------------------------------------------------------------

/// Clean interface for bucket list CRUD operations.
pub trait ListRepository: Send + Sync {
    fn list_lists(&self, user_token: &str) -> Result<Vec<List>, StorageError>;
    fn get_list(&self, id: i64) -> Result<Option<List>, StorageError>;
    fn create_list(&self, req: &CreateListRequest, user_token: &str) -> Result<List, StorageError>;
    fn update_list(&self, id: i64, req: &UpdateListRequest) -> Result<Option<List>, StorageError>;
    fn delete_list(&self, id: i64) -> Result<bool, StorageError>;
    fn check_permission(&self, user_token: &str, list_id: i64) -> Result<bool, StorageError>;
    fn join_list(&self, share_token: &str, user_token: &str) -> Result<Option<List>, StorageError>;
    fn auto_associate_device(&self, user_token: &str) -> Result<(), StorageError>;
    fn count_user_lists(&self, user_token: &str) -> Result<usize, StorageError>;
}

/// Clean interface for map pin CRUD, querying, and category operations.
pub trait PinRepository: Send + Sync {
    fn list_pins(&self, query: &ListPinsQuery, user_token: &str) -> Result<Vec<Pin>, StorageError>;
    fn get_pin(&self, id: i64) -> Result<Option<Pin>, StorageError>;
    fn create_pin(&self, req: &CreatePinRequest) -> Result<Pin, StorageError>;
    fn update_pin(&self, id: i64, req: &UpdatePinRequest) -> Result<Option<Pin>, StorageError>;
    fn toggle_visited(&self, id: i64) -> Result<Option<Pin>, StorageError>;
    fn delete_pin(&self, id: i64) -> Result<bool, StorageError>;
    fn get_categories(&self, list_id: Option<i64>, user_token: &str) -> Result<Vec<String>, StorageError>;
    fn count_list_pins(&self, list_id: i64) -> Result<usize, StorageError>;
    fn count_user_pins(&self, user_token: &str) -> Result<usize, StorageError>;
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
    fn list_lists(&self, user_token: &str) -> Result<Vec<List>, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        list_lists(&conn, Some(user_token)).map_err(Into::into)
    }

    fn get_list(&self, id: i64) -> Result<Option<List>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        get_list(&conn, id).map_err(Into::into)
    }

    fn create_list(&self, req: &CreateListRequest, user_token: &str) -> Result<List, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        create_list(&conn, req, Some(user_token)).map_err(Into::into)
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

    fn check_permission(&self, user_token: &str, list_id: i64) -> Result<bool, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM device_lists WHERE user_token = ? AND list_id = ?",
            params![user_token, list_id],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    fn join_list(&self, share_token: &str, user_token: &str) -> Result<Option<List>, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        
        let mut stmt = conn.prepare("SELECT id, name, icon, created_at, owner_token, share_token FROM lists WHERE share_token = ?")?;
        let mut rows = stmt.query(params![share_token])?;
        if let Some(row) = rows.next()? {
            let list = List {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                created_at: row.get(3)?,
                owner_token: row.get(4)?,
                share_token: row.get(5)?,
            };
            conn.execute(
                "INSERT OR IGNORE INTO device_lists (user_token, list_id) VALUES (?, ?)",
                params![user_token, list.id],
            )?;
            Ok(Some(list))
        } else {
            Ok(None)
        }
    }

    fn auto_associate_device(&self, user_token: &str) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM device_lists", [], |r| r.get(0))?;
        if count == 0 {
            let mut stmt = conn.prepare("SELECT id FROM lists")?;
            let list_ids = stmt.query_map([], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<i64>, rusqlite::Error>>()?;
            for lid in list_ids {
                conn.execute(
                    "INSERT OR IGNORE INTO device_lists (user_token, list_id) VALUES (?, ?)",
                    params![user_token, lid],
                )?;
            }
        }
        Ok(())
    }

    fn count_user_lists(&self, user_token: &str) -> Result<usize, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM device_lists WHERE user_token = ?",
            params![user_token],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }
}

impl PinRepository for SqliteRepository {
    fn list_pins(&self, query: &ListPinsQuery, user_token: &str) -> Result<Vec<Pin>, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        list_pins(&conn, query, Some(user_token)).map_err(Into::into)
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

    fn get_categories(&self, list_id: Option<i64>, user_token: &str) -> Result<Vec<String>, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        get_categories(&conn, list_id, Some(user_token)).map_err(Into::into)
    }

    fn count_list_pins(&self, list_id: i64) -> Result<usize, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pins WHERE list_id = ?",
            params![list_id],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }

    fn count_user_pins(&self, user_token: &str) -> Result<usize, StorageError> {
        self.auto_associate_device(user_token)?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pins WHERE list_id IN (SELECT list_id FROM device_lists WHERE user_token = ?)",
            params![user_token],
            |r| r.get(0),
        )?;
        Ok(count as usize)
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
    device_lists: RwLock<Vec<(String, i64)>>,
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
            owner_token: "".to_string(),
            share_token: uuid::Uuid::new_v4().to_string(),
        };
        storage.lists.write().unwrap().insert(1, default_list);
        *storage.next_list_id.write().unwrap() = 2;
        *storage.next_pin_id.write().unwrap() = 1;
        storage
    }
}

impl ListRepository for InMemoryStorage {
    fn list_lists(&self, user_token: &str) -> Result<Vec<List>, StorageError> {
        self.auto_associate_device(user_token)?;
        
        let device_lists = self.device_lists.read().unwrap();
        let lists = self.lists.read().unwrap();
        let mut result = Vec::new();
        for (tok, lid) in device_lists.iter() {
            if tok == user_token {
                if let Some(list) = lists.get(lid) {
                    result.push(list.clone());
                }
            }
        }
        result.sort_by_key(|l| l.id);
        Ok(result)
    }

    fn get_list(&self, id: i64) -> Result<Option<List>, StorageError> {
        let lists = self.lists.read().unwrap();
        Ok(lists.get(&id).cloned())
    }

    fn create_list(&self, req: &CreateListRequest, user_token: &str) -> Result<List, StorageError> {
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
            owner_token: user_token.to_string(),
            share_token: uuid::Uuid::new_v4().to_string(),
        };

        self.lists.write().unwrap().insert(id, list.clone());
        
        // Auto-associate the new list
        self.device_lists.write().unwrap().push((user_token.to_string(), id));
        
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
            
            // Remove device list mappings
            let mut device_lists = self.device_lists.write().unwrap();
            device_lists.retain(|(_, lid)| *lid != id);
        }
        Ok(removed)
    }

    fn check_permission(&self, user_token: &str, list_id: i64) -> Result<bool, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        Ok(device_lists.iter().any(|(tok, lid)| tok == user_token && *lid == list_id))
    }

    fn join_list(&self, share_token: &str, user_token: &str) -> Result<Option<List>, StorageError> {
        self.auto_associate_device(user_token)?;
        let lists = self.lists.read().unwrap();
        if let Some(list) = lists.values().find(|l| l.share_token == share_token) {
            let mut device_lists = self.device_lists.write().unwrap();
            if !device_lists.iter().any(|(tok, lid)| tok == user_token && *lid == list.id) {
                device_lists.push((user_token.to_string(), list.id));
            }
            Ok(Some(list.clone()))
        } else {
            Ok(None)
        }
    }

    fn auto_associate_device(&self, user_token: &str) -> Result<(), StorageError> {
        let mut device_lists = self.device_lists.write().unwrap();
        if device_lists.is_empty() {
            let lists = self.lists.read().unwrap();
            for &id in lists.keys() {
                device_lists.push((user_token.to_string(), id));
            }
        }
        Ok(())
    }

    fn count_user_lists(&self, user_token: &str) -> Result<usize, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let count = device_lists.iter().filter(|(tok, _)| tok == user_token).count();
        Ok(count)
    }
}

impl PinRepository for InMemoryStorage {
    fn list_pins(&self, query: &ListPinsQuery, user_token: &str) -> Result<Vec<Pin>, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let allowed_lists: std::collections::HashSet<i64> = device_lists.iter()
            .filter(|(tok, _)| tok == user_token)
            .map(|(_, lid)| *lid)
            .collect();

        let pins = self.pins.read().unwrap();
        let mut result: Vec<Pin> = pins
            .values()
            .filter(|p| {
                if !allowed_lists.contains(&p.list_id) {
                    return false;
                }
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

    fn get_categories(&self, list_id: Option<i64>, user_token: &str) -> Result<Vec<String>, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let allowed_lists: std::collections::HashSet<i64> = device_lists.iter()
            .filter(|(tok, _)| tok == user_token)
            .map(|(_, lid)| *lid)
            .collect();

        let pins = self.pins.read().unwrap();
        let mut set = std::collections::BTreeSet::new();
        for pin in pins.values() {
            if !allowed_lists.contains(&pin.list_id) {
                continue;
            }
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

    fn count_list_pins(&self, list_id: i64) -> Result<usize, StorageError> {
        let pins = self.pins.read().unwrap();
        let count = pins.values().filter(|p| p.list_id == list_id).count();
        Ok(count)
    }

    fn count_user_pins(&self, user_token: &str) -> Result<usize, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let allowed_lists: std::collections::HashSet<i64> = device_lists.iter()
            .filter(|(tok, _)| tok == user_token)
            .map(|(_, lid)| *lid)
            .collect();
        let pins = self.pins.read().unwrap();
        let count = pins.values().filter(|p| allowed_lists.contains(&p.list_id)).count();
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// Rusqlite Direct Database Helpers (Preserving 100% Backward Compatibility)
// ---------------------------------------------------------------------------

pub fn init_db(db_path: &str) -> Result<Connection> {
    let conn = if db_path == ":memory:" {
        Connection::open_in_memory()?
    } else {
        Connection::open(db_path)?
    };

    // Apply robust concurrency pragmas
    configure_pragmas(&conn)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS lists (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            icon TEXT NOT NULL DEFAULT '📍',
            created_at TEXT NOT NULL,
            owner_token TEXT NOT NULL DEFAULT '',
            share_token TEXT NOT NULL DEFAULT ''
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

        CREATE TABLE IF NOT EXISTS device_lists (
            user_token TEXT NOT NULL,
            list_id INTEGER NOT NULL,
            PRIMARY KEY (user_token, list_id),
            FOREIGN KEY (list_id) REFERENCES lists(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_device_lists_user_token ON device_lists(user_token);
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

    // Migration check for lists table (owner_token, share_token)
    let (has_owner_token, has_share_token) = {
        let mut stmt = conn.prepare("PRAGMA table_info(lists)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_owner = false;
        let mut has_share = false;
        for col in columns {
            let col_name = col?;
            if col_name == "owner_token" {
                has_owner = true;
            } else if col_name == "share_token" {
                has_share = true;
            }
        }
        (has_owner, has_share)
    };

    if !has_owner_token {
        conn.execute("ALTER TABLE lists ADD COLUMN owner_token TEXT NOT NULL DEFAULT ''", [])?;
    }
    if !has_share_token {
        conn.execute("ALTER TABLE lists ADD COLUMN share_token TEXT NOT NULL DEFAULT ''", [])?;
    }

    // Populate share_token with random UUID for any lists where share_token is empty
    {
        let mut stmt = conn.prepare("SELECT id FROM lists WHERE share_token = ''")?;
        let list_ids = stmt.query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<i64>, rusqlite::Error>>()?;
        for id in list_ids {
            let uuid = uuid::Uuid::new_v4().to_string();
            conn.execute("UPDATE lists SET share_token = ? WHERE id = ?", params![uuid, id])?;
        }
    }

    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_pins_list_id ON pins(list_id);
        CREATE INDEX IF NOT EXISTS idx_pins_category ON pins(category);
        CREATE INDEX IF NOT EXISTS idx_pins_visited ON pins(visited);
        CREATE INDEX IF NOT EXISTS idx_pins_coords ON pins(latitude, longitude);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_lists_share_token ON lists(share_token);
        "#,
    )?;

    // Seed default list if none exists
    let list_count: i64 = conn.query_row("SELECT COUNT(*) FROM lists", [], |r| r.get(0))?;
    if list_count == 0 {
        let created_at = Utc::now().to_rfc3339();
        let share_token = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO lists (id, name, icon, created_at, owner_token, share_token) VALUES (1, 'My Bucket List', '📍', ?, '', ?)",
            params![created_at, share_token],
        )?;
    }

    Ok(conn)
}

pub fn list_lists(conn: &Connection, user_token: Option<&str>) -> rusqlite::Result<Vec<List>> {
    let mut sql = String::from(
        "SELECT l.id, l.name, l.icon, l.created_at, l.owner_token, l.share_token \
         FROM lists l"
    );
    let mut params_vec = Vec::new();
    if let Some(tok) = user_token {
        sql.push_str(" INNER JOIN device_lists dl ON l.id = dl.list_id WHERE dl.user_token = ?");
        params_vec.push(tok.to_string());
    }
    sql.push_str(" ORDER BY l.id ASC");

    let mut lists = Vec::new();
    if let Some(tok) = user_token {
        let mut stmt = conn.prepare(&sql)?;
        let list_iter = stmt.query_map(params![tok], |row| {
            Ok(List {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                created_at: row.get(3)?,
                owner_token: row.get(4)?,
                share_token: row.get(5)?,
            })
        })?;
        for list in list_iter {
            lists.push(list?);
        }
    } else {
        let mut stmt = conn.prepare(&sql)?;
        let list_iter = stmt.query_map([], |row| {
            Ok(List {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                created_at: row.get(3)?,
                owner_token: row.get(4)?,
                share_token: row.get(5)?,
            })
        })?;
        for list in list_iter {
            lists.push(list?);
        }
    }
    Ok(lists)
}

pub fn get_list(conn: &Connection, id: i64) -> rusqlite::Result<Option<List>> {
    let mut stmt = conn.prepare("SELECT id, name, icon, created_at, owner_token, share_token FROM lists WHERE id = ?")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(List {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            created_at: row.get(3)?,
            owner_token: row.get(4)?,
            share_token: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create_list(conn: &Connection, req: &CreateListRequest, user_token: Option<&str>) -> rusqlite::Result<List> {
    let created_at = Utc::now().to_rfc3339();
    let default_icon = "📍".to_string();
    let icon = match &req.icon {
        Some(i) if !i.trim().is_empty() => i.trim(),
        _ => &default_icon,
    };
    let owner = user_token.unwrap_or("");
    let share_token = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO lists (name, icon, created_at, owner_token, share_token) VALUES (?, ?, ?, ?, ?)",
        params![req.name.trim(), icon, created_at, owner, share_token],
    )?;

    let id = conn.last_insert_rowid();

    if let Some(tok) = user_token {
        conn.execute(
            "INSERT OR IGNORE INTO device_lists (user_token, list_id) VALUES (?, ?)",
            params![tok, id],
        )?;
    }

    Ok(List {
        id,
        name: req.name.trim().to_string(),
        icon: icon.to_string(),
        created_at,
        owner_token: owner.to_string(),
        share_token,
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

pub fn delete_list(conn: &Connection, id: i64) -> Result<bool> {
    // Atomic transaction for deleting pins and list
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM pins WHERE list_id = ?", params![id])?;
    let rows_affected = tx.execute("DELETE FROM lists WHERE id = ?", params![id])?;
    tx.commit()?;
    Ok(rows_affected > 0)
}

pub fn list_pins(conn: &Connection, query: &ListPinsQuery, user_token: Option<&str>) -> rusqlite::Result<Vec<Pin>> {
    let mut sql = String::from(
        "SELECT p.id, p.list_id, p.title, p.description, p.latitude, p.longitude, p.category, p.source_url, p.image_url, p.address, p.notes, p.visited, p.created_at \
         FROM pins p"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(tok) = user_token {
        sql.push_str(" INNER JOIN device_lists dl ON p.list_id = dl.list_id WHERE dl.user_token = ?");
        params_vec.push(Box::new(tok.to_string()));
    } else {
        sql.push_str(" WHERE 1=1");
    }

    if let Some(list_id) = query.list_id {
        sql.push_str(" AND p.list_id = ?");
        params_vec.push(Box::new(list_id));
    }

    if let Some(ref cat) = query.category {
        if !cat.is_empty() && cat != "All" {
            sql.push_str(" AND p.category = ?");
            params_vec.push(Box::new(cat.clone()));
        }
    }

    if let Some(vis) = query.visited {
        sql.push_str(" AND p.visited = ?");
        params_vec.push(Box::new(if vis { 1 } else { 0 }));
    }

    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            sql.push_str(
                " AND (p.title LIKE ? OR p.address LIKE ? OR p.notes LIKE ? OR p.description LIKE ?)",
            );
            let pattern = format!("%{}%", search.trim());
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }
    }

    sql.push_str(" ORDER BY p.id DESC");

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

pub fn get_categories(conn: &Connection, list_id: Option<i64>, user_token: Option<&str>) -> rusqlite::Result<Vec<String>> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = match (list_id, user_token) {
        (Some(lid), Some(tok)) => (
            "SELECT DISTINCT p.category FROM pins p \
             INNER JOIN device_lists dl ON p.list_id = dl.list_id \
             WHERE dl.user_token = ? AND p.list_id = ? AND p.category IS NOT NULL AND p.category != '' ORDER BY p.category ASC".to_string(),
            vec![Box::new(tok.to_string()), Box::new(lid)],
        ),
        (None, Some(tok)) => (
            "SELECT DISTINCT p.category FROM pins p \
             INNER JOIN device_lists dl ON p.list_id = dl.list_id \
             WHERE dl.user_token = ? AND p.category IS NOT NULL AND p.category != '' ORDER BY p.category ASC".to_string(),
            vec![Box::new(tok.to_string())],
        ),
        (Some(lid), None) => (
            "SELECT DISTINCT category FROM pins WHERE list_id = ? AND category IS NOT NULL AND category != '' ORDER BY category ASC".to_string(),
            vec![Box::new(lid)],
        ),
        (None, None) => (
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
    use crate::models::{
        CreateListRequest, CreatePinRequest, ListPinsQuery, UpdateListRequest, UpdatePinRequest,
    };

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
            visited: Some(false),
        };
        create_pin(&conn, &pin_req).expect("create pin");

        let pins_before = list_pins(
            &conn,
            &ListPinsQuery {
                list_id: Some(list.id),
                category: None,
                visited: None,
                search: None,
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
                search: None,
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
            visited: Some(false),
        };
        let created_pin = create_pin(&conn, &pin_req).expect("create pin");
        assert_eq!(created_pin.title, "Ramen Street");
        assert_eq!(created_pin.visited, false);
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
            visited: None,
        };
        let updated = update_pin(&conn, created_pin.id, &update_req)
            .expect("update pin")
            .expect("updated pin");
        assert_eq!(updated.title, "Tokyo Ramen Street (Updated)");
        assert_eq!(updated.notes, Some("Special miso ramen".to_string()));
        assert_eq!(updated.description, Some("Tasty noodles".to_string()));

        // Toggle visited
        let toggled1 = toggle_visited(&conn, created_pin.id).expect("toggle").expect("toggled pin");
        assert_eq!(toggled1.visited, true);
        let toggled2 = toggle_visited(&conn, created_pin.id).expect("toggle").expect("toggled pin");
        assert_eq!(toggled2.visited, false);

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
                search: None,
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
                search: None,
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
                search: None,
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
                search: None,
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
                search: Some("Eiffel".to_string()),
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
                search: Some("Odeon".to_string()),
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
                search: Some("Mona Lisa".to_string()),
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
                search: Some("NonExistentKeywordXYZ".to_string()),
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
                search: None,
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
                visited: None,
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
                visited: None,
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
                visited: None,
            }
        )
        .expect("update pin")
        .is_none());
        assert!(toggle_visited(&conn, 9999).expect("toggle visited").is_none());
    }
}
