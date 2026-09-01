use crate::models::GeoLocation;
use crate::security::build_safe_http_client;
use serde::Deserialize;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// ---------------------------------------------------------------------------
// GeocoderProvider Trait
// ---------------------------------------------------------------------------

/// Pluggable interface for forward and reverse geocoding providers.
pub trait GeocoderProvider: Send + Sync {
    /// Returns the provider name (e.g. "nominatim", "mapbox", "google_places", "mock")
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Geocode a search query or address into GPS coordinates.
    fn geocode<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<GeoLocation>, String>>;

    /// Reverse-geocode GPS coordinates into a human-readable display address.
    fn reverse_geocode<'a>(
        &'a self,
        lat: f64,
        lon: f64,
    ) -> BoxFuture<'a, Result<Option<String>, String>>;
}

// ---------------------------------------------------------------------------
// OpenStreetMap Nominatim Provider (Default)
// ---------------------------------------------------------------------------

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

pub struct NominatimGeocoder {
    client: reqwest::Client,
    base_url: String,
}

impl NominatimGeocoder {
    pub fn new() -> Self {
        Self::with_base_url("https://nominatim.openstreetmap.org")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let client = build_safe_http_client(Duration::from_secs(10));

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }
}

impl Default for NominatimGeocoder {
    fn default() -> Self {
        Self::new()
    }
}

impl GeocoderProvider for NominatimGeocoder {
    fn name(&self) -> &'static str {
        "nominatim"
    }

    fn geocode<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<GeoLocation>, String>> {
        Box::pin(async move {
            let trimmed = query.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            let encoded = urlencoding::encode(trimmed);
            let url = format!(
                "{}/search?format=json&q={}&limit=1&addressdetails=1",
                self.base_url, encoded
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to contact geocoding service: {}", e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Geocoding service returned status: {}",
                    response.status()
                ));
            }

            let results: Vec<NominatimSearchResult> = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse geocoding response: {}", e))?;

            if let Some(first) = results.into_iter().next() {
                let lat: f64 = first
                    .lat
                    .parse()
                    .map_err(|_| "Invalid latitude in geocoding response")?;
                let lon: f64 = first
                    .lon
                    .parse()
                    .map_err(|_| "Invalid longitude in geocoding response")?;
                Ok(Some(GeoLocation {
                    latitude: lat,
                    longitude: lon,
                    display_name: first.display_name,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn reverse_geocode<'a>(
        &'a self,
        lat: f64,
        lon: f64,
    ) -> BoxFuture<'a, Result<Option<String>, String>> {
        Box::pin(async move {
            let url = format!(
                "{}/reverse?format=json&lat={}&lon={}",
                self.base_url, lat, lon
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
        })
    }
}

// ---------------------------------------------------------------------------
// Mapbox Geocoder Provider (Pluggable Backend)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MapboxFeature {
    place_name: String,
    center: Vec<f64>, // [longitude, latitude]
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MapboxResponse {
    features: Vec<MapboxFeature>,
}

#[allow(dead_code)]
pub struct MapboxGeocoder {
    client: reqwest::Client,
    access_token: String,
    base_url: String,
}

impl MapboxGeocoder {
    #[allow(dead_code)]
    pub fn new(access_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            access_token: access_token.into(),
            base_url: "https://api.mapbox.com/geocoding/v5/mapbox.places".to_string(),
        }
    }
}

impl GeocoderProvider for MapboxGeocoder {
    fn name(&self) -> &'static str {
        "mapbox"
    }

    fn geocode<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<GeoLocation>, String>> {
        Box::pin(async move {
            let trimmed = query.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            let encoded = urlencoding::encode(trimmed);
            let url = format!(
                "{}/{}.json?access_token={}&limit=1",
                self.base_url, encoded, self.access_token
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to contact Mapbox geocoding service: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Mapbox returned status: {}", response.status()));
            }

            let data: MapboxResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse Mapbox response: {}", e))?;

            if let Some(first) = data.features.into_iter().next() {
                if first.center.len() >= 2 {
                    return Ok(Some(GeoLocation {
                        longitude: first.center[0],
                        latitude: first.center[1],
                        display_name: first.place_name,
                    }));
                }
            }

            Ok(None)
        })
    }

    fn reverse_geocode<'a>(
        &'a self,
        lat: f64,
        lon: f64,
    ) -> BoxFuture<'a, Result<Option<String>, String>> {
        Box::pin(async move {
            let url = format!(
                "{}/{},{}.json?access_token={}&limit=1",
                self.base_url, lon, lat, self.access_token
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to contact Mapbox reverse geocoder: {}", e))?;

            if !response.status().is_success() {
                return Ok(None);
            }

            let data: Result<MapboxResponse, _> = response.json().await;
            match data {
                Ok(res) => {
                    if let Some(first) = res.features.into_iter().next() {
                        Ok(Some(first.place_name))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => Ok(None),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Google Places / Geocoding Provider (Pluggable Backend)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GoogleGeocodeResult {
    formatted_address: String,
    geometry: GoogleGeometry,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GoogleGeometry {
    location: GoogleLocation,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GoogleLocation {
    lat: f64,
    lng: f64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GoogleGeocodeResponse {
    results: Vec<GoogleGeocodeResult>,
    status: String,
}

#[allow(dead_code)]
pub struct GooglePlacesGeocoder {
    client: reqwest::Client,
    api_key: String,
}

impl GooglePlacesGeocoder {
    #[allow(dead_code)]
    pub fn new(api_key: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            api_key: api_key.into(),
        }
    }
}

impl GeocoderProvider for GooglePlacesGeocoder {
    fn name(&self) -> &'static str {
        "google_places"
    }

    fn geocode<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<GeoLocation>, String>> {
        Box::pin(async move {
            let trimmed = query.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            let encoded = urlencoding::encode(trimmed);
            let url = format!(
                "https://maps.googleapis.com/maps/api/geocode/json?address={}&key={}",
                encoded, self.api_key
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to contact Google Geocoding API: {}", e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Google Geocoding API returned status: {}",
                    response.status()
                ));
            }

            let data: GoogleGeocodeResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse Google Geocoding response: {}", e))?;

            if data.status != "OK" && data.status != "ZERO_RESULTS" {
                return Err(format!(
                    "Google Geocoding API status error: {}",
                    data.status
                ));
            }

            if let Some(first) = data.results.into_iter().next() {
                Ok(Some(GeoLocation {
                    latitude: first.geometry.location.lat,
                    longitude: first.geometry.location.lng,
                    display_name: first.formatted_address,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn reverse_geocode<'a>(
        &'a self,
        lat: f64,
        lon: f64,
    ) -> BoxFuture<'a, Result<Option<String>, String>> {
        Box::pin(async move {
            let url = format!(
                "https://maps.googleapis.com/maps/api/geocode/json?latlng={},{}&key={}",
                lat, lon, self.api_key
            );

            let response =
                self.client.get(&url).send().await.map_err(|e| {
                    format!("Failed to contact Google Reverse Geocoding API: {}", e)
                })?;

            if !response.status().is_success() {
                return Ok(None);
            }

            let data: Result<GoogleGeocodeResponse, _> = response.json().await;
            match data {
                Ok(res) => {
                    if let Some(first) = res.results.into_iter().next() {
                        Ok(Some(first.formatted_address))
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => Ok(None),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Mock Geocoder Provider (For Testing & Offline Environments)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Default)]
pub struct MockGeocoder {
    geocode_map: RwLock<HashMap<String, GeoLocation>>,
    reverse_map: RwLock<HashMap<String, String>>,
}

impl MockGeocoder {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn add_location(&self, query: impl Into<String>, location: GeoLocation) {
        let key = query.into().trim().to_lowercase();
        let reverse_key = format!("{:.4},{:.4}", location.latitude, location.longitude);
        self.reverse_map
            .write()
            .unwrap()
            .insert(reverse_key, location.display_name.clone());
        self.geocode_map.write().unwrap().insert(key, location);
    }
}

impl GeocoderProvider for MockGeocoder {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn geocode<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<Option<GeoLocation>, String>> {
        Box::pin(async move {
            let key = query.trim().to_lowercase();
            let map = self.geocode_map.read().unwrap();
            Ok(map.get(&key).cloned())
        })
    }

    fn reverse_geocode<'a>(
        &'a self,
        lat: f64,
        lon: f64,
    ) -> BoxFuture<'a, Result<Option<String>, String>> {
        Box::pin(async move {
            let key = format!("{:.4},{:.4}", lat, lon);
            let map = self.reverse_map.read().unwrap();
            Ok(map.get(&key).cloned())
        })
    }
}

// ---------------------------------------------------------------------------
// In-Memory Thread-Safe Geocoding Cache
// ---------------------------------------------------------------------------

#[derive(Default, Debug)]
pub struct GeocoderCache {
    forward_cache: RwLock<HashMap<String, Option<GeoLocation>>>,
    reverse_cache: RwLock<HashMap<String, Option<String>>>,
}

impl GeocoderCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn normalize_query(query: &str) -> String {
        query.trim().to_lowercase()
    }

    fn reverse_key(lat: f64, lon: f64) -> String {
        format!("{:.4},{:.4}", lat, lon)
    }

    pub fn get_forward(&self, query: &str) -> Option<Option<GeoLocation>> {
        let key = Self::normalize_query(query);
        let cache = self.forward_cache.read().ok()?;
        cache.get(&key).cloned()
    }

    pub fn insert_forward(&self, query: &str, result: Option<GeoLocation>) {
        let key = Self::normalize_query(query);
        if let Ok(mut cache) = self.forward_cache.write() {
            cache.insert(key, result);
        }
    }

    pub fn get_reverse(&self, lat: f64, lon: f64) -> Option<Option<String>> {
        let key = Self::reverse_key(lat, lon);
        let cache = self.reverse_cache.read().ok()?;
        cache.get(&key).cloned()
    }

    pub fn insert_reverse(&self, lat: f64, lon: f64, result: Option<String>) {
        let key = Self::reverse_key(lat, lon);
        if let Ok(mut cache) = self.reverse_cache.write() {
            cache.insert(key, result);
        }
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut c) = self.forward_cache.write() {
            c.clear();
        }
        if let Ok(mut c) = self.reverse_cache.write() {
            c.clear();
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> (usize, usize) {
        let f_len = self.forward_cache.read().map(|c| c.len()).unwrap_or(0);
        let r_len = self.reverse_cache.read().map(|c| c.len()).unwrap_or(0);
        (f_len, r_len)
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        let (f, r) = self.len();
        f == 0 && r == 0
    }
}

// ---------------------------------------------------------------------------
// Unified Geocoder Service
// ---------------------------------------------------------------------------

pub struct Geocoder {
    provider: Arc<dyn GeocoderProvider>,
    cache: Arc<GeocoderCache>,
}

impl Geocoder {
    /// Constructs a Geocoder with the default OpenStreetMap Nominatim provider and in-memory cache.
    pub fn new() -> Self {
        Self::with_provider(Arc::new(NominatimGeocoder::new()))
    }

    /// Constructs a Geocoder with a custom provider and a fresh in-memory cache.
    pub fn with_provider(provider: Arc<dyn GeocoderProvider>) -> Self {
        Self {
            provider,
            cache: Arc::new(GeocoderCache::new()),
        }
    }

    /// Constructs a Geocoder with a custom provider and an existing shared cache.
    #[allow(dead_code)]
    pub fn with_provider_and_cache(
        provider: Arc<dyn GeocoderProvider>,
        cache: Arc<GeocoderCache>,
    ) -> Self {
        Self { provider, cache }
    }

    /// Return the active provider's identifier.
    #[allow(dead_code)]
    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    /// Return the cache handle.
    #[allow(dead_code)]
    pub fn cache(&self) -> &Arc<GeocoderCache> {
        &self.cache
    }

    /// Geocode a search query into GPS coordinates, using in-memory cache when available.
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

        // 1. Check cache
        if let Some(cached) = self.cache.get_forward(safe_query) {
            return Ok(cached);
        }

        // 2. Check if the query is a valid Full Plus Code (Open Location Code)
        let first_word = safe_query.split_whitespace().next().unwrap_or(safe_query);
        if crate::plus_code::is_full(first_word) {
            if let Some((lat, lon)) = crate::plus_code::decode(first_word) {
                let display_name = match self.reverse_geocode(lat, lon).await {
                    Ok(Some(addr)) => format!("{} ({})", addr, first_word.to_uppercase()),
                    _ => format!("Plus Code: {}", first_word.to_uppercase()),
                };
                let geo = GeoLocation {
                    latitude: lat,
                    longitude: lon,
                    display_name,
                };
                self.cache.insert_forward(safe_query, Some(geo.clone()));
                return Ok(Some(geo));
            }
        }

        // 3. Delegate to underlying provider
        let result = self.provider.geocode(safe_query).await?;

        // 3. Populate cache
        self.cache.insert_forward(safe_query, result.clone());

        Ok(result)
    }

    /// Reverse geocode GPS coordinates into a display address, using in-memory cache when available.
    pub async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Option<String>, String> {
        if !lat.is_finite()
            || !lon.is_finite()
            || !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lon)
        {
            return Err(
                "Invalid coordinates: latitude must be in [-90, 90] and longitude in [-180, 180]"
                    .to_string(),
            );
        }

        // 1. Check cache
        if let Some(cached) = self.cache.get_reverse(lat, lon) {
            return Ok(cached);
        }

        // 2. Delegate to underlying provider
        let result = self.provider.reverse_geocode(lat, lon).await?;

        // 3. Populate cache
        self.cache.insert_reverse(lat, lon, result.clone());

        Ok(result)
    }
}

impl Default for Geocoder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_geocoder_and_cache() {
        let mock = Arc::new(MockGeocoder::new());
        mock.add_location(
            "Eiffel Tower",
            GeoLocation {
                latitude: 48.8584,
                longitude: 2.2945,
                display_name: "Eiffel Tower, Paris, France".to_string(),
            },
        );

        let custom_cache = Arc::new(GeocoderCache::new());
        let geocoder = Geocoder::with_provider_and_cache(mock, custom_cache);
        assert_eq!(geocoder.provider_name(), "mock");

        // Forward geocode lookup
        let res = geocoder.geocode("Eiffel Tower").await.unwrap();
        assert!(res.is_some());
        let loc = res.unwrap();
        assert!((loc.latitude - 48.8584).abs() < 1e-4);
        assert!((loc.longitude - 2.2945).abs() < 1e-4);

        // Verify cache was populated
        let (f_len, _) = geocoder.cache().len();
        assert_eq!(f_len, 1);

        // Case-insensitive lookup should hit cache
        let cached_res = geocoder.geocode("  eiffel tower  ").await.unwrap();
        assert!(cached_res.is_some());
        assert_eq!(
            cached_res.unwrap().display_name,
            "Eiffel Tower, Paris, France"
        );

        // Reverse geocode lookup
        let rev = geocoder.reverse_geocode(48.8584, 2.2945).await.unwrap();
        assert_eq!(rev, Some("Eiffel Tower, Paris, France".to_string()));

        let (_, r_len) = geocoder.cache().len();
        assert_eq!(r_len, 1);

        // Unknown location returns None
        let not_found = geocoder.geocode("Atlantis Submerged City").await.unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_geocoder_cache_normalization_and_quantization() {
        let cache = GeocoderCache::new();
        assert!(cache.is_empty());

        cache.insert_forward(
            "Tokyo Tower",
            Some(GeoLocation {
                latitude: 35.6586,
                longitude: 139.7454,
                display_name: "Tokyo Tower, Japan".to_string(),
            }),
        );

        let hit = cache.get_forward("  TOKYO TOWER  ").flatten();
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().display_name, "Tokyo Tower, Japan");

        cache.insert_reverse(35.65858, 139.74542, Some("Tokyo Tower".to_string()));
        let rev_hit = cache.get_reverse(35.6586, 139.7454).flatten();
        assert_eq!(rev_hit, Some("Tokyo Tower".to_string()));

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_provider_initialization() {
        let mapbox = MapboxGeocoder::new("test_token");
        assert_eq!(mapbox.name(), "mapbox");

        let google = GooglePlacesGeocoder::new("test_key");
        assert_eq!(google.name(), "google_places");

        let nominatim = NominatimGeocoder::new();
        assert_eq!(nominatim.name(), "nominatim");
    }

    #[tokio::test]
    async fn test_geocode_empty_and_whitespace_query() {
        let geocoder = Geocoder::new();
        assert_eq!(geocoder.geocode("").await.expect("empty query"), None);
        assert_eq!(geocoder.geocode("   ").await.expect("spaces"), None);
        assert_eq!(
            geocoder.geocode("\t\n  \n").await.expect("tabs/newlines"),
            None
        );
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

        let results: Vec<NominatimSearchResult> =
            serde_json::from_str(json_data).expect("deserialize search result");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].lat, "35.6585805");
        assert_eq!(results[0].lon, "139.7454329");
        assert_eq!(
            results[0].display_name,
            "Tokyo Tower, 4-2-8, Shibakoen, Minato, Tokyo, Japan"
        );

        let lat: f64 = results[0].lat.parse().expect("parse lat");
        let lon: f64 = results[0].lon.parse().expect("parse lon");
        assert!((lat - 35.6585805).abs() < 1e-5);
        assert!((lon - 139.7454329).abs() < 1e-5);
    }

    #[test]
    fn test_nominatim_search_result_empty_array() {
        let json_data = "[]";
        let results: Vec<NominatimSearchResult> =
            serde_json::from_str(json_data).expect("deserialize empty array");
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

        let results: Vec<NominatimSearchResult> =
            serde_json::from_str(json_data).expect("deserialize search result");
        assert_eq!(results.len(), 1);
        assert!(results[0].lat.parse::<f64>().is_err());
    }

    #[test]
    fn test_nominatim_reverse_result_deserialization() {
        let json_data = r#"{
            "display_name": "Eiffel Tower, 5, Avenue Anatole France, Quartier du Gros-Caillou, Paris, France"
        }"#;

        let result: NominatimReverseResult =
            serde_json::from_str(json_data).expect("deserialize reverse result");
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

    #[tokio::test]
    async fn test_geocoder_invalid_coordinates() {
        let geocoder = Geocoder::new();
        assert!(geocoder.reverse_geocode(95.0, 10.0).await.is_err());
        assert!(geocoder.reverse_geocode(-95.0, 10.0).await.is_err());
        assert!(geocoder.reverse_geocode(10.0, 185.0).await.is_err());
        assert!(geocoder.reverse_geocode(f64::NAN, 0.0).await.is_err());
        assert!(geocoder.reverse_geocode(0.0, f64::INFINITY).await.is_err());
    }

    #[test]
    fn test_geocoder_negative_caching() {
        let cache = GeocoderCache::new();
        // Insert negative cache entry (None)
        cache.insert_forward("Nonexistent Place Atlantis", None);
        assert_eq!(cache.get_forward("Nonexistent Place Atlantis"), Some(None));
        assert_eq!(
            cache.get_forward("  nonexistent place atlantis  "),
            Some(None)
        );

        cache.insert_reverse(0.0, 0.0, None);
        assert_eq!(cache.get_reverse(0.0, 0.0), Some(None));
    }

    #[tokio::test]
    async fn test_mock_geocoder_multiple_locations_and_reverse() {
        let mock = Arc::new(MockGeocoder::new());
        mock.add_location(
            "Tokyo Skytree",
            GeoLocation {
                latitude: 35.7100,
                longitude: 139.8107,
                display_name: "Tokyo Skytree, Sumida, Tokyo, Japan".to_string(),
            },
        );
        mock.add_location(
            "Colosseum",
            GeoLocation {
                latitude: 41.8902,
                longitude: 12.4922,
                display_name: "Colosseum, Piazza del Colosseo, Rome, Italy".to_string(),
            },
        );

        let geocoder = Geocoder::with_provider(mock);

        // Forward geocode Tokyo
        let tokyo = geocoder
            .geocode("Tokyo Skytree")
            .await
            .unwrap()
            .expect("found tokyo");
        assert!((tokyo.latitude - 35.7100).abs() < 1e-4);
        assert!((tokyo.longitude - 139.8107).abs() < 1e-4);

        // Forward geocode Rome
        let rome = geocoder
            .geocode("  COLOSSEUM  ")
            .await
            .unwrap()
            .expect("found rome");
        assert!((rome.latitude - 41.8902).abs() < 1e-4);

        // Reverse geocode
        let rev_tokyo = geocoder.reverse_geocode(35.7100, 139.8107).await.unwrap();
        assert_eq!(
            rev_tokyo,
            Some("Tokyo Skytree, Sumida, Tokyo, Japan".to_string())
        );

        let rev_rome = geocoder.reverse_geocode(41.8902, 12.4922).await.unwrap();
        assert_eq!(
            rev_rome,
            Some("Colosseum, Piazza del Colosseo, Rome, Italy".to_string())
        );
    }
}
