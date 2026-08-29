use crate::geocoder::Geocoder;
use crate::models::ScrapedMetadata;
use regex::Regex;
use scraper::{Html, Selector};
use std::time::Duration;

pub struct Scraper {
    client: reqwest::Client,
    geocoder: Geocoder,
}

impl Scraper {
    pub fn new() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
            ),
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(12))
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            geocoder: Geocoder::new(),
        }
    }

    pub async fn scrape_url(&self, raw_url: &str) -> Result<ScrapedMetadata, String> {
        let clean_url = raw_url.trim();
        if clean_url.is_empty() {
            return Err("URL cannot be empty".to_string());
        }

        let full_url = if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
            format!("https://{}", clean_url)
        } else {
            clean_url.to_string()
        };

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

        // Check for client-side / meta-refresh redirect (common in some Google share links)
        if html_text.contains("http-equiv=\"refresh\"") || html_text.contains("http-equiv='refresh'") {
            let re_refresh = Regex::new(r#"(?i)content=["'][0-9]+;\s*url=([^"']+)["']"#).unwrap();
            if let Some(caps) = re_refresh.captures(&html_text) {
                if let Some(m) = caps.get(1) {
                    let next_url = m.as_str().replace("&amp;", "&");
                    if let Ok(next_res) = self.client.get(&next_url).send().await {
                        final_url = next_res.url().to_string();
                        html_text = next_res.text().await.unwrap_or_default();
                    }
                }
            }
        }

        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;
        let mut title: Option<String> = None;
        let mut address: Option<String> = None;
        let mut image_url: Option<String> = None;

        // 1. Check coordinates in URL variants
        let re_at = Regex::new(r"@(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
        if let Some(caps) = re_at.captures(&final_url) {
            lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
            lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
        }

        if lat.is_none() {
            let re_3d4d = Regex::new(r"!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)").unwrap();
            if let Some(caps) = re_3d4d.captures(&final_url) {
                lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
            }
        }

        if lat.is_none() {
            let re_q = Regex::new(r"[?&](?:q|ll|center|query)=(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
            if let Some(caps) = re_q.captures(&final_url) {
                lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
            }
        }

        // 2. Extract Place Name from URL (/place/Place+Name/...)
        let re_place = Regex::new(r"/place/([^/@?]+)").unwrap();
        if let Some(caps) = re_place.captures(&final_url) {
            if let Some(m) = caps.get(1) {
                let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
                let clean_name = unencoded.replace('+', " ");
                if !clean_name.is_empty() && clean_name != "data=" {
                    title = Some(clean_name);
                }
            }
        }

        if title.is_none() {
            let re_query_text = Regex::new(r"[?&](?:q|query)=([^&]+)").unwrap();
            if let Some(caps) = re_query_text.captures(&final_url).or_else(|| re_query_text.captures(url)) {
                if let Some(m) = caps.get(1) {
                    let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
                    let clean = unencoded.replace('+', " ");
                    // If not just raw coords
                    if !clean.contains(',') || clean.chars().any(|c| c.is_alphabetic()) {
                        title = Some(clean);
                    }
                }
            }
        }

        // 3. Extract HTML Metadata
        if !html_text.is_empty() {
            let (og_title, og_img, og_desc, og_url, schema_geo) = {
                let document = Html::parse_document(&html_text);
                let t = extract_meta_content(&document, "meta[property='og:title']")
                    .or_else(|| extract_tag_text(&document, "title"))
                    .map(|s| clean_google_maps_title(&s));
                let img = extract_meta_content(&document, "meta[property='og:image']");
                let desc = extract_meta_content(&document, "meta[property='og:description']")
                    .or_else(|| extract_meta_content(&document, "meta[name='description']"));
                let u = extract_meta_content(&document, "meta[property='og:url']");

                let mut s_geo = None;
                if let (Some(la_str), Some(lo_str)) = (
                    extract_meta_content(&document, "meta[itemprop='latitude']"),
                    extract_meta_content(&document, "meta[itemprop='longitude']"),
                ) {
                    if let (Ok(la), Ok(lo)) = (la_str.parse::<f64>(), lo_str.parse::<f64>()) {
                        s_geo = Some((la, lo));
                    }
                }

                (t, img, desc, u, s_geo)
            };

            if title.is_none() || title.as_deref() == Some("Google Maps") {
                if let Some(t) = og_title {
                    if t != "Google Maps" {
                        title = Some(t);
                    }
                }
            }

            if image_url.is_none() {
                image_url = og_img.clone();
            }
            if address.is_none() {
                address = og_desc;
            }

            if lat.is_none() {
                if let Some((la, lo)) = schema_geo {
                    lat = Some(la);
                    lon = Some(lo);
                }
            }

            if lat.is_none() {
                if let Some(ref img) = og_img {
                    let re_staticmap = Regex::new(r"(?:center|markers|ll)=(-?\d+\.\d+)(?:%2C|,)(-?\d+\.\d+)").unwrap();
                    if let Some(caps) = re_staticmap.captures(img) {
                        lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                        lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
                    }
                }
            }

            if lat.is_none() {
                if let Some(ref u) = og_url {
                    if let Some(caps) = re_at.captures(u) {
                        lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                        lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
                    }
                }
            }

            if lat.is_none() {
                let re_html_coords = Regex::new(r"\[null,null,(-?\d+\.\d+),(-?\d+\.\d+)\]").unwrap();
                if let Some(caps) = re_html_coords.captures(&html_text) {
                    lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
                    lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
                }
            }
        }

        // 4. Fallback to OpenStreetMap Geocoder if coords or address still missing
        if lat.is_none() || lon.is_none() {
            let search_term = title.as_deref().or(address.as_deref());
            if let Some(query) = search_term {
                if query != "Google Maps" {
                    if let Ok(Some(geo)) = self.geocoder.geocode(query).await {
                        lat = Some(geo.latitude);
                        lon = Some(geo.longitude);
                        if address.is_none() {
                            address = Some(geo.display_name);
                        }
                    }
                }
            }
        } else if address.is_none() || address.as_deref() == Some("Google Maps") {
            if let (Some(la), Some(lo)) = (lat, lon) {
                if let Ok(Some(addr)) = self.geocoder.reverse_geocode(la, lo).await {
                    address = Some(addr);
                }
            }
        }

        let mut resolved_title = title.unwrap_or_else(|| "Saved Place".to_string());
        if resolved_title == "Google Maps" {
            if let Some(ref a) = address {
                resolved_title = a.split(',').next().unwrap_or("Saved Place").trim().to_string();
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

        let re_ll = Regex::new(r"[?&]ll=(-?\d+\.\d+),(-?\d+\.\d+)").unwrap();
        if let Some(caps) = re_ll.captures(url) {
            lat = caps.get(1).and_then(|m| m.as_str().parse().ok());
            lon = caps.get(2).and_then(|m| m.as_str().parse().ok());
        }

        let re_q = Regex::new(r"[?&]q=([^&]+)").unwrap();
        if let Some(caps) = re_q.captures(url) {
            if let Some(m) = caps.get(1) {
                let unencoded = urlencoding::decode(m.as_str()).unwrap_or_default().to_string();
                let clean = unencoded.replace('+', " ");
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
                let clean = unencoded.replace('+', " ");
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
                        title = Some(geo.display_name.clone());
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

        if let Ok(Some(geo)) = self.geocoder.geocode(&clean_title).await {
            lat = Some(geo.latitude);
            lon = Some(geo.longitude);
            address = Some(geo.display_name);
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

fn clean_google_maps_title(title: &str) -> String {
    let mut cleaned = title.trim().to_string();
    if let Some(pos) = cleaned.rfind(" - Google Maps") {
        cleaned = cleaned[..pos].trim().to_string();
    } else if let Some(pos) = cleaned.rfind(" · Google Maps") {
        cleaned = cleaned[..pos].trim().to_string();
    }
    if let Some(pos) = cleaned.find(" · ") {
        let first_part = cleaned[..pos].trim();
        if first_part.len() > 2 {
            cleaned = first_part.to_string();
        }
    }
    cleaned
}

fn clean_page_title(title: &str) -> String {
    let mut cleaned = title.trim().to_string();
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
