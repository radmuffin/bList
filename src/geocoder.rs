use crate::models::GeoLocation;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct NominatimSearchResult {
    lat: String,
    lon: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct NominatimReverseResult {
    display_name: String,
}

pub struct Geocoder {
    client: reqwest::Client,
}

impl Geocoder {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("MapBucketList/0.1.0 (contact@mapbucketlist.local)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    pub async fn geocode(&self, query: &str) -> Result<Option<GeoLocation>, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let encoded = urlencoding::encode(trimmed);
        let url = format!(
            "https://nominatim.openstreetmap.org/search?format=json&q={}&limit=1&addressdetails=1",
            encoded
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to contact geocoding service: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Geocoding service returned status: {}", response.status()));
        }

        let results: Vec<NominatimSearchResult> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse geocoding response: {}", e))?;

        if let Some(first) = results.into_iter().next() {
            let lat: f64 = first.lat.parse().map_err(|_| "Invalid latitude in geocoding response")?;
            let lon: f64 = first.lon.parse().map_err(|_| "Invalid longitude in geocoding response")?;
            Ok(Some(GeoLocation {
                latitude: lat,
                longitude: lon,
                display_name: first.display_name,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Option<String>, String> {
        let url = format!(
            "https://nominatim.openstreetmap.org/reverse?format=json&lat={}&lon={}",
            lat, lon
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to contact reverse geocoding service: {}", e))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let result: Result<NominatimReverseResult, _> = response.json().await;
        match result {
            Ok(res) => Ok(Some(res.display_name)),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_geocode_empty_and_whitespace_query() {
        let geocoder = Geocoder::new();
        assert_eq!(geocoder.geocode("").await.expect("empty query"), None);
        assert_eq!(geocoder.geocode("   ").await.expect("spaces"), None);
        assert_eq!(geocoder.geocode("	
  
").await.expect("tabs/newlines"), None);
    }

    #[test]
    fn test_nominatim_search_result_deserialization() {
        let json_data = r#"[
            {
                "lat": "35.6585805",
                "lon": "139.7454329",
                "display_name": "Tokyo Tower, 4-2-8, Shibakoen, Minato, Tokyo, Japan"
            }
        ]"#;

        let results: Vec<NominatimSearchResult> = serde_json::from_str(json_data).expect("deserialize search result");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].lat, "35.6585805");
        assert_eq!(results[0].lon, "139.7454329");
        assert_eq!(results[0].display_name, "Tokyo Tower, 4-2-8, Shibakoen, Minato, Tokyo, Japan");

        let lat: f64 = results[0].lat.parse().expect("parse lat");
        let lon: f64 = results[0].lon.parse().expect("parse lon");
        assert!((lat - 35.6585805).abs() < 1e-5);
        assert!((lon - 139.7454329).abs() < 1e-5);
    }

    #[test]
    fn test_nominatim_search_result_empty_array() {
        let json_data = "[]";
        let results: Vec<NominatimSearchResult> = serde_json::from_str(json_data).expect("deserialize empty array");
        assert!(results.is_empty());
    }

    #[test]
    fn test_nominatim_search_result_invalid_number() {
        let json_data = r#"[
            {
                "lat": "invalid_lat",
                "lon": "139.7454329",
                "display_name": "Invalid Coord Place"
            }
        ]"#;

        let results: Vec<NominatimSearchResult> = serde_json::from_str(json_data).expect("deserialize search result");
        assert_eq!(results.len(), 1);
        assert!(results[0].lat.parse::<f64>().is_err());
    }

    #[test]
    fn test_nominatim_reverse_result_deserialization() {
        let json_data = r#"{
            "display_name": "Eiffel Tower, 5, Avenue Anatole France, Quartier du Gros-Caillou, Paris, France"
        }"#;

        let result: NominatimReverseResult = serde_json::from_str(json_data).expect("deserialize reverse result");
        assert_eq!(
            result.display_name,
            "Eiffel Tower, 5, Avenue Anatole France, Quartier du Gros-Caillou, Paris, France"
        );
    }

    #[test]
    fn test_url_encoding_for_query() {
        let query = "Café de Flore & Les Deux Magots, Paris";
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://nominatim.openstreetmap.org/search?format=json&q={}&limit=1&addressdetails=1",
            encoded
        );
        assert!(url.contains("Caf%C3%A9%20de%20Flore%20%26%20Les%20Deux%20Magots%2C%20Paris"));
    }
}
