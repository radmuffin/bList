use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::sync::{Arc, Mutex};

use super::{ListRepository, PinRepository, StorageError, UserRepository};
use crate::models::{
    Collaborator, CreateListRequest, CreatePinRequest, List, ListPinsQuery, Pin,
    UpdateListRequest, UpdatePinRequest, UpdateUserProfileRequest, UserProfile,
};

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
        let user_list_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM device_lists WHERE user_token = ?",
            params![user_token],
            |r| r.get(0),
        )?;
        if user_list_count == 0 {
            // Check if there is an unassociated/unclaimed list #1
            let unassociated_list1: i64 = conn.query_row(
                "SELECT COUNT(*) FROM lists l WHERE l.id = 1 AND (l.owner_token = '' OR l.owner_token IS NULL) AND NOT EXISTS (SELECT 1 FROM device_lists dl WHERE dl.list_id = 1)",
                [],
                |r| r.get(0),
            ).unwrap_or(0);

            if unassociated_list1 > 0 {
                conn.execute(
                    "UPDATE lists SET owner_token = ? WHERE id = 1 AND (owner_token = '' OR owner_token IS NULL)",
                    params![user_token],
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO device_lists (user_token, list_id) VALUES (?, 1)",
                    params![user_token],
                )?;
            } else {
                // Auto-provision a default "My Bucket List" for this new user
                let created_at = Utc::now().to_rfc3339();
                let share_token = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO lists (name, icon, created_at, owner_token, share_token) VALUES ('My Bucket List', '📍', ?, ?, ?)",
                    params![created_at, user_token, share_token],
                )?;
                let new_list_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO device_lists (user_token, list_id) VALUES (?, ?)",
                    params![user_token, new_list_id],
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

    fn find_duplicate_pin(
        &self,
        list_id: i64,
        title: &str,
        lat: f64,
        lon: f64,
        source_url: Option<&str>,
        exclude_id: Option<i64>,
    ) -> Result<Option<Pin>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        find_duplicate_pin(&conn, list_id, title, lat, lon, source_url, exclude_id)
            .map_err(Into::into)
    }

    fn create_pin(&self, req: &CreatePinRequest) -> Result<Pin, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        create_pin(&conn, req).map_err(Into::into)
    }

    fn create_pins_batch(&self, list_id: i64, pins: &[CreatePinRequest]) -> Result<Vec<Pin>, StorageError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Lock(e.to_string()))?;
        create_pins_batch(&mut conn, list_id, pins).map_err(Into::into)
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

impl UserRepository for SqliteRepository {
    fn get_user_profile(&self, user_token: &str) -> std::result::Result<UserProfile, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT user_token, name, avatar, color FROM users WHERE user_token = ?")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let profile = stmt.query_row(params![user_token], |row| {
            Ok(UserProfile {
                user_token: row.get(0)?,
                name: row.get(1)?,
                avatar: row.get(2)?,
                color: row.get(3)?,
            })
        }).unwrap_or_else(|_| UserProfile {
            user_token: user_token.to_string(),
            name: "".to_string(),
            avatar: "🧭".to_string(),
            color: "#3b82f6".to_string(),
        });

        Ok(profile)
    }

    fn update_user_profile(
        &self,
        user_token: &str,
        req: &UpdateUserProfileRequest,
    ) -> std::result::Result<UserProfile, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT user_token, name, avatar, color FROM users WHERE user_token = ?")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let current = stmt
            .query_row(params![user_token], |row| {
                Ok(UserProfile {
                    user_token: row.get(0)?,
                    name: row.get(1)?,
                    avatar: row.get(2)?,
                    color: row.get(3)?,
                })
            })
            .unwrap_or_else(|_| UserProfile {
                user_token: user_token.to_string(),
                name: "".to_string(),
                avatar: "🧭".to_string(),
                color: "#3b82f6".to_string(),
            });

        let new_name = req.name.as_deref().unwrap_or(&current.name).trim().to_string();
        let new_avatar = req.avatar.as_deref().unwrap_or(&current.avatar).trim().to_string();
        let new_color = req.color.as_deref().unwrap_or(&current.color).trim().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO users (user_token, name, avatar, color, updated_at) VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(user_token) DO UPDATE SET name = excluded.name, avatar = excluded.avatar, color = excluded.color, updated_at = excluded.updated_at",
            params![user_token, new_name, new_avatar, new_color, now],
        ).map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(UserProfile {
            user_token: user_token.to_string(),
            name: new_name,
            avatar: new_avatar,
            color: new_color,
        })
    }

    fn get_list_collaborators(&self, list_id: i64) -> std::result::Result<Vec<Collaborator>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT dl.user_token, COALESCE(u.name, ''), COALESCE(u.avatar, '🧭'), COALESCE(u.color, '#3b82f6'), \
             CASE WHEN l.owner_token = dl.user_token THEN 1 ELSE 0 END as is_owner \
             FROM device_lists dl \
             LEFT JOIN users u ON dl.user_token = u.user_token \
             LEFT JOIN lists l ON dl.list_id = l.id \
             WHERE dl.list_id = ? ORDER BY is_owner DESC, dl.user_token ASC"
        ).map_err(|e| StorageError::Database(e.to_string()))?;

        let rows = stmt.query_map(params![list_id], |row| {
            let _token: String = row.get(0)?;
            let raw_name: String = row.get(1)?;
            let raw_avatar: String = row.get(2)?;
            let raw_color: String = row.get(3)?;
            let is_owner: i64 = row.get(4)?;

            let name = if raw_name.trim().is_empty() {
                "Traveler".to_string()
            } else {
                raw_name
            };
            let avatar = if raw_avatar.trim().is_empty() {
                "🧭".to_string()
            } else {
                raw_avatar
            };
            let color = if raw_color.trim().is_empty() {
                "#3b82f6".to_string()
            } else {
                raw_color
            };

            Ok(Collaborator {
                name,
                avatar,
                color,
                is_owner: is_owner == 1,
            })
        }).map_err(|e| StorageError::Database(e.to_string()))?;

        let result: Vec<Collaborator> = rows.flatten().collect();
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Rusqlite Direct Database Helpers (Preserving 100% Backward Compatibility)
// ---------------------------------------------------------------------------

pub fn init_db(db_path: &str) -> Result<Connection> {
    let conn = fly_common::db::FlyDb::open(db_path)?;

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
            emoji TEXT,
            tags TEXT,
            priority INTEGER NOT NULL DEFAULT 0,
            day_group INTEGER NOT NULL DEFAULT 0,
            custom_order INTEGER NOT NULL DEFAULT 0,
            opening_hours TEXT,
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

        CREATE TABLE IF NOT EXISTS users (
            user_token TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            avatar TEXT NOT NULL DEFAULT '',
            color TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
        );
        "#,
    )?;

    // Migration check for pins table
    let pin_columns = {
        let mut stmt = conn.prepare("PRAGMA table_info(pins)")?;
        let columns: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .collect();
        columns
    };

    if !pin_columns.contains(&"list_id".to_string()) {
        conn.execute("ALTER TABLE pins ADD COLUMN list_id INTEGER NOT NULL DEFAULT 1 REFERENCES lists(id) ON DELETE CASCADE", [])?;
    }
    if !pin_columns.contains(&"emoji".to_string()) {
        conn.execute("ALTER TABLE pins ADD COLUMN emoji TEXT", [])?;
    }
    if !pin_columns.contains(&"tags".to_string()) {
        conn.execute("ALTER TABLE pins ADD COLUMN tags TEXT", [])?;
    }
    if !pin_columns.contains(&"priority".to_string()) {
        conn.execute("ALTER TABLE pins ADD COLUMN priority INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !pin_columns.contains(&"day_group".to_string()) {
        conn.execute("ALTER TABLE pins ADD COLUMN day_group INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !pin_columns.contains(&"custom_order".to_string()) {
        conn.execute("ALTER TABLE pins ADD COLUMN custom_order INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !pin_columns.contains(&"opening_hours".to_string()) {
        conn.execute("ALTER TABLE pins ADD COLUMN opening_hours TEXT", [])?;
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
        CREATE INDEX IF NOT EXISTS idx_pins_priority ON pins(priority);
        CREATE INDEX IF NOT EXISTS idx_pins_day_group ON pins(day_group);
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

pub fn list_lists(conn: &Connection, user_token: Option<&str>) -> Result<Vec<List>> {
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

pub fn get_list(conn: &Connection, id: i64) -> Result<Option<List>> {
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

pub fn create_list(conn: &Connection, req: &CreateListRequest, user_token: Option<&str>) -> Result<List> {
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
) -> Result<Option<List>> {
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
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM pins WHERE list_id = ?", params![id])?;
    let rows_affected = tx.execute("DELETE FROM lists WHERE id = ?", params![id])?;
    tx.commit()?;
    Ok(rows_affected > 0)
}

pub fn list_pins(conn: &Connection, query: &ListPinsQuery, user_token: Option<&str>) -> Result<Vec<Pin>> {
    let mut sql = String::from(
        "SELECT p.id, p.list_id, p.title, p.description, p.latitude, p.longitude, p.category, p.emoji, p.tags, p.priority, p.day_group, p.custom_order, p.opening_hours, p.source_url, p.image_url, p.address, p.notes, p.visited, p.created_at \
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

    if let Some(prio) = query.priority {
        sql.push_str(" AND p.priority = ?");
        params_vec.push(Box::new(if prio { 1 } else { 0 }));
    }

    if let Some(day) = query.day_group {
        sql.push_str(" AND p.day_group = ?");
        params_vec.push(Box::new(day));
    }

    if let Some(ref tag) = query.tag {
        let trimmed = tag.trim().trim_start_matches('#');
        if !trimmed.is_empty() {
            sql.push_str(" AND (p.tags LIKE ? OR p.tags = ?)");
            params_vec.push(Box::new(format!("%{}%", trimmed)));
            params_vec.push(Box::new(trimmed.to_string()));
        }
    }

    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            sql.push_str(
                " AND (p.title LIKE ? OR p.address LIKE ? OR p.notes LIKE ? OR p.description LIKE ? OR p.tags LIKE ?)",
            );
            let pattern = format!("%{}%", search.trim());
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }
    }

    sql.push_str(" ORDER BY p.custom_order ASC, p.id DESC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;

    let pin_iter = stmt.query_map(params_slice.as_slice(), |row| {
        let priority_int: i32 = row.get(9).unwrap_or(0);
        let visited_int: i32 = row.get(17)?;
        Ok(Pin {
            id: row.get(0)?,
            list_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            latitude: row.get(4)?,
            longitude: row.get(5)?,
            category: row.get(6)?,
            emoji: row.get(7)?,
            tags: row.get(8)?,
            priority: priority_int != 0,
            day_group: row.get(10).unwrap_or(0),
            custom_order: row.get(11).unwrap_or(0),
            opening_hours: row.get(12)?,
            source_url: row.get(13)?,
            image_url: row.get(14)?,
            address: row.get(15)?,
            notes: row.get(16)?,
            visited: visited_int != 0,
            created_at: row.get(18)?,
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
        "SELECT id, list_id, title, description, latitude, longitude, category, emoji, tags, priority, day_group, custom_order, opening_hours, source_url, image_url, address, notes, visited, created_at FROM pins WHERE id = ?",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let priority_int: i32 = row.get(9).unwrap_or(0);
        let visited_int: i32 = row.get(17)?;
        Ok(Some(Pin {
            id: row.get(0)?,
            list_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            latitude: row.get(4)?,
            longitude: row.get(5)?,
            category: row.get(6)?,
            emoji: row.get(7)?,
            tags: row.get(8)?,
            priority: priority_int != 0,
            day_group: row.get(10).unwrap_or(0),
            custom_order: row.get(11).unwrap_or(0),
            opening_hours: row.get(12)?,
            source_url: row.get(13)?,
            image_url: row.get(14)?,
            address: row.get(15)?,
            notes: row.get(16)?,
            visited: visited_int != 0,
            created_at: row.get(18)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn find_duplicate_pin(
    conn: &Connection,
    list_id: i64,
    title: &str,
    lat: f64,
    lon: f64,
    source_url: Option<&str>,
    exclude_id: Option<i64>,
) -> Result<Option<Pin>> {
    let clean_title = title.trim();
    let clean_source = source_url.map(|s| s.trim()).filter(|s| !s.is_empty());

    // 1. If source_url is present, check if another pin in this list has the same source_url
    if let Some(src) = clean_source {
        let mut sql = String::from(
            "SELECT id, list_id, title, description, latitude, longitude, category, emoji, tags, priority, day_group, custom_order, opening_hours, source_url, image_url, address, notes, visited, created_at \
             FROM pins WHERE list_id = ? AND LOWER(TRIM(source_url)) = LOWER(TRIM(?))"
        );
        if let Some(eid) = exclude_id {
            sql.push_str(&format!(" AND id != {}", eid));
        }
        sql.push_str(" LIMIT 1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(params![list_id, src])?;
        if let Some(row) = rows.next()? {
            let priority_int: i32 = row.get(9).unwrap_or(0);
            let visited_int: i32 = row.get(17)?;
            return Ok(Some(Pin {
                id: row.get(0)?,
                list_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                latitude: row.get(4)?,
                longitude: row.get(5)?,
                category: row.get(6)?,
                emoji: row.get(7)?,
                tags: row.get(8)?,
                priority: priority_int != 0,
                day_group: row.get(10).unwrap_or(0),
                custom_order: row.get(11).unwrap_or(0),
                opening_hours: row.get(12)?,
                source_url: row.get(13)?,
                image_url: row.get(14)?,
                address: row.get(15)?,
                notes: row.get(16)?,
                visited: visited_int != 0,
                created_at: row.get(18)?,
            }));
        }
    }

    // 2. Check by coordinates & title in the same list
    let mut sql = String::from(
        "SELECT id, list_id, title, description, latitude, longitude, category, emoji, tags, priority, day_group, custom_order, opening_hours, source_url, image_url, address, notes, visited, created_at \
         FROM pins WHERE list_id = ? AND ( \
            (ABS(latitude - ?) < 0.0001 AND ABS(longitude - ?) < 0.0001) OR \
            (LOWER(TRIM(title)) = LOWER(TRIM(?)) AND ABS(latitude - ?) < 0.001 AND ABS(longitude - ?) < 0.001) \
         )"
    );
    if let Some(eid) = exclude_id {
        sql.push_str(&format!(" AND id != {}", eid));
    }
    sql.push_str(" LIMIT 1");

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![list_id, lat, lon, clean_title, lat, lon])?;
    if let Some(row) = rows.next()? {
        let priority_int: i32 = row.get(9).unwrap_or(0);
        let visited_int: i32 = row.get(17)?;
        return Ok(Some(Pin {
            id: row.get(0)?,
            list_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            latitude: row.get(4)?,
            longitude: row.get(5)?,
            category: row.get(6)?,
            emoji: row.get(7)?,
            tags: row.get(8)?,
            priority: priority_int != 0,
            day_group: row.get(10).unwrap_or(0),
            custom_order: row.get(11).unwrap_or(0),
            opening_hours: row.get(12)?,
            source_url: row.get(13)?,
            image_url: row.get(14)?,
            address: row.get(15)?,
            notes: row.get(16)?,
            visited: visited_int != 0,
            created_at: row.get(18)?,
        }));
    }

    Ok(None)
}

pub fn create_pin(conn: &Connection, req: &CreatePinRequest) -> Result<Pin> {
    let created_at = Utc::now().to_rfc3339();
    let list_id = req.list_id.unwrap_or(1);
    let category = req.category.clone().unwrap_or_else(|| "General".to_string());
    let visited_int = if req.visited.unwrap_or(false) { 1 } else { 0 };
    let priority_int = if req.priority.unwrap_or(false) { 1 } else { 0 };
    let day_group = req.day_group.unwrap_or(0);
    let custom_order = req.custom_order.unwrap_or(0);

    conn.execute(
        r#"
        INSERT INTO pins (list_id, title, description, latitude, longitude, category, emoji, tags, priority, day_group, custom_order, opening_hours, source_url, image_url, address, notes, visited, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            list_id,
            req.title,
            req.description,
            req.latitude,
            req.longitude,
            category,
            req.emoji,
            req.tags,
            priority_int,
            day_group,
            custom_order,
            req.opening_hours,
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
        emoji: req.emoji.clone(),
        tags: req.tags.clone(),
        priority: req.priority.unwrap_or(false),
        day_group,
        custom_order,
        opening_hours: req.opening_hours.clone(),
        source_url: req.source_url.clone(),
        image_url: req.image_url.clone(),
        address: req.address.clone(),
        notes: req.notes.clone(),
        visited: req.visited.unwrap_or(false),
        created_at,
    })
}

pub fn create_pins_batch(
    conn: &mut Connection,
    list_id: i64,
    pins: &[CreatePinRequest],
) -> Result<Vec<Pin>> {
    let tx = conn.transaction()?;
    let created_at = Utc::now().to_rfc3339();
    let mut inserted_pins = Vec::with_capacity(pins.len());

    {
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO pins (list_id, title, description, latitude, longitude, category, emoji, tags, priority, day_group, custom_order, opening_hours, source_url, image_url, address, notes, visited, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )?;

        for req in pins {
            let category = req.category.as_deref().unwrap_or("General");
            let visited_int = if req.visited.unwrap_or(false) { 1 } else { 0 };
            let priority_int = if req.priority.unwrap_or(false) { 1 } else { 0 };
            let day_group = req.day_group.unwrap_or(0);
            let custom_order = req.custom_order.unwrap_or(0);

            stmt.execute(params![
                list_id,
                req.title.trim(),
                req.description,
                req.latitude,
                req.longitude,
                category,
                req.emoji,
                req.tags,
                priority_int,
                day_group,
                custom_order,
                req.opening_hours,
                req.source_url,
                req.image_url,
                req.address,
                req.notes,
                visited_int,
                created_at
            ])?;

            let id = tx.last_insert_rowid();
            inserted_pins.push(Pin {
                id,
                list_id,
                title: req.title.trim().to_string(),
                description: req.description.clone(),
                latitude: req.latitude,
                longitude: req.longitude,
                category: category.to_string(),
                emoji: req.emoji.clone(),
                tags: req.tags.clone(),
                priority: req.priority.unwrap_or(false),
                day_group,
                custom_order,
                opening_hours: req.opening_hours.clone(),
                source_url: req.source_url.clone(),
                image_url: req.image_url.clone(),
                address: req.address.clone(),
                notes: req.notes.clone(),
                visited: req.visited.unwrap_or(false),
                created_at: created_at.clone(),
            });
        }
    }

    tx.commit()?;
    Ok(inserted_pins)
}

pub fn update_pin(
    conn: &Connection,
    id: i64,
    req: &UpdatePinRequest,
) -> Result<Option<Pin>> {
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
    let emoji = match &req.emoji {
        Some(e) => Some(e.clone()),
        None => existing.emoji,
    };
    let tags = match &req.tags {
        Some(t) => Some(t.clone()),
        None => existing.tags,
    };
    let priority = req.priority.unwrap_or(existing.priority);
    let priority_int = if priority { 1 } else { 0 };
    let day_group = req.day_group.unwrap_or(existing.day_group);
    let custom_order = req.custom_order.unwrap_or(existing.custom_order);
    let opening_hours = match &req.opening_hours {
        Some(o) => Some(o.clone()),
        None => existing.opening_hours,
    };

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
        SET list_id = ?, title = ?, description = ?, latitude = ?, longitude = ?, category = ?, emoji = ?, tags = ?, priority = ?, day_group = ?, custom_order = ?, opening_hours = ?, source_url = ?, image_url = ?, address = ?, notes = ?, visited = ?
        WHERE id = ?
        "#,
        params![
            list_id,
            title,
            description,
            latitude,
            longitude,
            category,
            emoji,
            tags,
            priority_int,
            day_group,
            custom_order,
            opening_hours,
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

pub fn get_categories(conn: &Connection, list_id: Option<i64>, user_token: Option<&str>) -> Result<Vec<String>> {
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
