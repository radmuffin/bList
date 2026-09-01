pub mod sqlite;
pub mod in_memory;
#[cfg(test)]
pub mod tests;

use axum::http::StatusCode;
use rusqlite::Connection;
use std::fmt;

#[allow(unused_imports)]
pub use in_memory::InMemoryStorage;
#[allow(unused_imports)]
pub use sqlite::{
    create_list, create_pin, create_pins_batch, delete_list, delete_pin, find_duplicate_pin,
    get_categories, get_list, get_pin, init_db, list_lists, list_pins, toggle_visited, update_list,
    update_pin, SqliteRepository,
};

use crate::models::{
    CreateListRequest, CreatePinRequest, List, ListPinsQuery, Pin, UpdateListRequest,
    UpdatePinRequest,
};

/// Configure database connection pragmas for high concurrency and data integrity:
/// - WAL mode for non-blocking reads during writes
/// - synchronous = NORMAL for optimal WAL write throughput
/// - busy_timeout = 5000ms for busy lock retry
/// - foreign_keys = ON for relational integrity
#[allow(dead_code)]
pub fn configure_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    fly_common::db::FlyDb::apply_pragmas(conn)?;
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
    fn find_duplicate_pin(
        &self,
        list_id: i64,
        title: &str,
        lat: f64,
        lon: f64,
        source_url: Option<&str>,
        exclude_id: Option<i64>,
    ) -> Result<Option<Pin>, StorageError>;
    fn create_pin(&self, req: &CreatePinRequest) -> Result<Pin, StorageError>;
    fn create_pins_batch(&self, list_id: i64, pins: &[CreatePinRequest]) -> Result<Vec<Pin>, StorageError>;
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
