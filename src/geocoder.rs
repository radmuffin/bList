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
