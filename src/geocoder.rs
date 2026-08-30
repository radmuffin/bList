use crate::models::GeoLocation;
use crate::security::{build_safe_http_client, validate_url_for_ssrf};
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
        let client = build_safe_http_client(Duration::from_secs(10));
        Self { client }
    }

    pub async fn geocode(&self, query: &str) -> Result<Option<GeoLocation>, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        // Limit query length to prevent abusive requests
        let safe_query = if trimmed.len() > 500 {
            &trimmed[..500]
        } else {
            trimmed
        };

        let encoded = urlencoding::encode(safe_query);
        let url_str = format!(
            "https://nominatim.openstreetmap.org/search?format=json&q={}&limit=1&addressdetails=1",
            encoded
        );

        let validated_url = validate_url_for_ssrf(&url_str)?;

        let response = self
            .client
            .get(validated_url.as_str())
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
        if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err("Invalid coordinates: latitude must be in [-90, 90] and longitude in [-180, 180]".to_string());
        }

        let url_str = format!(
            "https://nominatim.openstreetmap.org/reverse?format=json&lat={}&lon={}",
            lat, lon
        );

        let validated_url = validate_url_for_ssrf(&url_str)?;

        let response = self
            .client
            .get(validated_url.as_str())
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
    async fn test_geocoder_empty_query() {
        let geocoder = Geocoder::new();
        let res = geocoder.geocode("   ").await;
        assert_eq!(res, Ok(None));
    }

    #[tokio::test]
    async fn test_geocoder_invalid_coordinates() {
        let geocoder = Geocoder::new();
        assert!(geocoder.reverse_geocode(95.0, 10.0).await.is_err());
        assert!(geocoder.reverse_geocode(-95.0, 10.0).await.is_err());
        assert!(geocoder.reverse_geocode(10.0, 185.0).await.is_err());
        assert!(geocoder.reverse_geocode(f64::NAN, 0.0).await.is_err());
        assert!(geocoder.reverse_geocode(0.0, f64::INFINITY).await.is_err());
    }
}
