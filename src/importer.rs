use crate::models::ImportItem;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Compiled-once regex constants (LazyLock — stable since Rust 1.80)
// ---------------------------------------------------------------------------

/// KML `<Placemark>` block extractor.
static RE_KML_PLACEMARK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<Placemark\b[^>]*>(.*?)</Placemark>").unwrap());
/// KML `<name>` extractor (within a Placemark block).
static RE_KML_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<name>(.*?)</name>").unwrap());
/// KML `<description>` extractor.
static RE_KML_DESC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<description>(.*?)</description>").unwrap());
/// KML `<coordinates>` extractor.
static RE_KML_COORDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<coordinates>\s*([^\s<]+)").unwrap());
/// KML `<address>` extractor.
static RE_KML_ADDRESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<address>(.*?)</address>").unwrap());

/// Google Maps `/@lat,lon` pattern.
static RE_URL_AT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/@(-?\d+\.\d+),(-?\d+\.\d+)").unwrap());
/// Google Maps `!3dlat!4dlon` embed pattern.
static RE_URL_EMBED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)").unwrap());
/// Generic `?q=lat,lon` / `?ll=lat,lon` query-string pattern.
static RE_URL_QUERY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[?&](?:q|ll)=(-?\d+\.\d+),(-?\d+\.\d+)").unwrap());

/// Detect and parse import data in various formats:
/// - Google Takeout JSON ("Saved Places.json")
/// - Standard GeoJSON
/// - Google Takeout CSV / Starred CSV
/// - Google My Maps / Earth KML
pub fn parse_import_data(raw: &str, format_hint: Option<&str>) -> Result<Vec<ImportItem>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Import content is empty".to_string());
    }

    let hint = format_hint.unwrap_or("auto").to_lowercase();

    match hint.as_str() {
        "takeout_json" | "geojson" | "json" => parse_json_data(trimmed),
        "takeout_csv" | "csv" => parse_csv_data(trimmed),
        "kml" | "xml" => parse_kml_data(trimmed),
        _ => {
            // Auto-detect format
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                parse_json_data(trimmed)
            } else if trimmed.starts_with("<?xml")
                || trimmed.contains("<kml")
                || trimmed.contains("<Placemark")
            {
                parse_kml_data(trimmed)
            } else {
                parse_csv_data(trimmed)
            }
        }
    }
}

/// Parse JSON format (both GeoJSON and Google Takeout Saved Places JSON)
pub fn parse_json_data(raw: &str) -> Result<Vec<ImportItem>, String> {
    let parsed: Value =
        serde_json::from_str(raw).map_err(|e| format!("Invalid JSON format: {}", e))?;
    let mut items = Vec::new();

    if let Some(features) = parsed.get("features").and_then(|f| f.as_array()) {
        // Standard GeoJSON FeatureCollection or Google Takeout GeoJSON
        for feat in features {
            if let Some(item) = parse_geojson_feature(feat) {
                items.push(item);
            }
        }
    } else if let Some(arr) = parsed.as_array() {
        // Direct array of items or features
        for val in arr {
            if val.get("type").and_then(|t| t.as_str()) == Some("Feature") {
                if let Some(item) = parse_geojson_feature(val) {
                    items.push(item);
                }
            } else {
                if let Some(item) = parse_generic_json_object(val) {
                    items.push(item);
                }
            }
        }
    } else if parsed.get("type").and_then(|t| t.as_str()) == Some("Feature") {
        if let Some(item) = parse_geojson_feature(&parsed) {
            items.push(item);
        }
    } else {
        if let Some(item) = parse_generic_json_object(&parsed) {
            items.push(item);
        }
    }

    if items.is_empty() {
        return Err("No valid places found in JSON data".to_string());
    }

    Ok(items)
}

fn parse_geojson_feature(feat: &Value) -> Option<ImportItem> {
    let props = feat.get("properties");

    // Title extraction
    let title = props
        .and_then(|p| {
            p.get("Title")
                .or_else(|| p.get("title"))
                .or_else(|| p.get("name"))
                .or_else(|| p.get("Name"))
                .or_else(|| p.get("Location").and_then(|l| l.get("Business Name")))
                .or_else(|| p.get("Location").and_then(|l| l.get("name")))
        })
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut lat = None;
    let mut lon = None;

    // 1. Check geometry coordinates [lon, lat]
    if let Some(geom) = feat.get("geometry") {
        if let Some(coords) = geom.get("coordinates").and_then(|c| c.as_array()) {
            if coords.len() >= 2 {
                if let (Some(x), Some(y)) = (coords[0].as_f64(), coords[1].as_f64()) {
                    if is_valid_lat_lon(y, x) {
                        lon = Some(x);
                        lat = Some(y);
                    }
                }
            }
        }
    }

    // 2. Check Location.Geo Coordinates (Google Takeout JSON structure)
    if (lat.is_none() || lon.is_none()) && props.is_some() {
        if let Some(geo) = props
            .and_then(|p| p.get("Location"))
            .and_then(|l| l.get("Geo Coordinates"))
        {
            if let (Some(la), Some(lo)) = (
                geo.get("Latitude")
                    .or_else(|| geo.get("latitude"))
                    .and_then(|v| v.as_f64()),
                geo.get("Longitude")
                    .or_else(|| geo.get("longitude"))
                    .and_then(|v| v.as_f64()),
            ) {
                if is_valid_lat_lon(la, lo) {
                    lat = Some(la);
                    lon = Some(lo);
                }
            }
        }
    }

    // Address
    let address = props
        .and_then(|p| {
            p.get("address")
                .or_else(|| p.get("Address"))
                .or_else(|| p.get("Location").and_then(|l| l.get("Address")))
                .or_else(|| p.get("Location").and_then(|l| l.get("Formatted Address")))
        })
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    // URL / Google Maps Link
    let url = props
        .and_then(|p| {
            p.get("URL")
                .or_else(|| p.get("url"))
                .or_else(|| p.get("Google Maps URL"))
                .or_else(|| p.get("source_url"))
        })
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    // Fallback: coordinate extraction from URL if lat/lon missing
    if lat.is_none() || lon.is_none() {
        if let Some(ref u) = url {
            if let Some((u_lat, u_lon)) = extract_coordinates_from_url(u) {
                lat = Some(u_lat);
                lon = Some(u_lon);
            }
        }
    }

    // Notes / Description
    let notes = props
        .and_then(|p| {
            p.get("Comment")
                .or_else(|| p.get("comment"))
                .or_else(|| p.get("notes"))
                .or_else(|| p.get("Note"))
                .or_else(|| p.get("description"))
        })
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    // Category
    let category = props
        .and_then(|p| p.get("category").or_else(|| p.get("Category")))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    // Emoji
    let emoji = props
        .and_then(|p| p.get("emoji").or_else(|| p.get("icon")))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    // Tags
    let tags = props
        .and_then(|p| p.get("tags").or_else(|| p.get("Tags")))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.trim().to_string())
            } else if let Some(arr) = v.as_array() {
                let list: Vec<String> = arr
                    .iter()
                    .filter_map(|t| t.as_str().map(|s| s.trim().to_string()))
                    .collect();
                Some(list.join(","))
            } else {
                None
            }
        });

    // Priority
    let priority = props
        .and_then(|p| {
            p.get("priority")
                .or_else(|| p.get("Priority"))
                .or_else(|| p.get("starred"))
        })
        .and_then(|v| v.as_bool());

    // Opening Hours
    let opening_hours = props
        .and_then(|p| {
            p.get("opening_hours")
                .or_else(|| p.get("Opening Hours"))
                .or_else(|| p.get("hours"))
        })
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    // Image URL / Photo
    let image_url = props
        .and_then(|p| {
            p.get("image_url")
                .or_else(|| p.get("image"))
                .or_else(|| p.get("photo_url"))
                .or_else(|| p.get("photo"))
                .or_else(|| p.get("Photo"))
        })
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    // Visited
    let visited = props
        .and_then(|p| p.get("visited").or_else(|| p.get("Visited")))
        .and_then(|v| v.as_bool());

    let final_title = title
        .or_else(|| address.clone())
        .unwrap_or_else(|| "Saved Place".to_string());

    Some(ImportItem {
        title: final_title,
        description: None,
        latitude: lat,
        longitude: lon,
        category,
        emoji,
        tags,
        priority,
        day_group: None,
        opening_hours,
        source_url: url,
        image_url,
        address,
        notes,
        visited,
    })
}

fn parse_generic_json_object(obj: &Value) -> Option<ImportItem> {
    let title = obj
        .get("title")
        .or_else(|| obj.get("Title"))
        .or_else(|| obj.get("name"))
        .or_else(|| obj.get("Name"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let raw_lat = obj
        .get("latitude")
        .or_else(|| obj.get("lat"))
        .and_then(|v| v.as_f64());
    let raw_lon = obj
        .get("longitude")
        .or_else(|| obj.get("lon"))
        .or_else(|| obj.get("lng"))
        .and_then(|v| v.as_f64());
    let (lat, lon) = match (raw_lat, raw_lon) {
        (Some(la), Some(lo)) if is_valid_lat_lon(la, lo) => (Some(la), Some(lo)),
        _ => (None, None),
    };

    let address = obj
        .get("address")
        .or_else(|| obj.get("Address"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let url = obj
        .get("url")
        .or_else(|| obj.get("URL"))
        .or_else(|| obj.get("source_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let notes = obj
        .get("notes")
        .or_else(|| obj.get("comment"))
        .or_else(|| obj.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let category = obj
        .get("category")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let emoji = obj
        .get("emoji")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tags = obj
        .get("tags")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let priority = obj.get("priority").and_then(|v| v.as_bool());
    let opening_hours = obj
        .get("opening_hours")
        .or_else(|| obj.get("hours"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let image_url = obj
        .get("image_url")
        .or_else(|| obj.get("image"))
        .or_else(|| obj.get("photo_url"))
        .or_else(|| obj.get("photo"))
        .or_else(|| obj.get("Photo"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let visited = obj.get("visited").and_then(|v| v.as_bool());

    let final_title = title
        .or_else(|| address.clone())
        .unwrap_or_else(|| "Saved Place".to_string());

    Some(ImportItem {
        title: final_title,
        description: None,
        latitude: lat,
        longitude: lon,
        category,
        emoji,
        tags,
        priority,
        day_group: None,
        opening_hours,
        source_url: url,
        image_url,
        address,
        notes,
        visited,
    })
}

/// Parse CSV format (Google Takeout CSV, Starred places CSV, custom spreadsheets)
pub fn parse_csv_data(raw: &str) -> Result<Vec<ImportItem>, String> {
    let mut lines = raw.lines();
    let header_line = match lines.next() {
        Some(h) => h,
        None => return Err("CSV is empty".to_string()),
    };

    let delimiter = if header_line.contains('\t') {
        '\t'
    } else {
        ','
    };
    let headers: Vec<String> = parse_csv_row(header_line, delimiter)
        .into_iter()
        .map(|h| h.trim().to_lowercase())
        .collect();

    let mut title_idx = None;
    let mut lat_idx = None;
    let mut lon_idx = None;
    let mut url_idx = None;
    let mut note_idx = None;
    let mut addr_idx = None;
    let mut cat_idx = None;
    let mut visited_idx = None;
    let mut hours_idx = None;
    let mut img_idx = None;

    for (i, h) in headers.iter().enumerate() {
        if h == "title" || h == "name" || h == "place name" || h == "business name" {
            title_idx = Some(i);
        } else if h == "latitude" || h == "lat" {
            lat_idx = Some(i);
        } else if h == "longitude" || h == "lon" || h == "lng" {
            lon_idx = Some(i);
        } else if h == "url" || h == "google maps url" || h == "link" || h == "source_url" {
            url_idx = Some(i);
        } else if h == "note"
            || h == "notes"
            || h == "comment"
            || h == "comments"
            || h == "description"
        {
            note_idx = Some(i);
        } else if h == "address" || h == "formatted address" || h == "location" {
            addr_idx = Some(i);
        } else if h == "category" || h == "type" {
            cat_idx = Some(i);
        } else if h == "visited" || h == "status" {
            visited_idx = Some(i);
        } else if h == "opening_hours" || h == "hours" || h == "opening hours" {
            hours_idx = Some(i);
        } else if h == "image"
            || h == "image_url"
            || h == "image url"
            || h == "photo"
            || h == "photo_url"
            || h == "photo url"
        {
            img_idx = Some(i);
        }
    }

    let mut items = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let cols = parse_csv_row(trimmed, delimiter);
        if cols.is_empty() {
            continue;
        }

        let title = title_idx
            .and_then(|idx| cols.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let addr = addr_idx
            .and_then(|idx| cols.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let url = url_idx
            .and_then(|idx| cols.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let notes = note_idx
            .and_then(|idx| cols.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let cat = cat_idx
            .and_then(|idx| cols.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let opening_hours = hours_idx
            .and_then(|idx| cols.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let image_url = img_idx
            .and_then(|idx| cols.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let raw_lat = lat_idx
            .and_then(|idx| cols.get(idx))
            .and_then(|s| s.trim().parse::<f64>().ok());
        let raw_lon = lon_idx
            .and_then(|idx| cols.get(idx))
            .and_then(|s| s.trim().parse::<f64>().ok());
        let (mut lat, mut lon) = match (raw_lat, raw_lon) {
            (Some(la), Some(lo)) if is_valid_lat_lon(la, lo) => (Some(la), Some(lo)),
            _ => (None, None),
        };

        // Extract coords from URL if not in lat/lon columns
        if lat.is_none() || lon.is_none() {
            if let Some(ref u) = url {
                if let Some((u_lat, u_lon)) = extract_coordinates_from_url(u) {
                    lat = Some(u_lat);
                    lon = Some(u_lon);
                }
            }
        }

        let visited = visited_idx.and_then(|idx| cols.get(idx)).map(|s| {
            let lower = s.trim().to_lowercase();
            lower == "true" || lower == "1" || lower == "yes" || lower == "visited"
        });

        let final_title = title
            .or_else(|| addr.clone())
            .unwrap_or_else(|| "Saved Place".to_string());

        items.push(ImportItem {
            title: final_title,
            description: None,
            latitude: lat,
            longitude: lon,
            category: cat,
            emoji: None,
            tags: None,
            priority: None,
            day_group: None,
            opening_hours,
            source_url: url,
            image_url,
            address: addr,
            notes,
            visited,
        });
    }

    if items.is_empty() {
        return Err("No rows could be parsed from CSV".to_string());
    }

    Ok(items)
}

fn parse_csv_row(line: &str, delimiter: char) -> Vec<String> {
    let mut cols = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                current.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if c == delimiter && !in_quotes {
            cols.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    cols.push(current.trim().to_string());
    cols
}

/// Parse KML / XML format (Google My Maps, Google Earth, Maps.me)
pub fn parse_kml_data(raw: &str) -> Result<Vec<ImportItem>, String> {
    let mut items = Vec::new();

    for cap in RE_KML_PLACEMARK.captures_iter(raw) {
        let block = &cap[1];

        let name = RE_KML_NAME
            .captures(block)
            .map(|c| clean_xml_text(&c[1]))
            .filter(|s| !s.is_empty());
        let desc = RE_KML_DESC
            .captures(block)
            .map(|c| clean_xml_text(&c[1]))
            .filter(|s| !s.is_empty());
        let addr = RE_KML_ADDRESS
            .captures(block)
            .map(|c| clean_xml_text(&c[1]))
            .filter(|s| !s.is_empty());

        let mut lat = None;
        let mut lon = None;

        if let Some(c_cap) = RE_KML_COORDS.captures(block) {
            let coord_str = &c_cap[1];
            let parts: Vec<&str> = coord_str.split(',').collect();
            if parts.len() >= 2 {
                if let (Ok(x), Ok(y)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                ) {
                    if is_valid_lat_lon(y, x) {
                        lon = Some(x);
                        lat = Some(y);
                    }
                }
            }
        }

        let title = name
            .or_else(|| addr.clone())
            .unwrap_or_else(|| "Saved Place".to_string());

        items.push(ImportItem {
            title,
            description: None,
            latitude: lat,
            longitude: lon,
            category: None,
            emoji: None,
            tags: None,
            priority: None,
            day_group: None,
            opening_hours: None,
            source_url: None,
            image_url: None,
            address: addr,
            notes: desc,
            visited: None,
        });
    }

    if items.is_empty() {
        return Err("No <Placemark> elements found in KML data".to_string());
    }

    Ok(items)
}

fn clean_xml_text(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.starts_with("<![CDATA[") && s.ends_with("]]>") {
        s = s[9..s.len() - 3].trim().to_string();
    }
    s = s
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'");
    s.trim().to_string()
}

/// Extract GPS coordinates from Google Maps and common map URLs
pub fn extract_coordinates_from_url(url: &str) -> Option<(f64, f64)> {
    // 1. Match /@lat,lon,zoom or /@lat,lon
    if let Some(caps) = RE_URL_AT.captures(url) {
        if let (Ok(lat), Ok(lon)) = (caps[1].parse::<f64>(), caps[2].parse::<f64>()) {
            if is_valid_lat_lon(lat, lon) {
                return Some((lat, lon));
            }
        }
    }

    // 2. Match !3dlat!4dlon (Google Maps place URLs)
    if let Some(caps) = RE_URL_EMBED.captures(url) {
        if let (Ok(lat), Ok(lon)) = (caps[1].parse::<f64>(), caps[2].parse::<f64>()) {
            if is_valid_lat_lon(lat, lon) {
                return Some((lat, lon));
            }
        }
    }

    // 3. Match ?q=lat,lon or &q=lat,lon or ?ll=lat,lon
    if let Some(caps) = RE_URL_QUERY.captures(url) {
        if let (Ok(lat), Ok(lon)) = (caps[1].parse::<f64>(), caps[2].parse::<f64>()) {
            if is_valid_lat_lon(lat, lon) {
                return Some((lat, lon));
            }
        }
    }

    None
}

fn is_valid_lat_lon(lat: f64, lon: f64) -> bool {
    lat.is_finite()
        && lon.is_finite()
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_google_takeout_json() {
        let json_data = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [139.7004, 35.6595]
                    },
                    "properties": {
                        "Title": "Shibuya Crossing",
                        "Location": {
                            "Address": "Shibuya City, Tokyo, Japan",
                            "Business Name": "Shibuya Crossing"
                        },
                        "Comment": "Scramble intersection",
                        "URL": "https://maps.google.com/?cid=123"
                    }
                }
            ]
        }"#;

        let items = parse_import_data(json_data, Some("takeout_json")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Shibuya Crossing");
        assert_eq!(items[0].latitude, Some(35.6595));
        assert_eq!(items[0].longitude, Some(139.7004));
        assert_eq!(
            items[0].address,
            Some("Shibuya City, Tokyo, Japan".to_string())
        );
        assert_eq!(items[0].notes, Some("Scramble intersection".to_string()));
    }

    #[test]
    fn test_parse_csv_data() {
        let csv_data = "Title,Note,URL,Address,Latitude,Longitude\n\
                        Fuglen Tokyo,Great coffee,https://example.com,Tomigaya,35.6675,139.6923\n\
                        Sensoji,,\"https://maps.google.com/@35.7148,139.7967\",Asakusa,,";

        let items = parse_import_data(csv_data, Some("csv")).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Fuglen Tokyo");
        assert_eq!(items[0].latitude, Some(35.6675));
        assert_eq!(items[0].longitude, Some(139.6923));
        assert_eq!(items[1].title, "Sensoji");
        assert_eq!(items[1].latitude, Some(35.7148));
        assert_eq!(items[1].longitude, Some(139.7967));
    }

    #[test]
    fn test_parse_kml_data() {
        let kml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
        <kml xmlns="http://www.opengis.net/kml/2.2">
          <Document>
            <Placemark>
              <name>Eiffel Tower</name>
              <description><![CDATA[Famous iron tower]]></description>
              <Point>
                <coordinates>2.2945,48.8584,0</coordinates>
              </Point>
            </Placemark>
          </Document>
        </kml>"#;

        let items = parse_import_data(kml_data, Some("kml")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Eiffel Tower");
        assert_eq!(items[0].latitude, Some(48.8584));
        assert_eq!(items[0].longitude, Some(2.2945));
        assert_eq!(items[0].notes, Some("Famous iron tower".to_string()));
    }

    #[test]
    fn test_extract_coordinates_from_url() {
        let url1 = "https://www.google.com/maps/@34.0522,-118.2437,14z";
        assert_eq!(
            extract_coordinates_from_url(url1),
            Some((34.0522, -118.2437))
        );

        let url2 = "https://www.google.com/maps/place/Data/@35.6586,139.7454/data=!3m1!4b1!4m6!3m5!1s0x0:0x0!8m2!3d35.6586!4d139.7454";
        assert_eq!(
            extract_coordinates_from_url(url2),
            Some((35.6586, 139.7454))
        );

        let url3 = "https://maps.apple.com/?ll=51.5074,-0.1278&q=London";
        assert_eq!(extract_coordinates_from_url(url3), Some((51.5074, -0.1278)));
    }

    #[test]
    fn test_parse_invalid_out_of_bounds_coordinates() {
        // GeoJSON with invalid latitude 95.0 (> 90)
        let invalid_geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": { "type": "Point", "coordinates": [10.0, 95.0] },
                    "properties": { "Title": "Invalid North Pole" }
                }
            ]
        }"#;
        let items = parse_import_data(invalid_geojson, Some("geojson")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].latitude, None);
        assert_eq!(items[0].longitude, None);

        // CSV with out-of-bounds longitude 200.0 (> 180)
        let invalid_csv = "Title,Latitude,Longitude\nBad Coords Place,45.0,200.0\n";
        let items_csv = parse_import_data(invalid_csv, Some("csv")).unwrap();
        assert_eq!(items_csv.len(), 1);
        assert_eq!(items_csv[0].latitude, None);
        assert_eq!(items_csv[0].longitude, None);

        // KML with out-of-bounds latitude
        let invalid_kml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <kml xmlns="http://www.opengis.net/kml/2.2">
          <Document>
            <Placemark>
              <name>Bad KML Place</name>
              <Point><coordinates>500.0,-120.0,0</coordinates></Point>
            </Placemark>
          </Document>
        </kml>"#;
        let items_kml = parse_import_data(invalid_kml, Some("kml")).unwrap();
        assert_eq!(items_kml.len(), 1);
        assert_eq!(items_kml[0].latitude, None);
        assert_eq!(items_kml[0].longitude, None);
    }
}
