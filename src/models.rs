use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct List {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub created_at: String,
    pub owner_token: String,
    pub share_token: String,
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
pub struct JoinListRequest {
    pub share_token: String,
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
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub priority: bool,
    #[serde(default)]
    pub day_group: i64,
    #[serde(default)]
    pub custom_order: i64,
    #[serde(default)]
    pub opening_hours: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub visited: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatePinRequest {
    pub list_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub category: Option<String>,
    pub emoji: Option<String>,
    pub tags: Option<String>,
    pub priority: Option<bool>,
    pub day_group: Option<i64>,
    pub custom_order: Option<i64>,
    pub opening_hours: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub visited: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdatePinRequest {
    pub list_id: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub category: Option<String>,
    pub emoji: Option<String>,
    pub tags: Option<String>,
    pub priority: Option<bool>,
    pub day_group: Option<i64>,
    pub custom_order: Option<i64>,
    pub opening_hours: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub visited: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestRequest {
    pub url: String,
    pub list_id: Option<i64>,
    pub category: Option<String>,
    pub emoji: Option<String>,
    pub tags: Option<String>,
    pub priority: Option<bool>,
    pub day_group: Option<i64>,
    pub opening_hours: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportItem {
    pub title: String,
    pub description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub category: Option<String>,
    pub emoji: Option<String>,
    pub tags: Option<String>,
    pub priority: Option<bool>,
    pub day_group: Option<i64>,
    pub opening_hours: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub visited: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportPayload {
    pub list_id: Option<i64>,
    pub new_list_name: Option<String>,
    pub default_category: Option<String>,
    pub items: Option<Vec<ImportItem>>,
    pub raw_data: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub list_id: i64,
    pub list_name: String,
    pub total_processed: usize,
    pub imported_count: usize,
    pub skipped_count: usize,
    pub warnings: Vec<String>,
    pub created_pins: Vec<Pin>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ScrapedMetadata {
    pub title: String,
    pub description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub address: Option<String>,
    pub image_url: Option<String>,
    pub opening_hours: Option<String>,
    pub source_url: String,
    pub source_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListPinsQuery {
    pub list_id: Option<i64>,
    pub category: Option<String>,
    pub visited: Option<bool>,
    pub priority: Option<bool>,
    pub tag: Option<String>,
    pub day_group: Option<i64>,
    pub search: Option<String>,
}

pub use fly_common::models::ApiResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok() {
        let resp = ApiResponse::ok("data_value");
        assert!(resp.success);
        assert_eq!(resp.data, Some("data_value"));
        assert_eq!(resp.error, None);

        let json = serde_json::to_string(&resp).expect("serialize ok response");
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":\"data_value\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_api_response_err() {
        let resp: ApiResponse<String> = ApiResponse::err("something went wrong");
        assert!(!resp.success);
        assert_eq!(resp.data, None);
        assert_eq!(resp.error, Some("something went wrong".to_string()));

        let json = serde_json::to_string(&resp).expect("serialize err response");
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"something went wrong\""));
        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn test_list_and_requests_serde() {
        let list = List {
            id: 42,
            name: "Euro Summer".to_string(),
            icon: "🏖️".to_string(),
            created_at: "2026-08-30T12:00:00Z".to_string(),
            owner_token: "some-owner".to_string(),
            share_token: "some-share".to_string(),
        };
        let json_str = serde_json::to_string(&list).expect("serialize list");
        let deserialized: List = serde_json::from_str(&json_str).expect("deserialize list");
        assert_eq!(deserialized.id, 42);
        assert_eq!(deserialized.name, "Euro Summer");
        assert_eq!(deserialized.icon, "🏖️");
        assert_eq!(deserialized.owner_token, "some-owner");
        assert_eq!(deserialized.share_token, "some-share");

        // CreateListRequest
        let create_req = CreateListRequest {
            name: "Trip".to_string(),
            icon: Some("✈️".to_string()),
        };
        let create_json = serde_json::to_string(&create_req).expect("serialize create");
        let create_de: CreateListRequest = serde_json::from_str(&create_json).expect("deserialize create");
        assert_eq!(create_de.name, "Trip");
        assert_eq!(create_de.icon, Some("✈️".to_string()));

        // UpdateListRequest
        let update_req = UpdateListRequest {
            name: Some("Trip Updated".to_string()),
            icon: None,
        };
        let update_json = serde_json::to_string(&update_req).expect("serialize update");
        let update_de: UpdateListRequest = serde_json::from_str(&update_json).expect("deserialize update");
        assert_eq!(update_de.name, Some("Trip Updated".to_string()));
        assert_eq!(update_de.icon, None);
    }

    #[test]
    fn test_pin_and_requests_serde() {
        let pin = Pin {
            id: 1,
            list_id: 10,
            title: "Mount Fuji".to_string(),
            description: Some("Highest volcano in Japan".to_string()),
            latitude: 35.3606,
            longitude: 138.7274,
            category: "Nature & Outdoors".to_string(),
            emoji: Some("🏔️".to_string()),
            tags: Some("volcano,japan,hiking".to_string()),
            priority: true,
            day_group: 1,
            custom_order: 0,
            opening_hours: Some("Daily 06:00-18:00".to_string()),
            source_url: Some("https://example.com/fuji".to_string()),
            image_url: Some("https://example.com/fuji.jpg".to_string()),
            address: Some("Kitayama, Fujinomiya, Shizuoka".to_string()),
            notes: Some("Best view in autumn".to_string()),
            visited: true,
            created_at: "2026-08-30T12:00:00Z".to_string(),
        };

        let json_str = serde_json::to_string(&pin).expect("serialize pin");
        let deserialized: Pin = serde_json::from_str(&json_str).expect("deserialize pin");
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.list_id, 10);
        assert_eq!(deserialized.title, "Mount Fuji");
        assert_eq!(deserialized.latitude, 35.3606);
        assert_eq!(deserialized.opening_hours, Some("Daily 06:00-18:00".to_string()));
        assert!(deserialized.visited);

        // CreatePinRequest partial JSON deserialization
        let partial_create_json = r#"{"title":"Kyoto Shrine","latitude":35.0,"longitude":135.7}"#;
        let create_req: CreatePinRequest = serde_json::from_str(partial_create_json).expect("deserialize create partial");
        assert_eq!(create_req.title, "Kyoto Shrine");
        assert_eq!(create_req.latitude, 35.0);
        assert_eq!(create_req.longitude, 135.7);
        assert_eq!(create_req.category, None);
        assert_eq!(create_req.visited, None);

        // UpdatePinRequest partial JSON deserialization
        let partial_update_json = r#"{"visited":true,"notes":"Visited during festival"}"#;
        let update_req: UpdatePinRequest = serde_json::from_str(partial_update_json).expect("deserialize update partial");
        assert_eq!(update_req.visited, Some(true));
        assert_eq!(update_req.notes, Some("Visited during festival".to_string()));
        assert_eq!(update_req.title, None);
    }

    #[test]
    fn test_ingest_and_scraper_models_serde() {
        let ingest = IngestRequest {
            url: "https://www.instagram.com/p/ABC123xyz/".to_string(),
            list_id: Some(2),
            category: Some("Social".to_string()),
            emoji: Some("📸".to_string()),
            tags: Some("instagram,cafe".to_string()),
            priority: Some(true),
            day_group: Some(1),
            opening_hours: Some("09:00-21:00".to_string()),
            notes: Some("Saved from reel".to_string()),
        };
        let ingest_json = serde_json::to_string(&ingest).expect("serialize ingest");
        let ingest_de: IngestRequest = serde_json::from_str(&ingest_json).expect("deserialize ingest");
        assert_eq!(ingest_de.url, "https://www.instagram.com/p/ABC123xyz/");
        assert_eq!(ingest_de.list_id, Some(2));
        assert_eq!(ingest_de.opening_hours, Some("09:00-21:00".to_string()));

        let meta = ScrapedMetadata {
            title: "Cozy Coffee Roasters".to_string(),
            description: Some("Specialty coffee in downtown".to_string()),
            latitude: Some(40.7128),
            longitude: Some(-74.0060),
            address: Some("New York, NY".to_string()),
            image_url: Some("https://example.com/img.jpg".to_string()),
            opening_hours: Some("Mon-Sun 07:00-19:00".to_string()),
            source_url: "https://maps.google.com/?q=coffee".to_string(),
            source_type: "google_maps".to_string(),
        };
        let meta_json = serde_json::to_string(&meta).expect("serialize meta");
        let meta_de: ScrapedMetadata = serde_json::from_str(&meta_json).expect("deserialize meta");
        assert_eq!(meta_de.title, "Cozy Coffee Roasters");
        assert_eq!(meta_de.latitude, Some(40.7128));
        assert_eq!(meta_de.opening_hours, Some("Mon-Sun 07:00-19:00".to_string()));
        assert_eq!(meta_de.source_type, "google_maps");

        // ScrapedMetadata default
        let default_meta = ScrapedMetadata::default();
        assert_eq!(default_meta.title, "");
        assert_eq!(default_meta.latitude, None);
        assert_eq!(default_meta.longitude, None);
    }

    #[test]
    fn test_geolocation_and_list_pins_query_serde() {
        let geo = GeoLocation {
            latitude: 51.5074,
            longitude: -0.1278,
            display_name: "London, Greater London, England".to_string(),
        };
        let geo_json = serde_json::to_string(&geo).expect("serialize geo");
        let geo_de: GeoLocation = serde_json::from_str(&geo_json).expect("deserialize geo");
        assert_eq!(geo_de.latitude, 51.5074);
        assert_eq!(geo_de.display_name, "London, Greater London, England");

        let query = ListPinsQuery {
            list_id: Some(1),
            category: Some("Cafe".to_string()),
            visited: Some(false),
            search: Some("espresso".to_string()),
            ..Default::default()
        };
        let query_json = serde_json::to_string(&query).expect("serialize query");
        let query_de: ListPinsQuery = serde_json::from_str(&query_json).expect("deserialize query");
        assert_eq!(query_de.list_id, Some(1));
        assert_eq!(query_de.category, Some("Cafe".to_string()));
        assert_eq!(query_de.visited, Some(false));
        assert_eq!(query_de.search, Some("espresso".to_string()));
    }
}
