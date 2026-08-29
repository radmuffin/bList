use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: i64,
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

#[derive(Debug, Deserialize)]
pub struct CreatePinRequest {
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

#[derive(Debug, Deserialize)]
pub struct UpdatePinRequest {
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

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub url: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListPinsQuery {
    pub category: Option<String>,
    pub visited: Option<bool>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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
