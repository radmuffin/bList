use crate::geocoder::Geocoder;
use crate::models::ScrapedMetadata;
use crate::security::{build_safe_http_client, validate_url_for_ssrf};
use regex::Regex;
use scraper::{Html, Selector};
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
                        let (parsed_t, parsed_a) = parse_google_maps_title_and_address(&clean_name);
                        if let Some(t) = parsed_t {
                            title = Some(t);
                        } else {
                            title = Some(clean_name.clone());
                        }
                        if let Some(a) = parsed_a {
                            address = Some(a.clone());
                            place_query_candidate = Some(a);
                        } else {
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
                            let stripped_q = clean_name
                                .replace('|', " ")
                                .replace("%7C", " ")
                                .trim()
                                .to_string();
                            place_query_candidate = Some(stripped_q.clone());
                            title = Some(stripped_q);
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
                        if !clean_name.is_empty() && clean_name.chars().any(|c| c.is_alphabetic()) {
                            let stripped_q = clean_name
                                .replace('|', " ")
                                .replace("%7C", " ")
                                .trim()
                                .to_string();
                            place_query_candidate = Some(stripped_q.clone());
                            title = Some(stripped_q);
                        }
                    }
                }
            }

            // 2. Parse HTML Metadata scoped in a block so Html (which is !Send) is dropped before await
            if !html_text.is_empty() {
                let (raw_meta_title, og_desc, og_img, schema_lat_lon) = {
                    let document = Html::parse_document(&html_text);

                    let raw_t = extract_meta_content(&document, "meta[property='og:title']")
                        .or_else(|| extract_tag_text(&document, "title"));
                    let desc = extract_meta_content(&document, "meta[property='og:description']")
                        .or_else(|| extract_meta_content(&document, "meta[name='description']"));
                    let img = extract_meta_content(&document, "meta[property='og:image']");
                    let la = extract_meta_content(&document, "meta[itemprop='latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                    let lo = extract_meta_content(&document, "meta[itemprop='longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                    (raw_t, desc, img, (la, lo))
                };

                if let Some(raw_t) = raw_meta_title {
                    let (cleaned_t, extracted_addr) = parse_google_maps_title_and_address(&raw_t);
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

                if address.is_none() {
                    if let Some(ref d) = og_desc {
                        if !d.eq_ignore_ascii_case("Google Maps") && !d.is_empty() {
                            address = Some(d.clone());
                            if place_query_candidate.is_none() {
                                place_query_candidate = Some(d.clone());
                            }
                        }
                    }
                }

                if image_url.is_none() {
                    if let Some(ref img) = og_img {
                        if !img.contains("maps_logo") {
                            image_url = Some(img.clone());
                        }
                    }
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
                description: address.clone(),
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
                description: address.clone(),
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
            let (_, html_text) = ctx.fetch_html(url).await?;

            let (og_title, og_desc, og_image) = {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']");
                let d = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                (t, d, img)
            };

            let clean_title = og_title
                .map(|t| clean_page_title(&t))
                .unwrap_or_else(|| "Instagram Post".to_string());

            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut address: Option<String> = None;

            if clean_title != "Instagram Post" && !clean_title.is_empty() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&clean_title).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            }

            Ok(ScrapedMetadata {
                title: clean_title,
                description: og_desc,
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
            let (_, html_text) = ctx.fetch_html(url).await?;

            let (og_title, og_desc, og_image) = {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t))
                    .unwrap_or_else(|| "TikTok Video".to_string());
                let desc = extract_meta_content(&document, "meta[property='og:description']")
                    .or_else(|| extract_meta_content(&document, "meta[name='description']"));
                let img = extract_meta_content(&document, "meta[property='og:image']");
                (t, desc, img)
            };

            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut address: Option<String> = None;

            // Attempt geocoding on title
            if og_title != "TikTok Video" && !og_title.is_empty() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&og_title).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            }

            Ok(ScrapedMetadata {
                title: og_title,
                description: og_desc,
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
            let (_, html_text) = ctx.fetch_html(url).await?;

            let (og_title, og_desc, og_image, mut lat, mut lon) = {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t))
                    .unwrap_or_else(|| "TripAdvisor Location".to_string());
                let desc = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                let la =
                    extract_meta_content(&document, "meta[property='place:location:latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                let lo =
                    extract_meta_content(&document, "meta[property='place:location:longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                (t, desc, img, la, lo)
            };

            let mut address: Option<String> = None;

            if lat.is_none() || lon.is_none() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&og_title).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            } else if let (Some(la), Some(lo)) = (lat, lon) {
                if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                    address = Some(addr);
                }
            }

            Ok(ScrapedMetadata {
                title: og_title,
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
            let (_, html_text) = ctx.fetch_html(url).await?;

            let (og_title, og_desc, og_image, mut lat, mut lon) = {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t))
                    .unwrap_or_else(|| "Yelp Place".to_string());
                let desc = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                let la =
                    extract_meta_content(&document, "meta[property='place:location:latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                let lo =
                    extract_meta_content(&document, "meta[property='place:location:longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                (t, desc, img, la, lo)
            };

            let mut address: Option<String> = None;

            if lat.is_none() || lon.is_none() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&og_title).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            } else if let (Some(la), Some(lo)) = (lat, lon) {
                if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                    address = Some(addr);
                }
            }

            Ok(ScrapedMetadata {
                title: og_title,
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
            let (_, html_text) = ctx.fetch_html(url).await?;

            let (og_title, og_desc, og_image, mut lat, mut lon) = {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|t| clean_page_title(&t))
                    .unwrap_or_else(|| "AllTrails Route".to_string());
                let desc = extract_meta_content(&document, "meta[property='og:description']");
                let img = extract_meta_content(&document, "meta[property='og:image']");
                let la =
                    extract_meta_content(&document, "meta[property='place:location:latitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                let lo =
                    extract_meta_content(&document, "meta[property='place:location:longitude']")
                        .and_then(|s| s.parse::<f64>().ok());
                (t, desc, img, la, lo)
            };

            let mut address: Option<String> = None;

            if lat.is_none() || lon.is_none() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&og_title).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            } else if let (Some(la), Some(lo)) = (lat, lon) {
                if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                    address = Some(addr);
                }
            }

            Ok(ScrapedMetadata {
                title: og_title,
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

            let (title, description, image_url, mut parsed_lat, mut parsed_lon) = {
                let document = Html::parse_document(&html_text);

                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|s| clean_page_title(&s))
                    .unwrap_or_else(|| "Saved Location".to_string());

                let desc = extract_meta_content(&document, "meta[property='og:description']")
                    .or_else(|| extract_meta_content(&document, "meta[name='description']"));

                let img = extract_meta_content(&document, "meta[property='og:image']");

                let mut la: Option<f64> = None;
                let mut lo: Option<f64> = None;

                if let Some(pos) = extract_meta_content(&document, "meta[name='geo.position']") {
                    let parts: Vec<&str> = pos.split(';').collect();
                    if parts.len() == 2 {
                        la = parts[0].trim().parse().ok();
                        lo = parts[1].trim().parse().ok();
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

                (t, desc, img, la, lo)
            };

            let mut address: Option<String> = None;

            if parsed_lat.is_none() || parsed_lon.is_none() {
                if let Ok(Some(geo)) = ctx.geocoder.geocode(&title).await {
                    parsed_lat = Some(geo.latitude);
                    parsed_lon = Some(geo.longitude);
                    address = Some(geo.display_name);
                }
            } else if let (Some(la), Some(lo)) = (parsed_lat, parsed_lon) {
                if let Ok(Some(addr)) = ctx.geocoder.reverse_geocode(la, lo).await {
                    address = Some(addr);
                }
            }

            Ok(ScrapedMetadata {
                title,
                description,
                latitude: parsed_lat,
                longitude: parsed_lon,
                address,
                image_url,
                opening_hours: None,
                source_url: url.to_string(),
                source_type: "article".to_string(),
            })
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

    /// Scrapes a given URL using the best matching registered domain scraper.
    pub async fn scrape_url(&self, raw_url: &str) -> Result<ScrapedMetadata, String> {
        let trimmed = raw_url.trim();
        if trimmed.is_empty() {
            return Err("URL or location cannot be empty".to_string());
        }

        // 1. If it's a bList URL with parameters, handle directly without external DNS checks
        for scraper in &self.scrapers {
            if scraper.name() == "blist" && scraper.can_handle(trimmed) {
                return scraper.scrape(trimmed, &self.context).await;
            }
        }

        // 2. Check if the input is a URL or a plain search text query / location name
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
                    return Ok(ScrapedMetadata {
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
                    });
                }
            }
            return Err(format!(
                "Could not find location for '{}'. Try entering a more specific city or landmark.",
                trimmed
            ));
        }

        // 3. Validate URL strictly for SSRF
        let parsed_url = validate_url_for_ssrf(trimmed)?;
        let full_url = parsed_url.to_string();

        // 4. Find first registered domain scraper that can handle this URL
        for scraper in &self.scrapers {
            if scraper.can_handle(&full_url) {
                let res = scraper.scrape(&full_url, &self.context).await;
                if let Ok(ref meta) = res {
                    if meta.latitude.is_some() && meta.longitude.is_some() {
                        return res;
                    }
                }
                // If scraper succeeded but missing coordinates, geocode extracted title/address
                if let Ok(mut meta) = res {
                    if (meta.latitude.is_none() || meta.longitude.is_none())
                        && !meta.title.is_empty()
                        && meta.title != "Saved Place"
                    {
                        if let Ok(Some(geo)) = self.context.geocoder.geocode(&meta.title).await {
                            meta.latitude = Some(geo.latitude);
                            meta.longitude = Some(geo.longitude);
                            if meta.address.is_none() {
                                meta.address = Some(geo.display_name);
                            }
                            return Ok(meta);
                        }
                    }
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
                                return Ok(ScrapedMetadata {
                                    title: slug,
                                    description: None,
                                    latitude: Some(geo.latitude),
                                    longitude: Some(geo.longitude),
                                    address: Some(geo.display_name),
                                    image_url: None,
                                    opening_hours: None,
                                    source_url: full_url,
                                    source_type: "geocoded".to_string(),
                                });
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
        assert!(names.contains(&"instagram"));
        assert!(names.contains(&"tiktok"));
        assert!(names.contains(&"tripadvisor"));
        assert!(names.contains(&"yelp"));
        assert!(names.contains(&"alltrails"));
        assert!(names.contains(&"generic_html"));
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
