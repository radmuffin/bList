use crate::geocoder::Geocoder;
use crate::models::ScrapedMetadata;
use crate::plus_code;
use crate::security::{build_safe_http_client, validate_url_for_ssrf};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// ---------------------------------------------------------------------------
// Scraper Context & Helpers
// ---------------------------------------------------------------------------

pub struct ScraperContext {
    pub client: reqwest::Client,
    pub geocoder: Arc<Geocoder>,
}

impl ScraperContext {
    pub fn new(geocoder: Arc<Geocoder>) -> Self {
        let client = build_safe_http_client(Duration::from_secs(15));
        Self { client, geocoder }
    }

    /// Fetch HTML content and handle potential meta refresh redirects
    pub async fn fetch_html(&self, url: &str) -> Result<(String, String), String> {
        let safe_url = validate_url_for_ssrf(url)?;
        let response = self
            .client
            .get(safe_url.as_str())
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        let mut final_url = response.url().to_string();
        let mut html_text = response.text().await.unwrap_or_default();

        // Check for client-side meta refresh or JS redirect
        if html_text.contains("http-equiv=\"refresh\"")
            || html_text.contains("http-equiv='refresh'")
        {
            let re_refresh = Regex::new(r#"(?i)content=["'][0-9]+;\s*url=([^"']+)["']"#).unwrap();
            if let Some(caps) = re_refresh.captures(&html_text) {
                if let Some(m) = caps.get(1) {
                    let next_url = m.as_str().replace("&amp;", "&");
                    if let Ok(safe_next) = validate_url_for_ssrf(&next_url) {
                        if let Ok(next_res) = self.client.get(safe_next.as_str()).send().await {
                            final_url = next_res.url().to_string();
                            html_text = next_res.text().await.unwrap_or_default();
                        }
                    }
                }
            }
        }

        Ok((final_url, html_text))
    }
}

// ---------------------------------------------------------------------------
// LinkScraper Trait
// ---------------------------------------------------------------------------

/// Trait implemented by modular domain-specific scrapers.
pub trait LinkScraper: Send + Sync {
    /// Identifier for the scraper (e.g. "google_maps", "apple_maps", "instagram", "generic")
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Checks if this scraper can handle the given URL.
    fn can_handle(&self, url: &str) -> bool;

    /// Extracts place/location metadata from the given URL.
    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>>;
}

// ---------------------------------------------------------------------------
// Google Maps Scraper
// ---------------------------------------------------------------------------

pub struct GoogleMapsScraper;

impl LinkScraper for GoogleMapsScraper {
    fn name(&self) -> &'static str {
        "google_maps"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("maps.google.")
            || url.contains("goo.gl/maps")
            || url.contains("maps.app.goo.gl")
            || url.contains("google.com/maps")
            || url.contains("/maps/place")
            || url.contains("/maps/search")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let (final_url, html_text) = ctx.fetch_html(url).await?;

            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut title: Option<String> = None;
            let mut address: Option<String> = None;
            let mut image_url: Option<String> = None;
            let mut place_query_candidate: Option<String> = None;

            // 1. Extract place name / search query from URL paths and query parameters
            let re_place = Regex::new(r"/maps/place/([^/@?]+)").unwrap();
            if let Some(caps) = re_place
                .captures(&final_url)
                .or_else(|| re_place.captures(url))
            {
                if let Some(m) = caps.get(1) {
                    let unencoded = urlencoding::decode(m.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let clean_name = unencoded.replace('+', " ").trim().to_string();
                    if !clean_name.is_empty() && !clean_name.starts_with("data=") {
                        // Check if place path contains a Plus Code (e.g. "87G8P326+W2")
                        if let Some((code, label)) = parse_plus_code_token(&clean_name) {
                            if let Some((la, lo)) = plus_code::decode(&code) {
                                lat = Some(la);
                                lon = Some(lo);
                                if let Some(l) = label {
                                    title = Some(l.clone());
                                    place_query_candidate = Some(l);
                                }
                            }
                        }

                        let (parsed_t, parsed_a) = parse_google_maps_title_and_address(&clean_name);
                        if let Some(t) = parsed_t {
                            if title.is_none() {
                                title = Some(t);
                            }
                        } else if title.is_none() {
                            title = Some(clean_name.clone());
                        }

                        if let Some(a) = parsed_a {
                            address = Some(a.clone());
                            if place_query_candidate.is_none() {
                                place_query_candidate = Some(a);
                            }
                        } else if place_query_candidate.is_none() {
                            let stripped_q = clean_name
                                .replace('|', " ")
                                .replace("%7C", " ")
                                .trim()
                                .to_string();
                            place_query_candidate = Some(stripped_q);
                        }
                    }
                }
            }

            if place_query_candidate.is_none() {
                let re_search = Regex::new(r"/maps/search/([^/@?]+)").unwrap();
                if let Some(caps) = re_search
                    .captures(&final_url)
                    .or_else(|| re_search.captures(url))
                {
                    if let Some(m) = caps.get(1) {
                        let unencoded = urlencoding::decode(m.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let clean_name = unencoded.replace('+', " ").trim().to_string();
                        if !clean_name.is_empty() && !clean_name.starts_with("data=") {
                            if let Some((code, label)) = parse_plus_code_token(&clean_name) {
                                if let Some((la, lo)) = plus_code::decode(&code) {
                                    lat = Some(la);
                                    lon = Some(lo);
                                    if let Some(l) = label {
                                        title = Some(l);
                                    }
                                }
                            }
                            let stripped_q = clean_name
                                .replace('|', " ")
                                .replace("%7C", " ")
                                .trim()
                                .to_string();
                            place_query_candidate = Some(stripped_q.clone());
                            if title.is_none() {
                                title = Some(stripped_q);
                            }
                        }
                    }
                }
            }

            if place_query_candidate.is_none() {
                let re_q = Regex::new(r"[?&](?:q|query|destination|daddr)=([^&]+)").unwrap();
                if let Some(caps) = re_q.captures(&final_url).or_else(|| re_q.captures(url)) {
                    if let Some(m) = caps.get(1) {
                        let unencoded = urlencoding::decode(m.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let clean_name = unencoded.replace('+', " ").trim().to_string();
                        if !clean_name.is_empty() {
                            if let Some((code, label)) = parse_plus_code_token(&clean_name) {
                                if let Some((la, lo)) = plus_code::decode(&code) {
                                    lat = Some(la);
                                    lon = Some(lo);
                                    if let Some(l) = label {
                                        title = Some(l);
                                    }
                                }
                            }
                            if clean_name.chars().any(|c| c.is_alphabetic()) {
                                let stripped_q = clean_name
                                    .replace('|', " ")
                                    .replace("%7C", " ")
                                    .trim()
                                    .to_string();
                                place_query_candidate = Some(stripped_q.clone());
                                if title.is_none() {
                                    title = Some(stripped_q);
                                }
                            }
                        }
                    }
                }
            }

            // 2. Parse HTML Metadata scoped in an isolated block so Html (which is !Send) is dropped before await
            if !html_text.is_empty() {
                let (
                    raw_meta_title,
                    og_desc,
                    og_img,
                    twitter_img,
                    itemprop_img,
                    img_src_link,
                    schema_lat_lon,
                ) = {
                    let document = Html::parse_document(&html_text);

                    let raw_t = extract_meta_content(&document, "meta[property='og:title']")
                        .or_else(|| extract_tag_text(&document, "title"));
                    let desc = extract_meta_content(&document, "meta[property='og:description']")
                        .or_else(|| extract_meta_content(&document, "meta[name='description']"));
                    let img = extract_meta_content(&document, "meta[property='og:image']");
                    let tw_img = extract_meta_content(&document, "meta[name='twitter:image']");
                    let item_img = extract_meta_content(&document, "meta[itemprop='image']");
                    let link_img = extract_meta_content(&document, "link[rel='image_src']");
                    let la = extract_meta_content(&document, "meta[itemprop='latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                    let lo = extract_meta_content(&document, "meta[itemprop='longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                    (raw_t, desc, img, tw_img, item_img, link_img, (la, lo))
                };

                // Filter out Google consent page / captcha titles
                if let Some(raw_t) = raw_meta_title {
                    if !raw_t.contains("Before you continue to Google Maps")
                        && !raw_t.eq_ignore_ascii_case("Google Maps")
                    {
                        let (cleaned_t, extracted_addr) =
                            parse_google_maps_title_and_address(&raw_t);
                        if let Some(t) = cleaned_t {
                            if title.is_none() || title.as_deref() == Some("Google Maps") {
                                title = Some(t.clone());
                            }
                            if place_query_candidate.is_none() {
                                if let Some(ref a) = extracted_addr {
                                    place_query_candidate = Some(format!("{}, {}", t, a));
                                } else {
                                    place_query_candidate = Some(t);
                                }
                            }
                        }
                        if address.is_none() {
                            address = extracted_addr;
                        }
                    }
                }

                // Filter out Google Maps generic boilerplate descriptions
                if address.is_none() {
                    if let Some(ref d) = og_desc {
                        if !d.eq_ignore_ascii_case("Google Maps")
                            && !d.contains(
                                "Find local businesses, view maps and get driving directions",
                            )
                            && !d.contains("Before you continue to Google Maps")
                            && !d.is_empty()
                        {
                            address = Some(d.clone());
                            if place_query_candidate.is_none() {
                                place_query_candidate = Some(d.clone());
                            }
                        }
                    }
                }

                if image_url.is_none() {
                    image_url = extract_google_maps_photo(
                        og_img.as_deref(),
                        twitter_img.as_deref(),
                        itemprop_img.as_deref(),
                        img_src_link.as_deref(),
                        &html_text,
                    );
                }

                if let (Some(la), Some(lo)) = schema_lat_lon {
                    lat = Some(la);
                    lon = Some(lo);
                }
            }

            // 3. Extract exact pin coordinates from URL (!3d... !4d...)
            let combined_url_search = format!("{} {}", final_url, url);
            let re_3d = Regex::new(r"!3d(-?\d+\.\d+)").unwrap();
            let re_4d = Regex::new(r"!4d(-?\d+\.\d+)").unwrap();
            if lat.is_none() || lon.is_none() {
                if let (Some(c_lat), Some(c_lon)) = (
                    re_3d.captures(&combined_url_search),
                    re_4d.captures(&combined_url_search),
                ) {
                    lat = c_lat.get(1).and_then(|m| m.as_str().parse().ok());
                    lon = c_lon.get(1).and_then(|m| m.as_str().parse().ok());
                }
            }

            // 4. Extract query coordinates if present in URL (q=lat,lon)
            if lat.is_none() || lon.is_none() {
                let re_q_coords =
                    Regex::new(r"[?&](?:q|ll|query)=(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
                if let Some(caps) = re_q_coords.captures(&combined_url_search) {
                    lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                    lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
                }
            }

            // 5. Geocode place candidate via Geocoder if lat/lon still None
            if lat.is_none() || lon.is_none() {
                if let Some(ref query) = place_query_candidate {
                    let clean_q = query
                        .replace('|', " ")
                        .replace("%7C", " ")
                        .trim()
                        .to_string();
                    if !clean_q.is_empty() && clean_q != "Google Maps" {
                        if let Ok(Some(geo)) = ctx.geocoder.geocode(&clean_q).await {
                            lat = Some(geo.latitude);
                            lon = Some(geo.longitude);
                            if address.is_none() {
                                address = Some(geo.display_name.clone());
                            }
                        }
                    }
                }
            }

            if lat.is_none() || lon.is_none() {
                if let Some(ref addr) = address {
                    let clean_addr = addr
                        .replace('|', " ")
                        .replace("%7C", " ")
                        .trim()
                        .to_string();
                    if !clean_addr.is_empty() && clean_addr != "Google Maps" {
                        if let Ok(Some(geo)) = ctx.geocoder.geocode(&clean_addr).await {
                            lat = Some(geo.latitude);
                            lon = Some(geo.longitude);
                        }
                    }
                }
            }

            // 6. Viewport coords fallback (@lat,lon)
            if lat.is_none() || lon.is_none() {
                let re_at = Regex::new(r"@(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
                if let Some(caps) = re_at.captures(&final_url).or_else(|| re_at.captures(url)) {
                    lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                    lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
                }
            }

            // 7. Reverse geocode if coordinates exist but address is missing
            if let (Some(la), Some(lo)) = (lat, lon) {
                if address.is_none() || address.as_deref() == Some("Google Maps") {
                    if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                        if title.is_none() || title.as_deref() == Some("Google Maps") {
                            let short_title = addr
                                .split(',')
                                .next()
                                .unwrap_or("Saved Place")
                                .trim()
                                .to_string();
                            title = Some(short_title);
                        }
                        address = Some(addr);
                    }
                }
            } else {
                let fallback_search = title.as_deref().or(address.as_deref());
                if let Some(term) = fallback_search {
                    if term != "Google Maps" && !term.is_empty() {
                        if let Ok(Some(geo)) = ctx.geocoder.geocode(term).await {
                            lat = Some(geo.latitude);
                            lon = Some(geo.longitude);
                            if address.is_none() {
                                address = Some(geo.display_name);
                            }
                        }
                    }
                }
            }

            let mut resolved_title = title.unwrap_or_else(|| "Saved Place".to_string());
            if resolved_title == "Google Maps" || resolved_title.is_empty() {
                if let Some(ref a) = address {
                    resolved_title = a
                        .split(',')
                        .next()
                        .unwrap_or("Saved Place")
                        .trim()
                        .to_string();
                } else {
                    resolved_title = "Saved Place".to_string();
                }
            }

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: None,
                latitude: lat,
                longitude: lon,
                address,
                image_url,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "google_maps".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Apple Maps Scraper
// ---------------------------------------------------------------------------

pub struct AppleMapsScraper;

impl LinkScraper for AppleMapsScraper {
    fn name(&self) -> &'static str {
        "apple_maps"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("maps.apple.com")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut title: Option<String> = None;
            let mut address: Option<String> = None;

            let re_ll = Regex::new(r"[?&](?:ll|coordinate)=(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
            if let Some(caps) = re_ll.captures(url) {
                lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
            }

            let re_q = Regex::new(r"[?&]q=([^&]+)").unwrap();
            if let Some(caps) = re_q.captures(url) {
                if let Some(m) = caps.get(1) {
                    let unencoded = urlencoding::decode(m.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let clean = unencoded.replace('+', " ").trim().to_string();
                    if !clean.is_empty() {
                        title = Some(clean.clone());
                        address = Some(clean);
                    }
                }
            }

            let re_address = Regex::new(r"[?&]address=([^&]+)").unwrap();
            if let Some(caps) = re_address.captures(url) {
                if let Some(m) = caps.get(1) {
                    let unencoded = urlencoding::decode(m.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let clean = unencoded.replace('+', " ").trim().to_string();
                    if !clean.is_empty() {
                        address = Some(clean);
                    }
                }
            }

            if lat.is_none() || lon.is_none() {
                let search_term = address.as_ref().or(title.as_ref());
                if let Some(query) = search_term {
                    if let Ok(Some(geo)) = ctx.geocoder.geocode(query).await {
                        lat = Some(geo.latitude);
                        lon = Some(geo.longitude);
                        if title.is_none() {
                            title = Some(
                                geo.display_name
                                    .split(',')
                                    .next()
                                    .unwrap_or("Apple Maps Place")
                                    .trim()
                                    .to_string(),
                            );
                        }
                        address = Some(geo.display_name);
                    }
                }
            } else if address.is_none() {
                if let (Some(la), Some(lo)) = (lat, lon) {
                    if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                        address = Some(addr);
                    }
                }
            }

            let resolved_title = title.unwrap_or_else(|| "Apple Maps Place".to_string());

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: None, // Do NOT clone address into description
                latitude: lat,
                longitude: lon,
                address,
                image_url: None,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "apple_maps".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// OpenStreetMap Scraper
// ---------------------------------------------------------------------------

pub struct OpenStreetMapScraper;

impl LinkScraper for OpenStreetMapScraper {
    fn name(&self) -> &'static str {
        "openstreetmap"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("openstreetmap.org") || url.contains("osm.org")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut title: Option<String> = None;
            let mut address: Option<String> = None;

            // 1. Check hash coordinates: #map=16/51.5074/-0.1278
            let re_map = Regex::new(r"#map=\d+/(-?\d+\.\d+)/(-?\d+\.\d+)").unwrap();
            if let Some(caps) = re_map.captures(url) {
                lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
            }

            // 2. Check query coordinates: ?mlat=...&mlon=... or ?lat=...&lon=...
            if lat.is_none() || lon.is_none() {
                let re_mlat =
                    Regex::new(r"[?&](?:m?lat)=(-?\d+\.\d+)[&](?:m?lon)=(-?\d+\.\d+)").unwrap();
                if let Some(caps) = re_mlat.captures(url) {
                    lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                    lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
                }
            }

            // 3. Check query search: ?query=...
            let re_q = Regex::new(r"[?&]query=([^&#]+)").unwrap();
            if let Some(caps) = re_q.captures(url) {
                if let Some(m) = caps.get(1) {
                    let unencoded = urlencoding::decode(m.as_str())
                        .unwrap_or_default()
                        .replace('+', " ");
                    let clean = unencoded.trim().to_string();
                    if !clean.is_empty() {
                        title = Some(clean.clone());
                        if lat.is_none() || lon.is_none() {
                            if let Ok(Some(geo)) = ctx.geocoder.geocode(&clean).await {
                                lat = Some(geo.latitude);
                                lon = Some(geo.longitude);
                                address = Some(geo.display_name);
                            }
                        }
                    }
                }
            }

            // 4. Reverse geocode if coordinates found
            if let (Some(la), Some(lo)) = (lat, lon) {
                if address.is_none() {
                    if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                        if title.is_none() {
                            title = Some(
                                addr.split(',')
                                    .next()
                                    .unwrap_or("OpenStreetMap Location")
                                    .trim()
                                    .to_string(),
                            );
                        }
                        address = Some(addr);
                    }
                }
            }

            let resolved_title = title.unwrap_or_else(|| "OpenStreetMap Location".to_string());

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: None,
                latitude: lat,
                longitude: lon,
                address,
                image_url: None,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "openstreetmap".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Instagram Scraper
// ---------------------------------------------------------------------------

pub struct InstagramScraper;

impl LinkScraper for InstagramScraper {
    fn name(&self) -> &'static str {
        "instagram"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("instagram.com") || url.contains("instagr.am")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut address: Option<String> = None;
            let mut place_candidate: Option<String> = None;

            // Check if URL is an Instagram location page (e.g. /explore/locations/213019808/golden-gate-bridge/)
            let re_loc = Regex::new(r#"/explore/locations/\d+/([^/?#]+)"#).unwrap();
            if let Some(caps) = re_loc.captures(url) {
                if let Some(m) = caps.get(1) {
                    let slug = m.as_str().replace(['-', '_', '+'], " ").trim().to_string();
                    if !slug.is_empty() {
                        let name = to_title_case(&slug);
                        place_candidate = Some(name);
                    }
                }
            }

            let (_, html_text) = ctx.fetch_html(url).await.unwrap_or_default();

            let (og_title, og_desc, og_image) = if !html_text.is_empty() {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']");
                let d = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                (t, d, img)
            } else {
                (None, None, None)
            };

            let mut resolved_title = place_candidate.unwrap_or_else(|| {
                og_title
                    .map(|t| clean_page_title(&t))
                    .unwrap_or_else(|| "Instagram Post".to_string())
            });

            let mut resolved_desc = og_desc;

            // If title contains social caption junk ("User on Instagram: ..."), clean it
            if resolved_title.contains(" on Instagram") || resolved_title.starts_with('"') {
                let (c_title, c_desc) = clean_social_caption(&resolved_title);
                resolved_title = c_title;
                if resolved_desc.is_none() {
                    resolved_desc = c_desc;
                }
            }

            if resolved_title != "Instagram Post" && !resolved_title.is_empty() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&resolved_title).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            }

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: resolved_desc,
                latitude: lat,
                longitude: lon,
                address,
                image_url: og_image,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "instagram".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// TikTok Scraper (Modular Domain Scraper)
// ---------------------------------------------------------------------------

pub struct TikTokScraper;

impl LinkScraper for TikTokScraper {
    fn name(&self) -> &'static str {
        "tiktok"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("tiktok.com") || url.contains("vt.tiktok.com")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let (_, html_text) = ctx.fetch_html(url).await.unwrap_or_default();

            let (og_title, og_desc, og_image) = if !html_text.is_empty() {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t))
                    .unwrap_or_else(|| "TikTok Video".to_string());
                let desc = extract_meta_content(&document, "meta[property='og:description']")
                    .or_else(|| extract_meta_content(&document, "meta[name='description']"));
                let img = extract_meta_content(&document, "meta[property='og:image']");
                (t, desc, img)
            } else {
                ("TikTok Video".to_string(), None, None)
            };

            let mut resolved_title = og_title;
            let mut resolved_desc = og_desc;

            if resolved_title.contains(" on TikTok") {
                let (c_title, c_desc) = clean_social_caption(&resolved_title);
                resolved_title = c_title;
                if resolved_desc.is_none() {
                    resolved_desc = c_desc;
                }
            }

            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut address: Option<String> = None;

            if resolved_title != "TikTok Video" && !resolved_title.is_empty() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&resolved_title).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            }

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: resolved_desc,
                latitude: lat,
                longitude: lon,
                address,
                image_url: og_image,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "tiktok".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// TripAdvisor Scraper (Modular Domain Scraper)
// ---------------------------------------------------------------------------

pub struct TripAdvisorScraper;

impl LinkScraper for TripAdvisorScraper {
    fn name(&self) -> &'static str {
        "tripadvisor"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("tripadvisor.com")
            || url.contains("tripadvisor.co.")
            || url.contains("tripadvisor.")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let (_, html_text) = ctx.fetch_html(url).await.unwrap_or_default();

            let (og_title, og_desc, og_image, mut lat, mut lon, jsonld_place) = if !html_text
                .is_empty()
            {
                let jsonld = extract_jsonld_place_metadata(&html_text);
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t));
                let desc = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                let la =
                    extract_meta_content(&document, "meta[property='place:location:latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                let lo =
                    extract_meta_content(&document, "meta[property='place:location:longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                (t, desc, img, la, lo, jsonld)
            } else {
                (None, None, None, None, None, None)
            };

            let mut title = None;
            let mut address = None;

            if let Some(ref jp) = jsonld_place {
                if title.is_none() {
                    title = jp.name.clone();
                }
                if address.is_none() {
                    address = jp.address.clone();
                }
                if lat.is_none() {
                    lat = jp.latitude;
                }
                if lon.is_none() {
                    lon = jp.longitude;
                }
            }

            if let Some(raw_t) = og_title {
                let (parsed_t, parsed_a) = parse_tripadvisor_title_and_address(&raw_t);
                if title.is_none() {
                    title = parsed_t;
                }
                if address.is_none() {
                    address = parsed_a;
                }
            }

            // Fallback to URL slug if blocked (e.g. 403 bot check)
            if title.is_none() {
                let re_slug = Regex::new(
                    r#"/(?:Restaurant_Review|Attraction_Review|Hotel_Review)-[a-zA-Z0-9_-]+-Reviews-([a-zA-Z0-9_]+)\.html"#,
                )
                .unwrap();
                if let Some(caps) = re_slug.captures(url) {
                    if let Some(m) = caps.get(1) {
                        let slug = m.as_str().replace('_', " ").trim().to_string();
                        title = Some(to_title_case(&slug));
                    }
                }
            }

            let resolved_title = title.unwrap_or_else(|| "TripAdvisor Location".to_string());

            if lat.is_none() || lon.is_none() {
                let search_target = if let Some(ref a) = address {
                    format!("{}, {}", resolved_title, a)
                } else {
                    resolved_title.clone()
                };
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&search_target).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    if address.is_none() {
                        address = Some(geo.display_name);
                    }
                }
            } else if let (Some(la), Some(lo)) = (lat, lon) {
                if address.is_none() {
                    if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                        address = Some(addr);
                    }
                }
            }

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: og_desc,
                latitude: lat,
                longitude: lon,
                address,
                image_url: og_image,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "tripadvisor".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Yelp Scraper (Modular Domain Scraper)
// ---------------------------------------------------------------------------

pub struct YelpScraper;

impl LinkScraper for YelpScraper {
    fn name(&self) -> &'static str {
        "yelp"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("yelp.com") || url.contains("yelp.ca") || url.contains("yelp.co.")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let (_, html_text) = ctx.fetch_html(url).await.unwrap_or_default();

            let (og_title, og_desc, og_image, mut lat, mut lon, jsonld_place) = if !html_text
                .is_empty()
            {
                let jsonld = extract_jsonld_place_metadata(&html_text);
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t));
                let desc = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                let la =
                    extract_meta_content(&document, "meta[property='place:location:latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                let lo =
                    extract_meta_content(&document, "meta[property='place:location:longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                (t, desc, img, la, lo, jsonld)
            } else {
                (None, None, None, None, None, None)
            };

            let mut title = None;
            let mut address = None;

            if let Some(ref jp) = jsonld_place {
                if title.is_none() {
                    title = jp.name.clone();
                }
                if address.is_none() {
                    address = jp.address.clone();
                }
                if lat.is_none() {
                    lat = jp.latitude;
                }
                if lon.is_none() {
                    lon = jp.longitude;
                }
            }

            if let Some(raw_t) = og_title {
                let (parsed_t, parsed_a) = parse_yelp_title_and_address(&raw_t);
                if title.is_none() {
                    title = parsed_t;
                }
                if address.is_none() {
                    address = parsed_a;
                }
            }

            // Fallback to URL slug if blocked (e.g. 403 bot check on Yelp)
            if title.is_none() {
                let re_slug = Regex::new(r#"/biz/([^/?#]+)"#).unwrap();
                if let Some(caps) = re_slug.captures(url) {
                    if let Some(m) = caps.get(1) {
                        let slug = m.as_str().replace(['-', '_', '+'], " ").trim().to_string();
                        title = Some(to_title_case(&slug));
                    }
                }
            }

            let resolved_title = title.unwrap_or_else(|| "Yelp Place".to_string());

            if lat.is_none() || lon.is_none() {
                let search_target = if let Some(ref a) = address {
                    format!("{}, {}", resolved_title, a)
                } else {
                    resolved_title.clone()
                };
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&search_target).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    if address.is_none() {
                        address = Some(geo.display_name);
                    }
                }
            } else if let (Some(la), Some(lo)) = (lat, lon) {
                if address.is_none() {
                    if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                        address = Some(addr);
                    }
                }
            }

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: og_desc,
                latitude: lat,
                longitude: lon,
                address,
                image_url: og_image,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "yelp".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// AllTrails Scraper (Modular Domain Scraper)
// ---------------------------------------------------------------------------

pub struct AllTrailsScraper;

impl LinkScraper for AllTrailsScraper {
    fn name(&self) -> &'static str {
        "alltrails"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("alltrails.com")
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let (_, html_text) = ctx.fetch_html(url).await.unwrap_or_default();

            let (og_title, og_desc, og_image, mut lat, mut lon) = if !html_text.is_empty() {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t));
                let desc = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                let la =
                    extract_meta_content(&document, "meta[property='place:location:latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                let lo =
                    extract_meta_content(&document, "meta[property='place:location:longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                (t, desc, img, la, lo)
            } else {
                (None, None, None, None, None)
            };

            let mut title = None;
            let mut region = None;

            if let Some(raw_t) = og_title {
                let (parsed_t, parsed_r) = parse_alltrails_title_and_region(&raw_t);
                title = parsed_t;
                region = parsed_r;
            }

            let resolved_title = title.unwrap_or_else(|| "AllTrails Route".to_string());
            let mut address = region;

            if lat.is_none() || lon.is_none() {
                let search_target = if let Some(ref a) = address {
                    format!("{}, {}", resolved_title, a)
                } else {
                    resolved_title.clone()
                };
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&search_target).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            } else if let (Some(la), Some(lo)) = (lat, lon) {
                if address.is_none() {
                    if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                        address = Some(addr);
                    }
                }
            }

            Ok(ScrapedMetadata {
                title: resolved_title,
                description: og_desc,
                latitude: lat,
                longitude: lon,
                address,
                image_url: og_image,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "alltrails".to_string(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// bList Scraper (for bList-to-bList shares and deep place links)
// ---------------------------------------------------------------------------

pub struct BListScraper;

impl LinkScraper for BListScraper {
    fn name(&self) -> &'static str {
        "blist"
    }

    fn can_handle(&self, url: &str) -> bool {
        let lower = url.to_lowercase();
        // NOTE: localhost/127.0.0.1 are matched here intentionally to handle bList-to-bList
        // deep-link sharing (e.g. from the PWA share target). BListScraper::scrape() only
        // parses query parameters from the URL — it makes NO outbound HTTP requests — so
        // accepting localhost URLs here does not introduce SSRF risk.
        lower.contains("blist")
            || lower.contains("localhost")
            || lower.contains("127.0.0.1")
            || ((lower.contains("lat=") || lower.contains("latitude="))
                && (lower.contains("lng=")
                    || lower.contains("lon=")
                    || lower.contains("longitude=")))
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let parsed_url = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut title: Option<String> = None;
            let mut address: Option<String> = None;
            let mut source_url: Option<String> = None;
            let mut category: Option<String> = None;

            for (k, v) in parsed_url.query_pairs() {
                match k.as_ref() {
                    "lat" | "latitude" => {
                        lat = v.parse().ok();
                    }
                    "lng" | "lon" | "longitude" => {
                        lon = v.parse().ok();
                    }
                    "title" | "name" | "q" => {
                        let t = v.trim().to_string();
                        if !t.is_empty() {
                            title = Some(t);
                        }
                    }
                    "address" | "addr" => {
                        let a = v.trim().to_string();
                        if !a.is_empty() {
                            address = Some(a);
                        }
                    }
                    "source" | "source_url" => {
                        let s = v.trim().to_string();
                        if !s.is_empty() {
                            source_url = Some(s);
                        }
                    }
                    "category" => {
                        let c = v.trim().to_string();
                        if !c.is_empty() {
                            category = Some(c);
                        }
                    }
                    _ => {}
                }
            }

            if lat.is_none() || lon.is_none() {
                let query = address.as_deref().or(title.as_deref());
                if let Some(q) = query {
                    if let Ok(Some(geo)) = ctx.geocoder.geocode(q).await {
                        lat = Some(geo.latitude);
                        lon = Some(geo.longitude);
                        if address.is_none() {
                            address = Some(geo.display_name);
                        }
                    }
                }
            }

            if lat.is_none() || lon.is_none() {
                return Err(
                    "Cannot ingest bList URL: Missing location coordinates or place details."
                        .to_string(),
                );
            }

            let final_title = title
                .or_else(|| address.clone())
                .unwrap_or_else(|| "Saved Place".to_string());
            let final_source = source_url.unwrap_or_else(|| url.to_string());

            Ok(ScrapedMetadata {
                title: final_title,
                description: None,
                latitude: lat,
                longitude: lon,
                address,
                image_url: None,
                opening_hours: None,
                source_url: final_source,
                source_type: category.unwrap_or_else(|| "blist".to_string()),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Generic HTML Scraper (Fallback for Any Website)
// ---------------------------------------------------------------------------

pub struct GenericHtmlScraper;

impl LinkScraper for GenericHtmlScraper {
    fn name(&self) -> &'static str {
        "generic_html"
    }

    fn can_handle(&self, _url: &str) -> bool {
        true
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        ctx: &'a ScraperContext,
    ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
        Box::pin(async move {
            let (_, html_text) = ctx.fetch_html(url).await?;

            let (title, description, image_url, mut parsed_lat, mut parsed_lon, mut address) = {
                let jsonld = extract_jsonld_place_metadata(&html_text);
                let document = Html::parse_document(&html_text);

                let mut t = jsonld.as_ref().and_then(|j| j.name.clone());
                if t.is_none() {
                    t = extract_meta_content(&document, "meta[property='og:title']")
                        .or_else(|| extract_tag_text(&document, "title"))
                        .map(|s| clean_page_title(&s));
                }
                let title = t.unwrap_or_else(|| "Saved Location".to_string());

                let mut desc = jsonld.as_ref().and_then(|j| j.description.clone());
                if desc.is_none() {
                    desc = extract_meta_content(&document, "meta[property='og:description']")
                        .or_else(|| extract_meta_content(&document, "meta[name='description']"));
                }

                let mut img = jsonld.as_ref().and_then(|j| j.image_url.clone());
                if img.is_none() {
                    img = extract_meta_content(&document, "meta[property='og:image']");
                }

                let mut la = jsonld.as_ref().and_then(|j| j.latitude);
                let mut lo = jsonld.as_ref().and_then(|j| j.longitude);
                let addr = jsonld.as_ref().and_then(|j| j.address.clone());

                // Wikipedia and microformats: check <span class="geo"> or <span class="latitude">
                if la.is_none() || lo.is_none() {
                    if let Some((geo_la, geo_lo)) = extract_microformat_coordinates(&document) {
                        la = Some(geo_la);
                        lo = Some(geo_lo);
                    }
                }

                if la.is_none() {
                    if let Some(pos) = extract_meta_content(&document, "meta[name='geo.position']")
                    {
                        let parts: Vec<&str> = pos.split(';').collect();
                        if parts.len() == 2 {
                            la = parts[0].trim().parse().ok();
                            lo = parts[1].trim().parse().ok();
                        }
                    }
                }

                if la.is_none() {
                    if let Some(icbm) = extract_meta_content(&document, "meta[name='ICBM']") {
                        let parts: Vec<&str> = icbm.split(',').collect();
                        if parts.len() == 2 {
                            la = parts[0].trim().parse().ok();
                            lo = parts[1].trim().parse().ok();
                        }
                    }
                }

                if la.is_none() {
                    let og_lat =
                        extract_meta_content(&document, "meta[property='place:location:latitude']");
                    let og_lon = extract_meta_content(
                        &document,
                        "meta[property='place:location:longitude']",
                    );
                    if let (Some(la_str), Some(lo_str)) = (og_lat, og_lon) {
                        la = la_str.trim().parse().ok();
                        lo = lo_str.trim().parse().ok();
                    }
                }

                (title, desc, img, la, lo, addr)
            };

            if parsed_lat.is_none() || parsed_lon.is_none() {
                let search_target = if let Some(ref a) = address {
                    format!("{}, {}", title, a)
                } else {
                    title.clone()
                };
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&search_target).await {
                    parsed_lat = Some(geo.latitude);
                    parsed_lon = Some(geo.longitude);
                    if address.is_none() {
                        address = Some(geo.display_name);
                    }
                }
            } else if let (Some(la), Some(lo)) = (parsed_lat, parsed_lon) {
                if address.is_none() {
                    if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                        address = Some(addr);
                    }
                }
            }

            let mut meta = ScrapedMetadata {
                title,
                description,
                latitude: parsed_lat,
                longitude: parsed_lon,
                address,
                image_url,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "article".to_string(),
            };
            clean_metadata_concise(&mut meta);
            Ok(meta)
        })
    }
}

// ---------------------------------------------------------------------------
// Unified Scraper Service & Registry
// ---------------------------------------------------------------------------

pub struct Scraper {
    context: Arc<ScraperContext>,
    scrapers: Vec<Arc<dyn LinkScraper>>,
}

impl Scraper {
    /// Creates a default scraper registering all built-in domain scrapers.
    pub fn new() -> Self {
        Self::with_geocoder(Arc::new(Geocoder::new()))
    }

    /// Creates a scraper service using a specific Geocoder instance.
    pub fn with_geocoder(geocoder: Arc<Geocoder>) -> Self {
        let context = Arc::new(ScraperContext::new(geocoder));

        let scrapers: Vec<Arc<dyn LinkScraper>> = vec![
            Arc::new(GoogleMapsScraper),
            Arc::new(AppleMapsScraper),
            Arc::new(OpenStreetMapScraper),
            Arc::new(InstagramScraper),
            Arc::new(TikTokScraper),
            Arc::new(TripAdvisorScraper),
            Arc::new(YelpScraper),
            Arc::new(AllTrailsScraper),
            Arc::new(BListScraper),
            Arc::new(GenericHtmlScraper),
        ];

        Self { context, scrapers }
    }

    /// Registers a custom domain scraper at the top of the chain.
    #[allow(dead_code)]
    pub fn register<S: LinkScraper + 'static>(&mut self, scraper: S) {
        // Insert right before the fallback GenericHtmlScraper
        let insert_idx = if self.scrapers.is_empty() {
            0
        } else {
            self.scrapers.len() - 1
        };
        self.scrapers.insert(insert_idx, Arc::new(scraper));
    }

    /// Registers an Arc-wrapped scraper.
    #[allow(dead_code)]
    pub fn register_arc(&mut self, scraper: Arc<dyn LinkScraper>) {
        let insert_idx = if self.scrapers.is_empty() {
            0
        } else {
            self.scrapers.len() - 1
        };
        self.scrapers.insert(insert_idx, scraper);
    }

    /// List all registered scraper identifiers.
    #[allow(dead_code)]
    pub fn registered_scrapers(&self) -> Vec<&'static str> {
        self.scrapers.iter().map(|s| s.name()).collect()
    }

    /// Scrapes a given URL or place string using the best matching registered domain scraper.
    pub async fn scrape_url(&self, raw_url: &str) -> Result<ScrapedMetadata, String> {
        // 0. Extract embedded URL and hint if message contains text + URL
        let (extracted_url, hint) = extract_embedded_url_and_hint(raw_url);
        let trimmed = extracted_url.trim();
        if trimmed.is_empty() {
            return Err("URL or location cannot be empty".to_string());
        }

        // 1. Direct geo: URI parsing (RFC 5870)
        if let Some((lat, lon, geo_q)) = parse_geo_uri(trimmed) {
            let mut title = geo_q.clone().or_else(|| hint.clone());
            let mut address = None;
            if let Ok(Some(addr)) = self.context.geocoder.reverse_geocode(lat, lon).await {
                if title.is_none() {
                    title = Some(
                        addr.split(',')
                            .next()
                            .unwrap_or("Saved Location")
                            .trim()
                            .to_string(),
                    );
                }
                address = Some(addr);
            }
            let final_title = title.unwrap_or_else(|| format!("{:.5}, {:.5}", lat, lon));
            let mut meta = ScrapedMetadata {
                title: final_title,
                description: None,
                latitude: Some(lat),
                longitude: Some(lon),
                address,
                image_url: None,
                opening_hours: None,
                source_url: trimmed.to_string(),
                source_type: "coordinates".to_string(),
            };
            clean_metadata_concise(&mut meta);
            return Ok(meta);
        }

        // 2. Direct Plus Code parsing (e.g. "87G8Q235+X8 Tokyo" or "849VCWC8+R9")
        if let Some((code, label)) = parse_plus_code_token(trimmed) {
            if let Some((lat, lon)) = plus_code::decode(&code) {
                let mut title = label.clone().or_else(|| hint.clone());
                let mut address = None;
                if let Ok(Some(addr)) = self.context.geocoder.reverse_geocode(lat, lon).await {
                    if title.is_none() {
                        title = Some(
                            addr.split(',')
                                .next()
                                .unwrap_or("Saved Location")
                                .trim()
                                .to_string(),
                        );
                    }
                    address = Some(addr);
                }
                let final_title = title.unwrap_or_else(|| code.clone());
                let mut meta = ScrapedMetadata {
                    title: final_title,
                    description: None,
                    latitude: Some(lat),
                    longitude: Some(lon),
                    address,
                    image_url: None,
                    opening_hours: None,
                    source_url: format!("https://plus.codes/{}", code),
                    source_type: "plus_code".to_string(),
                };
                clean_metadata_concise(&mut meta);
                return Ok(meta);
            }
        }

        // 3. Direct Raw Coordinates parsing (e.g. "37.7749, -122.4194")
        if let Some((lat, lon)) = parse_raw_coordinates(trimmed) {
            let mut title = hint.clone();
            let mut address = None;
            if let Ok(Some(addr)) = self.context.geocoder.reverse_geocode(lat, lon).await {
                if title.is_none() {
                    title = Some(
                        addr.split(',')
                            .next()
                            .unwrap_or("Saved Location")
                            .trim()
                            .to_string(),
                    );
                }
                address = Some(addr);
            }
            let final_title = title.unwrap_or_else(|| format!("{:.5}, {:.5}", lat, lon));
            let mut meta = ScrapedMetadata {
                title: final_title,
                description: None,
                latitude: Some(lat),
                longitude: Some(lon),
                address,
                image_url: None,
                opening_hours: None,
                source_url: format!("https://www.openstreetmap.org/?mlat={}&mlon={}", lat, lon),
                source_type: "coordinates".to_string(),
            };
            clean_metadata_concise(&mut meta);
            return Ok(meta);
        }

        // 4. If it's a bList URL with parameters, handle directly without external DNS checks
        for scraper in &self.scrapers {
            if scraper.name() == "blist" && scraper.can_handle(trimmed) {
                let mut meta = scraper.scrape(trimmed, &self.context).await?;
                clean_metadata_concise(&mut meta);
                return Ok(meta);
            }
        }

        // 5. Check if the input is a URL or a plain search text query / location name
        let has_explicit_scheme = trimmed.contains("://")
            || trimmed.starts_with("javascript:")
            || trimmed.starts_with("file:")
            || trimmed.starts_with("data:")
            || trimmed.starts_with("about:");
        let has_domain_format = trimmed.starts_with("www.")
            || (trimmed.contains('.')
                && !trimmed.contains(' ')
                && (trimmed.ends_with(".com")
                    || trimmed.ends_with(".org")
                    || trimmed.ends_with(".net")
                    || trimmed.ends_with(".io")
                    || trimmed.ends_with(".gl")
                    || trimmed.ends_with(".app")));

        let is_url = has_explicit_scheme || has_domain_format;

        if !is_url {
            // Direct geocoding for plain text place names (e.g. "Spain!", "Eiffel Tower", "Tokyo Tower")
            let clean_query = trimmed.trim_end_matches(['!', '?', '.', ',', ' ']).trim();
            if !clean_query.is_empty() {
                if let Ok(Some(geo)) = self.context.geocoder.geocode(clean_query).await {
                    let mut meta = ScrapedMetadata {
                        title: clean_query.to_string(),
                        description: None,
                        latitude: Some(geo.latitude),
                        longitude: Some(geo.longitude),
                        address: Some(geo.display_name),
                        image_url: None,
                        opening_hours: None,
                        source_url: format!(
                            "https://www.openstreetmap.org/search?query={}",
                            urlencoding::encode(clean_query)
                        ),
                        source_type: "geocoded".to_string(),
                    };
                    clean_metadata_concise(&mut meta);
                    return Ok(meta);
                }
            }
            return Err(format!(
                "Could not find location for '{}'. Try entering a more specific city or landmark.",
                trimmed
            ));
        }

        // 6. Validate URL strictly for SSRF
        let parsed_url = validate_url_for_ssrf(trimmed)?;
        let full_url = parsed_url.to_string();

        // 7. Find first registered domain scraper that can handle this URL
        for scraper in &self.scrapers {
            if scraper.can_handle(&full_url) {
                let res = scraper.scrape(&full_url, &self.context).await;
                if let Ok(mut meta) = res {
                    if meta.latitude.is_some() && meta.longitude.is_some() {
                        if meta.title.is_empty()
                            || meta.title == "Saved Place"
                            || meta.title == "Saved Location"
                        {
                            if let Some(h) = hint {
                                meta.title = h;
                            }
                        }
                        clean_metadata_concise(&mut meta);
                        return Ok(meta);
                    }
                    // If scraper succeeded but missing coordinates, geocode extracted title/address
                    if !meta.title.is_empty()
                        && meta.title != "Saved Place"
                        && meta.title != "Saved Location"
                    {
                        let search_target = if let Some(ref a) = meta.address {
                            format!("{}, {}", meta.title, a)
                        } else {
                            meta.title.clone()
                        };
                        if let Ok(Some(geo)) = self.context.geocoder.geocode(&search_target).await {
                            meta.latitude = Some(geo.latitude);
                            meta.longitude = Some(geo.longitude);
                            if meta.address.is_none() {
                                meta.address = Some(geo.display_name);
                            }
                            clean_metadata_concise(&mut meta);
                            return Ok(meta);
                        }
                    }
                    clean_metadata_concise(&mut meta);
                    return Ok(meta);
                } else if let Err(err_msg) = res {
                    // Fallback to URL path slug geocoding if network blocked (e.g. 403 bot check on Yelp/IG)
                    let path_segments: Vec<&str> = parsed_url
                        .path_segments()
                        .map(|c| c.collect())
                        .unwrap_or_default();
                    if let Some(last_seg) = path_segments.last() {
                        let slug = last_seg.replace(['-', '_', '+'], " ").trim().to_string();
                        if slug.len() >= 3 && slug.chars().any(|c| c.is_alphabetic()) {
                            if let Ok(Some(geo)) = self.context.geocoder.geocode(&slug).await {
                                let mut meta = ScrapedMetadata {
                                    title: to_title_case(&slug),
                                    description: None,
                                    latitude: Some(geo.latitude),
                                    longitude: Some(geo.longitude),
                                    address: Some(geo.display_name),
                                    image_url: None,
                                    opening_hours: None,
                                    source_url: full_url,
                                    source_type: "geocoded".to_string(),
                                };
                                clean_metadata_concise(&mut meta);
                                return Ok(meta);
                            }
                        }
                    }
                    return Err(err_msg);
                }
            }
        }

        Err("No matching scraper found for URL".to_string())
    }
}

impl Default for Scraper {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// HTML & Text Extraction Utility Functions
// ---------------------------------------------------------------------------

pub fn extract_meta_content(document: &Html, selector_str: &str) -> Option<String> {
    if let Ok(selector) = Selector::parse(selector_str) {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

pub fn extract_tag_text(document: &Html, tag_name: &str) -> Option<String> {
    if let Ok(selector) = Selector::parse(tag_name) {
        if let Some(element) = document.select(&selector).next() {
            let text = element.text().collect::<Vec<_>>().join(" ");
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub fn parse_google_maps_title_and_address(raw_title: &str) -> (Option<String>, Option<String>) {
    let mut text = raw_title.trim();
    for suffix in &[
        " - Google Maps",
        " · Google Maps",
        " - Google Search",
        " - Google",
    ] {
        if let Some(pos) = text.rfind(suffix) {
            text = text[..pos].trim();
        }
    }
    if text.is_empty() || text.eq_ignore_ascii_case("Google Maps") {
        return (None, None);
    }

    // 1. Check for " · " separator
    if let Some(pos) = text.find(" · ") {
        let title_part = text[..pos].trim();
        let address_part = text[pos + " · ".len()..].trim();
        let title = if !title_part.is_empty() {
            Some(title_part.to_string())
        } else {
            None
        };
        let addr = if !address_part.is_empty() {
            Some(address_part.to_string())
        } else {
            None
        };
        return (title, addr);
    }

    // 2. Check for pipe "|" or "%7C" category descriptor (e.g. "El Gallo Giro | Mexican, 91 S State St, Orem, UT 84058")
    let pipe_clean = text.replace("%7C", "|");
    if let Some(pos) = pipe_clean.find('|') {
        let title_part = pipe_clean[..pos].trim();
        let rest = pipe_clean[pos + 1..].trim();

        let title = if !title_part.is_empty() {
            Some(title_part.to_string())
        } else {
            None
        };

        let addr = if let Some(comma_pos) = rest.find(',') {
            let addr_str = rest[comma_pos + 1..].trim();
            if !addr_str.is_empty() {
                Some(addr_str.to_string())
            } else {
                None
            }
        } else if !rest.is_empty() {
            Some(rest.to_string())
        } else {
            None
        };

        return (title, addr);
    }

    (Some(text.to_string()), None)
}

pub fn clean_page_title(title: &str) -> String {
    let mut cleaned = title.trim().to_string();
    let suffixes = [
        " • Instagram photos and videos",
        " | Instagram",
        " on Instagram",
        " - Wikipedia",
        " | TikTok",
        " on TikTok",
        " - TripAdvisor",
        " - Tripadvisor",
        " | Yelp",
        " | AllTrails",
    ];
    for suffix in suffixes {
        if let Some(pos) = cleaned.rfind(suffix) {
            cleaned = cleaned[..pos].trim().to_string();
        }
    }

    let separators = [" | ", " - ", " – ", " — ", " • "];
    for sep in separators {
        if let Some(pos) = cleaned.rfind(sep) {
            let candidate = cleaned[..pos].trim();
            if candidate.len() > 3 {
                cleaned = candidate.to_string();
            }
        }
    }
    cleaned
}

/// Converts a slug or string into Title Case.
pub fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => {
                    f.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Detects and parses Full Open Location Codes (Plus Codes), e.g. "87G8Q235+X8 Tokyo" -> ("87G8Q235+X8", Some("Tokyo"))
pub fn parse_plus_code_token(s: &str) -> Option<(String, Option<String>)> {
    let trimmed = s.trim();
    let re_code =
        Regex::new(r"(?i)\b([23456789CFGHJMPQRVWX]{4,8}\+[23456789CFGHJMPQRVWX]{2,7})\b").unwrap();

    if let Some(caps) = re_code.captures(trimmed) {
        let matched = caps.get(1)?;
        let code = matched.as_str().to_uppercase();
        if plus_code::is_full(&code) {
            let label_part = trimmed
                .replace(matched.as_str(), "")
                .trim_matches([' ', ',', ';', '-', '—', '|', ':'])
                .trim()
                .to_string();
            let label = if !label_part.is_empty() {
                Some(label_part)
            } else {
                None
            };
            return Some((code, label));
        }
    }

    None
}

/// Parses RFC 5870 `geo:` URIs (e.g. `geo:37.7749,-122.4194?q=San+Francisco`).
pub fn parse_geo_uri(s: &str) -> Option<(f64, f64, Option<String>)> {
    let lower = s.trim();
    if !lower.to_lowercase().starts_with("geo:") {
        return None;
    }

    let remainder = &lower[4..];
    let (coords_part, query_part) = if let Some(idx) = remainder.find('?') {
        (&remainder[..idx], Some(&remainder[idx + 1..]))
    } else {
        (remainder, None)
    };

    let coords_clean = if let Some(idx) = coords_part.find(';') {
        &coords_part[..idx]
    } else {
        coords_part
    };

    let mut q_label = None;
    if let Some(qp) = query_part {
        for pair in qp.split('&') {
            if let Some(stripped) = pair.strip_prefix("q=") {
                let decoded = urlencoding::decode(stripped)
                    .unwrap_or_default()
                    .replace('+', " ");
                let clean_q = decoded.trim().to_string();
                if !clean_q.is_empty() {
                    q_label = Some(clean_q);
                }
            }
        }
    }

    // Check if q has (Label) and coordinates: e.g. "37.7749,-122.4194(Golden Gate Bridge)"
    if let Some(ref q) = q_label {
        let re_q_coord = Regex::new(r"^(-?\d+\.\d+)\s*,\s*(-?\d+\.\d+)(?:\((.*?)\))?$").unwrap();
        if let Some(caps) = re_q_coord.captures(q) {
            let lat: f64 = caps.get(1)?.as_str().parse().ok()?;
            let lon: f64 = caps.get(2)?.as_str().parse().ok()?;
            let label = caps
                .get(3)
                .map(|m| m.as_str().trim().to_string())
                .filter(|s| !s.is_empty());
            if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                return Some((lat, lon, label));
            }
        }
    }

    let parts: Vec<&str> = coords_clean.split(',').collect();
    if parts.len() >= 2 {
        let lat: f64 = parts[0].trim().parse().ok()?;
        let lon: f64 = parts[1].trim().parse().ok()?;
        if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
            if lat.abs() < f64::EPSILON && lon.abs() < f64::EPSILON && q_label.is_some() {
                return None;
            }
            return Some((lat, lon, q_label));
        }
    }

    None
}

/// Parses plain coordinate strings (e.g. "37.7749, -122.4194" or "48.8584 N, 2.2945 E").
pub fn parse_raw_coordinates(s: &str) -> Option<(f64, f64)> {
    let trimmed = s.trim().trim_matches(['(', ')', '[', ']']);
    let re_coord = Regex::new(
        r#"^(-?\d{1,2}(?:\.\d+)?)\s*°?\s*([NSns])?\s*[,;\s]\s*(-?\d{1,3}(?:\.\d+)?)\s*°?\s*([EWew])?$"#,
    )
    .unwrap();

    if let Some(caps) = re_coord.captures(trimmed) {
        let mut lat: f64 = caps.get(1)?.as_str().parse().ok()?;
        if let Some(ns) = caps.get(2).map(|m| m.as_str().to_uppercase()) {
            if ns == "S" && lat > 0.0 {
                lat = -lat;
            }
        }

        let mut lon: f64 = caps.get(3)?.as_str().parse().ok()?;
        if let Some(ew) = caps.get(4).map(|m| m.as_str().to_uppercase()) {
            if ew == "W" && lon > 0.0 {
                lon = -lon;
            }
        }

        if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
            return Some((lat, lon));
        }
    }

    None
}

/// Separates embedded URLs and user hints from shared messages.
pub fn extract_embedded_url_and_hint(text: &str) -> (String, Option<String>) {
    let trimmed = text.trim();
    let re_url = Regex::new(r"https?://[^\s<>\[\]]+").unwrap();

    if let Some(m) = re_url.find(trimmed) {
        let url_str = m.as_str().to_string();
        let before = trimmed[..m.start()].trim();
        let after = trimmed[m.end()..].trim();

        let hint_raw = if !before.is_empty() {
            before
        } else if !after.is_empty() {
            after
        } else {
            ""
        };

        let hint_clean = hint_raw
            .trim_start_matches(|c: char| c == ':' || c == '-' || c == '—' || c.is_whitespace())
            .trim_end_matches(|c: char| c == ':' || c == '-' || c == '—' || c.is_whitespace());

        let hint_no_check =
            Regex::new(r"(?i)^(?:check out|look at|view on|shared from)\s+").unwrap();
        let hint_final = hint_no_check.replace(hint_clean, "").trim().to_string();

        let hint_opt = if hint_final.len() >= 2 && !hint_final.starts_with("http") {
            Some(hint_final)
        } else {
            None
        };

        (url_str, hint_opt)
    } else {
        (trimmed.to_string(), None)
    }
}

/// Cleans social captions (Instagram/TikTok), separating a concise title from description.
pub fn clean_social_caption(raw: &str) -> (String, Option<String>) {
    let mut text = raw.trim();

    let re_social_prefix = Regex::new(r#"^.*?\s+on\s+(?:Instagram|TikTok):\s*["“]?"#).unwrap();
    if let Some(m) = re_social_prefix.find(text) {
        text = text[m.end()..].trim();
    }

    let text = text.trim_end_matches(['"', '”', '\'', ' ']);
    let text = clean_page_title(text);
    let without_tags = strip_hashtag_wall(&text);

    if without_tags.is_empty() {
        return ("Social Post".to_string(), None);
    }

    let parts: Vec<&str> = without_tags.splitn(2, '\n').collect();
    if parts.len() == 2 && !parts[0].trim().is_empty() {
        let title = truncate_to_word_boundary(parts[0].trim(), 80);
        let desc = truncate_to_word_boundary(without_tags.trim(), 280);
        return (title, Some(desc));
    }

    if without_tags.len() > 80 {
        if let Some(idx) = without_tags.find(['!', '.', '?']) {
            if idx <= 80 && idx > 5 {
                let title = without_tags[..=idx].trim().to_string();
                let desc = truncate_to_word_boundary(&without_tags, 280);
                return (title, Some(desc));
            }
        }
        let title = truncate_to_word_boundary(&without_tags, 70);
        let desc = truncate_to_word_boundary(&without_tags, 280);
        (title, Some(desc))
    } else {
        (without_tags, None)
    }
}

/// Removes trailing hashtag walls (e.g. "#travel #food #tokyo") from descriptions.
pub fn strip_hashtag_wall(text: &str) -> String {
    let re_trailing_tags = Regex::new(r"(?:\s*#[A-Za-z0-9_]+)+\s*$").unwrap();
    let cleaned = re_trailing_tags.replace(text, "");
    cleaned.trim().to_string()
}

/// Truncates text cleanly at word boundaries.
pub fn truncate_to_word_boundary(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_len {
        return trimmed.to_string();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let mut cut = max_len;
    while cut > 0 && !chars[cut - 1].is_whitespace() {
        cut -= 1;
    }
    if cut == 0 {
        cut = max_len;
    }
    let s: String = chars[..cut].iter().collect();
    format!("{}...", s.trim_end_matches([' ', ',', '.', ';', ':', '-']))
}

/// Checks if a description string is generic boilerplate (cookie policy, robot check, login notice).
pub fn is_boilerplate_description(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("create an account or log in to instagram")
        || lower.contains("log in to instagram")
        || lower.contains("log in to tiktok")
        || lower.contains("watch more videos on tiktok")
        || lower.contains("before you continue to google")
        || lower.contains("we use cookies")
        || lower.contains("accept all cookies")
        || lower.contains("enable javascript")
        || lower.contains("access denied")
        || lower.contains("checking your browser")
        || lower.contains("please turn javascript on")
        || lower.contains("captcha")
        || lower.contains("robot check")
        || lower.contains("403 forbidden")
        || lower.contains("404 not found")
}

/// Enforces concise, clean metadata by pruning duplicate addresses, boilerplate, and wordy titles.
pub fn clean_metadata_concise(meta: &mut ScrapedMetadata) {
    let raw_title = meta.title.trim();
    if !raw_title.is_empty() {
        meta.title = clean_page_title(raw_title);
    }

    let (t_opt, a_opt) = parse_google_maps_title_and_address(&meta.title);
    if let Some(t) = t_opt {
        if !t.is_empty() {
            meta.title = t;
        }
    }
    if meta.address.is_none() && a_opt.is_some() {
        meta.address = a_opt;
    }

    if let Some(desc) = meta.description.take() {
        let trimmed_desc = desc.trim();
        let is_empty = trimmed_desc.is_empty();
        let is_boilerplate = is_boilerplate_description(trimmed_desc);

        let same_as_addr = meta
            .address
            .as_deref()
            .map(|a| a.trim().eq_ignore_ascii_case(trimmed_desc))
            .unwrap_or(false);
        let same_as_title = meta.title.trim().eq_ignore_ascii_case(trimmed_desc);

        if !is_empty && !is_boilerplate && !same_as_addr && !same_as_title {
            let without_hashtags = strip_hashtag_wall(trimmed_desc);
            let final_desc = truncate_to_word_boundary(&without_hashtags, 280);
            if !final_desc.is_empty() && !is_boilerplate_description(&final_desc) {
                meta.description = Some(final_desc);
            }
        }
    }
}

/// Parses Yelp page titles into clean place names and addresses.
pub fn parse_yelp_title_and_address(raw_title: &str) -> (Option<String>, Option<String>) {
    let mut text = raw_title.trim();
    for s in [" - Yelp", " | Yelp"] {
        if let Some(pos) = text.rfind(s) {
            text = text[..pos].trim();
        }
    }

    if text.is_empty() {
        return (None, None);
    }

    let parts: Vec<&str> = text.split(" - ").collect();
    if parts.is_empty() {
        return (None, None);
    }

    let raw_name = parts[0].trim();
    let mut title = raw_name.to_string();
    let mut address: Option<String> = None;

    if let Some(pos) = raw_name.find(',') {
        let t_cand = raw_name[..pos].trim();
        let c_cand = raw_name[pos + 1..].trim();
        if !t_cand.is_empty() {
            title = t_cand.to_string();
            if !c_cand.is_empty() {
                address = Some(c_cand.to_string());
            }
        }
    }

    for part in parts.iter().skip(1) {
        let p = part.trim();
        if p.contains("Reviews")
            || p.contains("Photos")
            || p.contains("Phone Number")
            || p.contains("Updated ")
            || p.contains("Order Online")
        {
            continue;
        }
        if p.contains(',') || p.chars().any(|c| c.is_ascii_digit()) {
            address = Some(p.to_string());
            break;
        } else if address.is_none() && p.len() > 2 {
            address = Some(p.to_string());
        }
    }

    let final_title = to_title_case(&title);
    (Some(final_title), address)
}

/// Parses TripAdvisor page titles into clean place names and addresses.
pub fn parse_tripadvisor_title_and_address(raw_title: &str) -> (Option<String>, Option<String>) {
    let mut text = raw_title.trim();
    let suffixes = [
        " - Tripadvisor",
        " - TripAdvisor",
        " | Tripadvisor",
        " | TripAdvisor",
    ];
    for s in suffixes {
        if let Some(pos) = text.rfind(s) {
            text = text[..pos].trim();
        }
    }

    if text.is_empty() {
        return (None, None);
    }

    let re_reviews = Regex::new(
        r"(?i)\s*-\s*(?:Updated\s+\d{4}|All You Need to Know|Menu,\s*Prices|Restaurant Reviews|Hotel Reviews).*",
    )
    .unwrap();
    let cleaned = re_reviews.replace(text, "");
    let mut parts: Vec<&str> = cleaned.split(" - ").collect();

    if parts.is_empty() {
        return (None, None);
    }

    let first_part = parts.remove(0).trim();
    let mut title = first_part.to_string();
    let mut address = None;

    if let Some(comma_pos) = first_part.find(',') {
        let possible_title = first_part[..comma_pos].trim();
        let possible_city = first_part[comma_pos + 1..].trim();
        if !possible_title.is_empty() {
            title = possible_title.to_string();
            if !possible_city.is_empty() {
                address = Some(possible_city.to_string());
            }
        }
    }

    if let Some(second) = parts.first() {
        let sec_trim = second.trim();
        if !sec_trim.is_empty() && !sec_trim.starts_with("See ") && !sec_trim.starts_with("Read ") {
            if let Some(ref a) = address {
                address = Some(format!("{}, {}", sec_trim, a));
            } else {
                address = Some(sec_trim.to_string());
            }
        }
    }

    let final_title = to_title_case(&title);
    (Some(final_title), address)
}

/// Parses AllTrails page titles into trail names and regions.
pub fn parse_alltrails_title_and_region(raw_title: &str) -> (Option<String>, Option<String>) {
    let mut text = raw_title.trim();
    for s in [" | AllTrails", " - AllTrails"] {
        if let Some(pos) = text.rfind(s) {
            text = text[..pos].trim();
        }
    }

    if text.is_empty() {
        return (None, None);
    }

    let re_rev = Regex::new(r"(?i)\s*-\s*\d[\d,]*\s*Reviews.*").unwrap();
    let cleaned = re_rev.replace(text, "");

    let parts: Vec<&str> = cleaned.split(" - ").collect();
    if parts.len() >= 2 {
        let trail = parts[0].trim().to_string();
        let region = parts[1].trim().to_string();
        return (
            Some(trail),
            if region.is_empty() {
                None
            } else {
                Some(region)
            },
        );
    }

    if let Some(pos) = cleaned.find(',') {
        let trail = cleaned[..pos].trim().to_string();
        let region = cleaned[pos + 1..].trim().to_string();
        return (
            Some(trail),
            if region.is_empty() {
                None
            } else {
                Some(region)
            },
        );
    }

    (Some(cleaned.to_string()), None)
}

/// Extracts Wikipedia and microformat coordinates (e.g. `<span class="geo">48.8584; 2.2945</span>`).
pub fn extract_microformat_coordinates(document: &Html) -> Option<(f64, f64)> {
    if let Ok(sel) = Selector::parse("span.geo") {
        if let Some(el) = document.select(&sel).next() {
            let text = el.text().collect::<Vec<_>>().join(" ");
            let parts: Vec<&str> = text.split([';', ',']).collect();
            if parts.len() == 2 {
                if let (Ok(lat), Ok(lon)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                ) {
                    if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                        return Some((lat, lon));
                    }
                }
            }
        }
    }

    if let (Ok(sel_lat), Ok(sel_lon)) = (
        Selector::parse("span.latitude"),
        Selector::parse("span.longitude"),
    ) {
        let lat_opt = document
            .select(&sel_lat)
            .next()
            .and_then(|el| el.text().collect::<String>().trim().parse::<f64>().ok());
        let lon_opt = document
            .select(&sel_lon)
            .next()
            .and_then(|el| el.text().collect::<String>().trim().parse::<f64>().ok());
        if let (Some(lat), Some(lon)) = (lat_opt, lon_opt) {
            if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                return Some((lat, lon));
            }
        }
    }

    None
}

#[derive(Debug, Default, Clone)]
pub struct JsonLdPlace {
    pub name: Option<String>,
    pub description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub address: Option<String>,
    pub image_url: Option<String>,
}

/// Extracts Schema.org Place / Restaurant / Attraction metadata from JSON-LD script blocks.
pub fn extract_jsonld_place_metadata(html: &str) -> Option<JsonLdPlace> {
    let re_script =
        Regex::new(r#"(?is)<script[^>]*type=["']application/ld\+json["'][^>]*>(.*?)</script>"#)
            .ok()?;

    for caps in re_script.captures_iter(html) {
        if let Some(m) = caps.get(1) {
            let raw_json = m.as_str().trim();
            if raw_json.is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<Value>(raw_json) {
                if let Some(place) = parse_jsonld_value(&val) {
                    return Some(place);
                }
            }
        }
    }

    None
}

fn parse_jsonld_value(val: &Value) -> Option<JsonLdPlace> {
    match val {
        Value::Array(arr) => {
            for item in arr {
                if let Some(p) = parse_jsonld_value(item) {
                    return Some(p);
                }
            }
            None
        }
        Value::Object(map) => {
            if let Some(graph) = map.get("@graph") {
                if let Some(p) = parse_jsonld_value(graph) {
                    return Some(p);
                }
            }

            let type_match = map
                .get("@type")
                .map(|t| match t {
                    Value::String(s) => is_place_schema_type(s),
                    Value::Array(arr) => arr
                        .iter()
                        .any(|v| v.as_str().map(is_place_schema_type).unwrap_or(false)),
                    _ => false,
                })
                .unwrap_or(false);

            let mut place = JsonLdPlace::default();

            if let Some(name_val) = map.get("name").and_then(|v| v.as_str()) {
                let clean = name_val.trim();
                if !clean.is_empty() {
                    place.name = Some(clean.to_string());
                }
            }

            if let Some(desc_val) = map.get("description").and_then(|v| v.as_str()) {
                let clean = desc_val.trim();
                if !clean.is_empty() {
                    place.description = Some(clean.to_string());
                }
            }

            if let Some(img_val) = map.get("image") {
                match img_val {
                    Value::String(s) => place.image_url = Some(s.clone()),
                    Value::Object(o) => {
                        if let Some(url_val) = o.get("url").and_then(|v| v.as_str()) {
                            place.image_url = Some(url_val.to_string());
                        }
                    }
                    Value::Array(arr) => {
                        if let Some(first) = arr.first() {
                            if let Some(s) = first.as_str() {
                                place.image_url = Some(s.to_string());
                            } else if let Some(url_val) = first.get("url").and_then(|v| v.as_str())
                            {
                                place.image_url = Some(url_val.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }

            if let Some(geo) = map.get("geo") {
                let (la, lo) = parse_geo_coordinates(geo);
                place.latitude = la;
                place.longitude = lo;
            }

            if let Some(addr_val) = map.get("address") {
                place.address = parse_jsonld_address(addr_val);
            }

            if type_match
                || place.latitude.is_some()
                || (place.name.is_some() && place.address.is_some())
            {
                return Some(place);
            }

            None
        }
        _ => None,
    }
}

fn is_place_schema_type(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("place")
        || lower.contains("restaurant")
        || lower.contains("foodestablishment")
        || lower.contains("touristattraction")
        || lower.contains("lodgingbusiness")
        || lower.contains("hotel")
        || lower.contains("park")
        || lower.contains("localbusiness")
        || lower.contains("store")
        || lower.contains("landmark")
        || lower.contains("civicstructure")
}

fn parse_geo_coordinates(geo: &Value) -> (Option<f64>, Option<f64>) {
    let mut lat = None;
    let mut lon = None;

    if let Some(obj) = geo.as_object() {
        if let Some(v) = obj.get("latitude") {
            lat = v
                .as_f64()
                .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()));
        }
        if let Some(v) = obj.get("longitude") {
            lon = v
                .as_f64()
                .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()));
        }
    }

    (lat, lon)
}

fn parse_jsonld_address(addr: &Value) -> Option<String> {
    match addr {
        Value::String(s) => {
            let clean = s.trim();
            if !clean.is_empty() {
                Some(clean.to_string())
            } else {
                None
            }
        }
        Value::Object(obj) => {
            let mut parts = Vec::new();
            if let Some(street) = obj.get("streetAddress").and_then(|v| v.as_str()) {
                let clean = street.trim();
                if !clean.is_empty() {
                    parts.push(clean.to_string());
                }
            }
            if let Some(locality) = obj.get("addressLocality").and_then(|v| v.as_str()) {
                let clean = locality.trim();
                if !clean.is_empty() {
                    parts.push(clean.to_string());
                }
            }
            if let Some(region) = obj.get("addressRegion").and_then(|v| v.as_str()) {
                let clean = region.trim();
                if !clean.is_empty() {
                    parts.push(clean.to_string());
                }
            }
            if let Some(postal) = obj.get("postalCode").and_then(|v| v.as_str()) {
                let clean = postal.trim();
                if !clean.is_empty() {
                    parts.push(clean.to_string());
                }
            }
            if let Some(country) = obj.get("addressCountry") {
                let c_str = match country {
                    Value::String(s) => s.as_str(),
                    Value::Object(co) => co.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    _ => "",
                };
                let clean = c_str.trim();
                if !clean.is_empty() {
                    parts.push(clean.to_string());
                }
            }

            if !parts.is_empty() {
                Some(parts.join(", "))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extracts the primary photo URL for a Google Maps location from metadata or embedded JS state.
pub fn extract_google_maps_photo(
    og_img: Option<&str>,
    twitter_img: Option<&str>,
    itemprop_img: Option<&str>,
    image_src_link: Option<&str>,
    html_text: &str,
) -> Option<String> {
    // 1. Check meta tags for direct place photos
    for candidate in [og_img, twitter_img, itemprop_img, image_src_link]
        .into_iter()
        .flatten()
    {
        let trimmed = candidate.trim();
        if !trimmed.is_empty()
            && (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
            && !trimmed.contains("maps_logo")
            && !trimmed.contains("googlelogo")
            && !trimmed.contains("branding/")
            && !trimmed.contains("static/images")
            && !trimmed.contains("favicon")
        {
            return Some(trimmed.to_string());
        }
    }

    // 2. Check embedded user photo uploads from Google Photos / Google Maps CDN
    // e.g. https://lh5.googleusercontent.com/p/AF1QipNX8...
    if let Ok(re_lh) = Regex::new(r#"https://lh\d+\.googleusercontent\.com/p/([A-Za-z0-9_-]+)"#) {
        if let Some(caps) = re_lh.captures(html_text) {
            if let Some(m) = caps.get(0) {
                let base = m.as_str();
                return Some(format!("{}=w800-h600-k-no", base));
            }
        }
    }

    // 3. Check Google Maps GPS proxy or photo service thumbnails
    if let Ok(re_proxy) = Regex::new(
        r#"https://(?:lh\d+\.googleusercontent\.com/gps-proxy|geo\d+\.ggpht\.com)/[A-Za-z0-9_\-\./]+"#,
    ) {
        if let Some(m) = re_proxy.find(html_text) {
            return Some(m.as_str().to_string());
        }
    }

    // 4. Check Street View thumbnail
    if let Ok(re_sv) =
        Regex::new(r#"https://streetviewpixels-pa\.googleapis\.com/v1/thumbnail\?[^"'\s\\]+"#)
    {
        if let Some(m) = re_sv.find(html_text) {
            return Some(m.as_str().replace("&amp;", "&"));
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scraper_registry_and_dispatch() {
        let scraper = Scraper::new();
        let names = scraper.registered_scrapers();
        assert!(names.contains(&"google_maps"));
        assert!(names.contains(&"apple_maps"));
        assert!(names.contains(&"openstreetmap"));
        assert!(names.contains(&"instagram"));
        assert!(names.contains(&"tiktok"));
        assert!(names.contains(&"tripadvisor"));
        assert!(names.contains(&"yelp"));
        assert!(names.contains(&"alltrails"));
        assert!(names.contains(&"generic_html"));
    }

    #[test]
    fn test_parse_plus_code_token() {
        let res1 = parse_plus_code_token("87G8Q235+X8 Tokyo");
        assert_eq!(
            res1,
            Some(("87G8Q235+X8".to_string(), Some("Tokyo".to_string())))
        );

        let res2 = parse_plus_code_token("849VCWC8+R9, Mountain View, CA");
        assert_eq!(
            res2,
            Some((
                "849VCWC8+R9".to_string(),
                Some("Mountain View, CA".to_string())
            ))
        );

        let res3 = parse_plus_code_token("8FW4V8FX+9H");
        assert_eq!(res3, Some(("8FW4V8FX+9H".to_string(), None)));

        let res4 = parse_plus_code_token("Not a plus code");
        assert_eq!(res4, None);
    }

    #[test]
    fn test_parse_geo_uri() {
        let (lat, lon, q) = parse_geo_uri("geo:37.7749,-122.4194").unwrap();
        assert!((lat - 37.7749).abs() < 1e-4);
        assert!((lon - (-122.4194)).abs() < 1e-4);
        assert_eq!(q, None);

        let (lat2, lon2, q2) = parse_geo_uri("geo:48.8584,2.2945?q=Eiffel+Tower").unwrap();
        assert!((lat2 - 48.8584).abs() < 1e-4);
        assert!((lon2 - 2.2945).abs() < 1e-4);
        assert_eq!(q2, Some("Eiffel Tower".to_string()));

        let (lat3, lon3, q3) =
            parse_geo_uri("geo:0,0?q=37.7749,-122.4194(Golden+Gate+Bridge)").unwrap();
        assert!((lat3 - 37.7749).abs() < 1e-4);
        assert!((lon3 - (-122.4194)).abs() < 1e-4);
        assert_eq!(q3, Some("Golden Gate Bridge".to_string()));

        assert_eq!(parse_geo_uri("https://example.com"), None);
    }

    #[test]
    fn test_parse_raw_coordinates() {
        let (lat1, lon1) = parse_raw_coordinates("37.7749, -122.4194").unwrap();
        assert!((lat1 - 37.7749).abs() < 1e-4);
        assert!((lon1 - (-122.4194)).abs() < 1e-4);

        let (lat2, lon2) = parse_raw_coordinates("48.8584° N, 2.2945° E").unwrap();
        assert!((lat2 - 48.8584).abs() < 1e-4);
        assert!((lon2 - 2.2945).abs() < 1e-4);

        let (lat3, lon3) = parse_raw_coordinates("33.8688 S, 151.2093 E").unwrap();
        assert!((lat3 - (-33.8688)).abs() < 1e-4);
        assert!((lon3 - 151.2093).abs() < 1e-4);

        assert_eq!(parse_raw_coordinates("Not coordinates"), None);
        assert_eq!(parse_raw_coordinates("190.0, 50.0"), None);
    }

    #[test]
    fn test_extract_embedded_url_and_hint() {
        let (url1, hint1) =
            extract_embedded_url_and_hint("Check out this place: https://maps.app.goo.gl/xyz123");
        assert_eq!(url1, "https://maps.app.goo.gl/xyz123");
        assert_eq!(hint1, Some("this place".to_string()));

        let (url2, hint2) =
            extract_embedded_url_and_hint("Tartine Bakery: https://maps.apple.com/?q=Tartine");
        assert_eq!(url2, "https://maps.apple.com/?q=Tartine");
        assert_eq!(hint2, Some("Tartine Bakery".to_string()));

        let (url3, hint3) = extract_embedded_url_and_hint("https://example.com/page");
        assert_eq!(url3, "https://example.com/page");
        assert_eq!(hint3, None);
    }

    #[test]
    fn test_clean_social_caption() {
        let raw_ig = r#"User on Instagram: "Delicious hand-pulled ramen in Shinjuku! 🍜 The broth is rich and simmered for 16 hours. #tokyo #ramen #japan #travel #foodie""#;
        let (title, desc) = clean_social_caption(raw_ig);
        assert_eq!(title, "Delicious hand-pulled ramen in Shinjuku!");
        assert!(desc.is_some());
        let d = desc.unwrap();
        assert!(d.contains("simmered for 16 hours"));
        assert!(!d.contains("#tokyo"));
        assert!(!d.contains("#foodie"));

        let short_post = "Amazing sunset at Bondi Beach";
        let (title2, desc2) = clean_social_caption(short_post);
        assert_eq!(title2, "Amazing sunset at Bondi Beach");
        assert_eq!(desc2, None);
    }

    #[test]
    fn test_parse_tripadvisor_title_and_address() {
        let raw = "LE JULES VERNE, Paris - 5, avenue Anatole France, 7th Arr. - Menu, Prices & Restaurant Reviews - Tripadvisor";
        let (t, a) = parse_tripadvisor_title_and_address(raw);
        assert_eq!(t, Some("Le Jules Verne".to_string()));
        assert!(a.is_some());
        let addr = a.unwrap();
        assert!(addr.contains("5, avenue Anatole France"));
        assert!(addr.contains("Paris"));
    }

    #[test]
    fn test_parse_yelp_title_and_address() {
        let raw = "TARTINE BAKERY - Updated March 2024 - 1577 Photos & 8769 Reviews - 600 Guerrero St, San Francisco, California - Bakeries - Phone Number - Yelp";
        let (t, a) = parse_yelp_title_and_address(raw);
        assert_eq!(t, Some("Tartine Bakery".to_string()));
        assert_eq!(
            a,
            Some("600 Guerrero St, San Francisco, California".to_string())
        );
    }

    #[test]
    fn test_parse_alltrails_title_and_region() {
        let raw = "Half Dome Trail - Yosemite National Park, California | AllTrails";
        let (t, r) = parse_alltrails_title_and_region(raw);
        assert_eq!(t, Some("Half Dome Trail".to_string()));
        assert_eq!(r, Some("Yosemite National Park, California".to_string()));
    }

    #[test]
    fn test_jsonld_place_extraction() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "Restaurant",
                "name": "Katz's Delicatessen",
                "description": "Legendary pastrami and corned beef sandwiches since 1888.",
                "image": "https://example.com/katzs.jpg",
                "geo": {
                    "@type": "GeoCoordinates",
                    "latitude": "40.7223",
                    "longitude": -73.9874
                },
                "address": {
                    "@type": "PostalAddress",
                    "streetAddress": "205 E Houston St",
                    "addressLocality": "New York",
                    "addressRegion": "NY",
                    "postalCode": "10002",
                    "addressCountry": "US"
                }
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let place = extract_jsonld_place_metadata(html).expect("parsed json-ld place");
        assert_eq!(place.name, Some("Katz's Delicatessen".to_string()));
        assert_eq!(
            place.description,
            Some("Legendary pastrami and corned beef sandwiches since 1888.".to_string())
        );
        assert_eq!(place.latitude, Some(40.7223));
        assert_eq!(place.longitude, Some(-73.9874));
        assert_eq!(
            place.address,
            Some("205 E Houston St, New York, NY, 10002, US".to_string())
        );
        assert_eq!(
            place.image_url,
            Some("https://example.com/katzs.jpg".to_string())
        );
    }

    #[test]
    fn test_extract_microformat_coordinates() {
        let html = r#"<html><body>
            <span class="geo">48.8584; 2.2945</span>
        </body></html>"#;
        let doc = Html::parse_document(html);
        let coords = extract_microformat_coordinates(&doc);
        assert_eq!(coords, Some((48.8584, 2.2945)));
    }

    #[test]
    fn test_clean_metadata_concise() {
        let mut meta = ScrapedMetadata {
            title: "Joe's Pizza · 1435 Broadway, New York, NY 10018 · Google Maps".to_string(),
            description: Some("1435 Broadway, New York, NY 10018".to_string()), // duplicate address!
            latitude: Some(40.75),
            longitude: Some(-73.98),
            address: Some("1435 Broadway, New York, NY 10018".to_string()),
            image_url: None,
            opening_hours: None,
            source_url: "https://maps.google.com/?q=Joe".to_string(),
            source_type: "google_maps".to_string(),
        };

        clean_metadata_concise(&mut meta);
        assert_eq!(meta.title, "Joe's Pizza");
        assert_eq!(meta.description, None); // duplicate address pruned!

        // Test boilerplate removal
        meta.description = Some("Before you continue to Google. We use cookies and data to deliver and maintain Google services.".to_string());
        clean_metadata_concise(&mut meta);
        assert_eq!(meta.description, None);
    }

    #[test]
    fn test_openstreetmap_scraper_can_handle_and_parsing() {
        let scraper = OpenStreetMapScraper;
        assert!(scraper.can_handle("https://www.openstreetmap.org/#map=16/51.5074/-0.1278"));
        assert!(scraper.can_handle("https://osm.org/?mlat=48.8584&mlon=2.2945"));
        assert!(!scraper.can_handle("https://maps.google.com/?q=London"));
    }

    #[test]
    fn test_custom_scraper_registration() {
        struct CustomDomainScraper;
        impl LinkScraper for CustomDomainScraper {
            fn name(&self) -> &'static str {
                "custom_blog"
            }
            fn can_handle(&self, url: &str) -> bool {
                url.contains("myfoodblog.com")
            }
            fn scrape<'a>(
                &'a self,
                url: &'a str,
                _ctx: &'a ScraperContext,
            ) -> BoxFuture<'a, Result<ScrapedMetadata, String>> {
                Box::pin(async move {
                    Ok(ScrapedMetadata {
                        title: "Best Croissant".to_string(),
                        description: Some("Bakery review".to_string()),
                        latitude: Some(48.85),
                        longitude: Some(2.35),
                        address: Some("Paris, France".to_string()),
                        image_url: None,
                        opening_hours: None,
                        source_url: url.to_string(),
                        source_type: "custom_blog".to_string(),
                    })
                })
            }
        }

        let mut scraper = Scraper::new();
        scraper.register(CustomDomainScraper);
        scraper.register_arc(Arc::new(CustomDomainScraper));
        let names = scraper.registered_scrapers();
        assert!(names.contains(&"custom_blog"));
        // Ensure generic_html remains last fallback
        assert_eq!(*names.last().unwrap(), "generic_html");
    }

    #[test]
    fn test_parse_google_maps_title_and_address() {
        let (title, addr) = parse_google_maps_title_and_address(
            "Chicken Boy · 5558 N Figueroa St, Los Angeles, CA 90042 · Google Maps",
        );
        assert_eq!(title, Some("Chicken Boy".to_string()));
        assert_eq!(
            addr,
            Some("5558 N Figueroa St, Los Angeles, CA 90042".to_string())
        );

        let (title2, addr2) = parse_google_maps_title_and_address("Space Needle - Google Maps");
        assert_eq!(title2, Some("Space Needle".to_string()));
        assert_eq!(addr2, None);

        let (title3, addr3) = parse_google_maps_title_and_address("Google Maps");
        assert_eq!(title3, None);
        assert_eq!(addr3, None);

        let (title4, addr4) = parse_google_maps_title_and_address(
            "El Gallo Giro | Mexican, 91 S State St, Orem, UT 84058",
        );
        assert_eq!(title4, Some("El Gallo Giro".to_string()));
        assert_eq!(addr4, Some("91 S State St, Orem, UT 84058".to_string()));
    }

    #[test]
    fn test_clean_page_title() {
        assert_eq!(
            clean_page_title("The Best Coffee in Tokyo | Travel Blog"),
            "The Best Coffee in Tokyo"
        );
        assert_eq!(
            clean_page_title("Delicious Ramen on Instagram"),
            "Delicious Ramen"
        );
        assert_eq!(
            clean_page_title("Half Dome Trail | AllTrails"),
            "Half Dome Trail"
        );
    }

    #[tokio::test]
    async fn test_scraper_ssrf_protection() {
        let scraper = Scraper::new();

        let blocked_urls = [
            "http://127.0.0.1:8080/secret",
            "http://localhost:3000",
            "http://169.254.169.254/latest/meta-data/",
            "http://metadata.google.internal/computeMetadata/v1/",
            "http://10.0.0.1/",
            "http://192.168.1.1/",
            "http://172.16.0.1/",
            "http://0.0.0.0:8000",
            "http://[::1]:8080",
            "http://[fe80::1]",
            "http://[fc00::1]",
            "file:///etc/passwd",
            "ftp://example.com/file",
            "javascript:alert(1)",
            "gopher://127.0.0.1:70/",
        ];

        for u in blocked_urls {
            let res = scraper.scrape_url(u).await;
            assert!(
                res.is_err(),
                "Expected scraping '{}' to fail SSRF check, got: {:?}",
                u,
                res
            );
        }
    }

    #[test]
    fn test_apple_maps_scraper_can_handle_and_url_extraction() {
        let scraper = AppleMapsScraper;
        assert!(scraper.can_handle("https://maps.apple.com/?ll=37.7749,-122.4194&q=San+Francisco"));
        assert!(
            scraper.can_handle("https://maps.apple.com/?address=1+Infinite+Loop,+Cupertino,+CA")
        );
        assert!(scraper.can_handle("http://maps.apple.com/place?auid=123456"));
        assert!(!scraper.can_handle("https://maps.google.com/?q=Paris"));
        assert!(!scraper.can_handle("https://www.instagram.com/p/123"));

        // Test coordinate regex
        let re_ll = Regex::new(r"[?&](?:ll|coordinate)=(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
        let caps = re_ll
            .captures("https://maps.apple.com/?ll=37.7749,-122.4194&q=SF")
            .unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "37.7749");
        assert_eq!(caps.get(2).unwrap().as_str(), "-122.4194");

        let caps_coord = re_ll
            .captures("https://maps.apple.com/?coordinate=48.8584,2.2945")
            .unwrap();
        assert_eq!(caps_coord.get(1).unwrap().as_str(), "48.8584");
        assert_eq!(caps_coord.get(2).unwrap().as_str(), "2.2945");

        // Test address regex
        let re_address = Regex::new(r"[?&]address=([^&]+)").unwrap();
        let caps_addr = re_address
            .captures("https://maps.apple.com/?address=1+Infinite+Loop,+Cupertino,+CA")
            .unwrap();
        let raw_addr = caps_addr.get(1).unwrap().as_str();
        let unencoded = urlencoding::decode(raw_addr).unwrap();
        let clean = unencoded.replace('+', " ");
        assert_eq!(clean, "1 Infinite Loop, Cupertino, CA");
    }

    #[test]
    fn test_google_maps_scraper_can_handle_and_url_patterns() {
        let scraper = GoogleMapsScraper;
        assert!(scraper.can_handle("https://maps.google.com/?q=Paris"));
        assert!(scraper
            .can_handle("https://www.google.com/maps/place/Tokyo+Tower/@35.6586,139.7454,17z"));
        assert!(scraper.can_handle("https://goo.gl/maps/xyz123"));
        assert!(scraper.can_handle("https://maps.app.goo.gl/abc456"));
        assert!(scraper.can_handle("https://www.google.com/maps/search/Sushi+Dai+Tokyo"));
        assert!(!scraper.can_handle("https://maps.apple.com/?q=Tokyo"));
        assert!(!scraper.can_handle("https://example.com/map"));

        // Test !3d and !4d coordinate regex
        let re_3d4d = Regex::new(r"!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)").unwrap();
        let url = "https://www.google.com/maps/place/Tokyo+Tower/@35.6585805,139.7454329,17z/data=!4m6!3m5!1s0x60188bbd9009a093:0x39a04a79d60f90e5!8m2!3d35.6585805!4d139.7454329";
        let caps = re_3d4d.captures(url).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "35.6585805");
        assert_eq!(caps.get(2).unwrap().as_str(), "139.7454329");

        // Test place path regex
        let re_place = Regex::new(r"/maps/place/([^/@?]+)").unwrap();
        let caps_place = re_place
            .captures(
                "https://www.google.com/maps/place/Grand+Canyon+National+Park/@36.0544,-112.1401",
            )
            .unwrap();
        let place_raw = caps_place.get(1).unwrap().as_str();
        let place_clean = urlencoding::decode(place_raw).unwrap().replace('+', " ");
        assert_eq!(place_clean, "Grand Canyon National Park");

        // Test search path regex
        let re_search = Regex::new(r"/maps/search/([^/@?]+)").unwrap();
        let caps_search = re_search
            .captures("https://www.google.com/maps/search/Best+Croissants+Paris")
            .unwrap();
        let search_raw = caps_search.get(1).unwrap().as_str();
        let search_clean = urlencoding::decode(search_raw).unwrap().replace('+', " ");
        assert_eq!(search_clean, "Best Croissants Paris");
    }

    #[test]
    fn test_social_and_directory_scrapers_can_handle() {
        assert!(InstagramScraper.can_handle("https://www.instagram.com/p/C_abc123/"));
        assert!(InstagramScraper.can_handle("https://instagr.am/reel/xyz789/"));
        assert!(!InstagramScraper.can_handle("https://twitter.com/post/123"));

        assert!(TikTokScraper.can_handle("https://www.tiktok.com/@foodie/video/123456789"));
        assert!(TripAdvisorScraper
            .can_handle("https://www.tripadvisor.com/Restaurant_Review-g60763-d12345"));
        assert!(YelpScraper.can_handle("https://www.yelp.com/biz/tartine-bakery-san-francisco"));
        assert!(AllTrailsScraper
            .can_handle("https://www.alltrails.com/trail/us/california/yosemite-falls"));
        assert!(GenericHtmlScraper.can_handle("https://anytravelblog.com/top-10-spots-rome"));
    }

    #[test]
    fn test_html_extraction_helpers() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <title>Central Park, NYC</title>
            <meta property="og:title" content="Central Park Iconic Green Space" />
            <meta property="og:description" content="5th Ave, New York, NY 10022" />
            <meta property="og:image" content="https://example.com/centralpark.jpg" />
            <meta itemprop="latitude" content="40.785091" />
            <meta itemprop="longitude" content="-73.968285" />
        </head>
        <body>
            <p>Welcome to NYC</p>
        </body>
        </html>"#;

        let doc = Html::parse_document(html);
        assert_eq!(
            extract_meta_content(&doc, "meta[property='og:title']"),
            Some("Central Park Iconic Green Space".to_string())
        );
        assert_eq!(
            extract_meta_content(&doc, "meta[property='og:description']"),
            Some("5th Ave, New York, NY 10022".to_string())
        );
        assert_eq!(
            extract_meta_content(&doc, "meta[property='og:image']"),
            Some("https://example.com/centralpark.jpg".to_string())
        );
        assert_eq!(
            extract_tag_text(&doc, "title"),
            Some("Central Park, NYC".to_string())
        );
        assert_eq!(
            extract_meta_content(&doc, "meta[itemprop='latitude']")
                .and_then(|s| s.parse::<f64>().ok()),
            Some(40.785091)
        );
        assert_eq!(
            extract_meta_content(&doc, "meta[itemprop='longitude']")
                .and_then(|s| s.parse::<f64>().ok()),
            Some(-73.968285)
        );
    }

    #[test]
    fn test_extract_google_maps_photo() {
        // Direct meta image
        let img1 = extract_google_maps_photo(
            Some("https://example.com/photos/sagrada_familia.jpg"),
            None,
            None,
            None,
            "",
        );
        assert_eq!(
            img1,
            Some("https://example.com/photos/sagrada_familia.jpg".to_string())
        );

        // Filters out generic google logos
        let img2 = extract_google_maps_photo(
            Some("https://maps.google.com/maps_logo.png"),
            None,
            None,
            None,
            r#"window.APP_INITIALIZATION_STATE=[[["https://lh5.googleusercontent.com/p/AF1QipNabc123xyz"]]];"#,
        );
        assert_eq!(
            img2,
            Some("https://lh5.googleusercontent.com/p/AF1QipNabc123xyz=w800-h600-k-no".to_string())
        );

        // Street View fallback
        let img3 = extract_google_maps_photo(
            None,
            None,
            None,
            None,
            r#"<img src="https://streetviewpixels-pa.googleapis.com/v1/thumbnail?panoid=abc&amp;w=400" />"#,
        );
        assert_eq!(
            img3,
            Some(
                "https://streetviewpixels-pa.googleapis.com/v1/thumbnail?panoid=abc&w=400"
                    .to_string()
            )
        );
    }

    #[tokio::test]
    async fn test_scraper_plain_location_geocoding_fallback() {
        let scraper = Scraper::new();
        let res = scraper.scrape_url("Paris, France").await;
        if let Ok(meta) = res {
            assert!(meta.latitude.is_some());
            assert!(meta.longitude.is_some());
            assert_eq!(meta.source_type, "geocoded");
        }
    }
}
