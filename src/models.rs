use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: i64,
    pub list_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub category: String,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub visited: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePinRequest {
    pub list_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub category: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub visited: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePinRequest {
    pub list_id: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub category: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub visited: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub url: String,
    pub list_id: Option<i64>,
    pub category: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ScrapedMetadata {
    pub title: String,
    pub description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub address: Option<String>,
    pub image_url: Option<String>,
    pub source_url: String,
    pub source_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPinsQuery {
    pub list_id: Option<i64>,
    pub category: Option<String>,
    pub visited: Option<bool>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}
