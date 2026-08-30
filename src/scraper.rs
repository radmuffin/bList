use crate::geocoder::Geocoder;
use crate::models::ScrapedMetadata;
use crate::security::{build_safe_http_client, validate_url_for_ssrf};
use regex::Regex;
use scraper::{Html, Selector};
use std::time::Duration;

pub struct Scraper {
    client: reqwest::Client,
    geocoder: Geocoder,
}

impl Scraper {
    pub fn new() -> Self {
        let client = build_safe_http_client(Duration::from_secs(15));

        Self {
            client,
            geocoder: Geocoder::new(),
        }
    }

    pub async fn scrape_url(&self, raw_url: &str) -> Result<ScrapedMetadata, String> {
        let parsed_url = validate_url_for_ssrf(raw_url)?;
        let full_url = parsed_url.to_string();

        if full_url.contains("maps.google.")
            || full_url.contains("goo.gl/maps")
            || full_url.contains("maps.app.goo.gl")
            || full_url.contains("google.com/maps")
            || full_url.contains("/maps/place")
            || full_url.contains("/maps/search")
        {
            self.scrape_google_maps(&full_url).await
        } else if full_url.contains("maps.apple.com") {
            self.scrape_apple_maps(&full_url).await
        } else if full_url.contains("instagram.com") || full_url.contains("instagr.am") {
            self.scrape_instagram(&full_url).await
        } else {
            self.scrape_generic_page(&full_url).await
        }
    }

    async fn scrape_google_maps(&self, url: &str) -> Result<ScrapedMetadata, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Google Maps link: {}", e))?;

        let mut final_url = response.url().to_string();
        let mut html_text = response.text().await.unwrap_or_default();

        // Check for client-side meta refresh or JS redirect
        if html_text.contains("http-equiv=\"refresh\"") || html_text.contains("http-equiv='refresh'") {
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

        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;
        let mut title: Option<String> = None;
        let mut address: Option<String> = None;
        let mut image_url: Option<String> = None;
        let mut place_query_candidate: Option<String> = None;

        // 1. Extract place name / search query from URL paths and query parameters
        let re_place = Regex::new(r"/maps/place/([^/@?]+)").unwrap();
        if let Some(caps) = re_place.captures(&final_url) {
            if let Some(m) = caps.get(1) {
                let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
                let clean_name = unencoded.replace('+', " ").trim().to_string();
                if !clean_name.is_empty() && !clean_name.starts_with("data=") {
                    place_query_candidate = Some(clean_name.clone());
                    title = Some(clean_name);
                }
            }
        }

        if place_query_candidate.is_none() {
            let re_search = Regex::new(r"/maps/search/([^/@?]+)").unwrap();
            if let Some(caps) = re_search.captures(&final_url).or_else(|| re_search.captures(url)) {
                if let Some(m) = caps.get(1) {
                    let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
                    let clean_name = unencoded.replace('+', " ").trim().to_string();
                    if !clean_name.is_empty() && !clean_name.starts_with("data=") {
                        place_query_candidate = Some(clean_name.clone());
                        title = Some(clean_name);
                    }
                }
            }
        }

        if place_query_candidate.is_none() {
            let re_q = Regex::new(r"[?&](?:q|query|destination|daddr)=([^&]+)").unwrap();
            if let Some(caps) = re_q.captures(&final_url).or_else(|| re_q.captures(url)) {
                if let Some(m) = caps.get(1) {
                    let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
                    let clean_name = unencoded.replace('+', " ").trim().to_string();
                    // Ensure it's not purely coordinate numbers
                    if !clean_name.is_empty() && clean_name.chars().any(|c| c.is_alphabetic()) {
                        place_query_candidate = Some(clean_name.clone());
                        title = Some(clean_name);
                    }
                }
            }
        }

        // 2. Parse HTML Metadata (og:title, <title>, og:description, og:image, schema.org)
        if !html_text.is_empty() {
            let document = Html::parse_document(&html_text);

            let raw_meta_title = extract_meta_content(&document, "meta[property='og:title']")
                .or_else(|| extract_tag_text(&document, "title"));

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

            let og_desc = extract_meta_content(&document, "meta[property='og:description']")
                .or_else(|| extract_meta_content(&document, "meta[name='description']"));
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

            let og_img = extract_meta_content(&document, "meta[property='og:image']");
            if image_url.is_none() {
                // Avoid using staticmap as image if it's the generic Google logo
                if let Some(ref img) = og_img {
                    if !img.contains("maps_logo") {
                        image_url = Some(img.clone());
                    }
                }
            }

            // Check schema.org geo coordinates if present in HTML
            if let (Some(la_str), Some(lo_str)) = (
                extract_meta_content(&document, "meta[itemprop='latitude']"),
                extract_meta_content(&document, "meta[itemprop='longitude']"),
            ) {
                if let (Ok(la), Ok(lo)) = (la_str.parse::<f64>(), lo_str.parse::<f64>()) {
                    lat = Some(la);
                    lon = Some(lo);
                }
            }
        }

        // 3. Extract exact pin coordinates from URL (!3d... !4d...)
        let re_3d4d = Regex::new(r"!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)").unwrap();
        if let Some(caps) = re_3d4d.captures(&final_url).or_else(|| re_3d4d.captures(url)) {
            lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
            lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
        }

        // 4. Extract query coordinates if present in URL (q=lat,lon)
        if lat.is_none() {
            let re_q_coords = Regex::new(r"[?&](?:q|ll|query)=(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
            if let Some(caps) = re_q_coords.captures(&final_url).or_else(|| re_q_coords.captures(url)) {
                lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
            }
        }

        // 5. Use OpenStreetMap Nominatim geocoder to resolve accurate place query details
        if let Some(ref query) = place_query_candidate {
            if query != "Google Maps" && !query.is_empty() {
                if let Ok(Some(geo)) = self.geocoder.geocode(query).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    address = Some(geo.display_name.clone());
                    if title.is_none() || title.as_deref() == Some("Google Maps") {
                        let short_title = geo.display_name.split(',').next().unwrap_or(query).trim().to_string();
                        title = Some(short_title);
                    }
                }
            }
        }

        // 6. If coordinates still missing, check camera viewport coords (@lat,lon)
        if lat.is_none() || lon.is_none() {
            let re_at = Regex::new(r"@(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
            if let Some(caps) = re_at.captures(&final_url).or_else(|| re_at.captures(url)) {
                lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
            }
        }

        // 7. If coordinates are found but address is missing, reverse geocode
        if let (Some(la), Some(lo)) = (lat, lon) {
            if address.is_none() || address.as_deref() == Some("Google Maps") {
                if let Ok(Some(addr)) = self.geocoder.reverse_geocode(la, lo).await {
                    if title.is_none() || title.as_deref() == Some("Google Maps") {
                        let short_title = addr.split(',').next().unwrap_or("Saved Place").trim().to_string();
                        title = Some(short_title);
                    }
                    address = Some(addr);
                }
            }
        } else {
            // Fallback geocoding if coordinates are still missing
            let fallback_search = title.as_deref().or(address.as_deref());
            if let Some(term) = fallback_search {
                if term != "Google Maps" && !term.is_empty() {
                    if let Ok(Some(geo)) = self.geocoder.geocode(term).await {
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
                resolved_title = a.split(',').next().unwrap_or("Saved Place").trim().to_string();
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
            source_url: url.to_string(),
            source_type: "google_maps".to_string(),
        })
    }

    async fn scrape_apple_maps(&self, url: &str) -> Result<ScrapedMetadata, String> {
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
                let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
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
                let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
                let clean = unencoded.replace('+', " ").trim().to_string();
                if !clean.is_empty() {
                    address = Some(clean);
                }
            }
        }

        if lat.is_none() || lon.is_none() {
            let search_term = address.as_ref().or(title.as_ref());
            if let Some(query) = search_term {
                if let Ok(Some(geo)) = self.geocoder.geocode(query).await {
                    lat = Some(geo.latitude);
                    lon = Some(geo.longitude);
                    if title.is_none() {
                        title = Some(geo.display_name.split(',').next().unwrap_or("Apple Maps Place").trim().to_string());
                    }
                    address = Some(geo.display_name);
                }
            }
        } else if address.is_none() {
            if let (Some(la), Some(lo)) = (lat, lon) {
                if let Ok(Some(addr)) = self.geocoder.reverse_geocode(la, lo).await {
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
            source_url: url.to_string(),
            source_type: "apple_maps".to_string(),
        })
    }

    async fn scrape_instagram(&self, url: &str) -> Result<ScrapedMetadata, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Instagram link: {}", e))?;

        let html_text = response.text().await.unwrap_or_default();

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
            if let Ok(Some(geo)) = self.geocoder.geocode(&clean_title).await {
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
            source_url: url.to_string(),
            source_type: "instagram".to_string(),
        })
    }

    async fn scrape_generic_page(&self, url: &str) -> Result<ScrapedMetadata, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch webpage: {}", e))?;

        let html_text = response.text().await.unwrap_or_default();

        let (title, description, image_url, mut lat, mut lon) = {
            let document = Html::parse_document(&html_text);

            let t = extract_meta_content(&document, "meta[property='og:title']")
                .or_else(|| extract_tag_text(&document, "title"))
                .map(|s| clean_page_title(&s))
                .unwrap_or_else(|| "Saved Location".to_string());

            let desc = extract_meta_content(&document, "meta[property='og:description']")
                .or_else(|| extract_meta_content(&document, "meta[name='description']"));

            let img = extract_meta_content(&document, "meta[property='og:image']");

            let mut parsed_lat: Option<f64> = None;
            let mut parsed_lon: Option<f64> = None;

            if let Some(pos) = extract_meta_content(&document, "meta[name='geo.position']") {
                let parts: Vec<&str> = pos.split(';').collect();
                if parts.len() == 2 {
                    parsed_lat = parts[0].trim().parse().ok();
                    parsed_lon = parts[1].trim().parse().ok();
                }
            }

            if parsed_lat.is_none() {
                if let Some(icbm) = extract_meta_content(&document, "meta[name='ICBM']") {
                    let parts: Vec<&str> = icbm.split(',').collect();
                    if parts.len() == 2 {
                        parsed_lat = parts[0].trim().parse().ok();
                        parsed_lon = parts[1].trim().parse().ok();
                    }
                }
            }

            if parsed_lat.is_none() {
                let og_lat = extract_meta_content(&document, "meta[property='place:location:latitude']");
                let og_lon = extract_meta_content(&document, "meta[property='place:location:longitude']");
                if let (Some(la_str), Some(lo_str)) = (og_lat, og_lon) {
                    parsed_lat = la_str.trim().parse().ok();
                    parsed_lon = lo_str.trim().parse().ok();
                }
            }

            (t, desc, img, parsed_lat, parsed_lon)
        };

        let mut address: Option<String> = None;

        if lat.is_none() || lon.is_none() {
            if let Ok(Some(geo)) = self.geocoder.geocode(&title).await {
                lat = Some(geo.latitude);
                lon = Some(geo.longitude);
                address = Some(geo.display_name);
            }
        } else if let (Some(la), Some(lo)) = (lat, lon) {
            if let Ok(Some(addr)) = self.geocoder.reverse_geocode(la, lo).await {
                address = Some(addr);
            }
        }

        Ok(ScrapedMetadata {
            title,
            description,
            latitude: lat,
            longitude: lon,
            address,
            image_url,
            source_url: url.to_string(),
            source_type: "article".to_string(),
        })
    }
}

fn extract_meta_content(document: &Html, selector_str: &str) -> Option<String> {
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

fn extract_tag_text(document: &Html, tag_name: &str) -> Option<String> {
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

fn parse_google_maps_title_and_address(raw_title: &str) -> (Option<String>, Option<String>) {
    let mut text = raw_title.trim();
    for suffix in &[" - Google Maps", " · Google Maps", " - Google Search", " - Google"] {
        if let Some(pos) = text.rfind(suffix) {
            text = text[..pos].trim();
        }
    }
    if text.is_empty() || text.eq_ignore_ascii_case("Google Maps") {
        return (None, None);
    }

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

    (Some(text.to_string()), None)
}

fn clean_page_title(title: &str) -> String {
    let mut cleaned = title.trim().to_string();
    let suffixes = [
        " • Instagram photos and videos",
        " | Instagram",
        " on Instagram",
        " - Wikipedia",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_google_maps_title_and_address() {
        let (title, addr) = parse_google_maps_title_and_address("Chicken Boy · 5558 N Figueroa St, Los Angeles, CA 90042 · Google Maps");
        assert_eq!(title, Some("Chicken Boy".to_string()));
        assert_eq!(addr, Some("5558 N Figueroa St, Los Angeles, CA 90042".to_string()));

        let (title2, addr2) = parse_google_maps_title_and_address("Space Needle - Google Maps");
        assert_eq!(title2, Some("Space Needle".to_string()));
        assert_eq!(addr2, None);

        let (title3, addr3) = parse_google_maps_title_and_address("Google Maps");
        assert_eq!(title3, None);
        assert_eq!(addr3, None);
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
}
