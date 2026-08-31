# 📥 Technical Specification: Universal Importer Engine

> **Pillar**: [Pillar 1: Ingestion & Data Sovereignty](../pillars/1-ingestion-data-sovereignty.md)  
> **Status**: Ready for Implementation  
> **Scope**: Google Takeout (JSON/CSV), KML, KMZ, GeoJSON Batch Migration

---

## 1. Supported Ingest Formats

| Format | Source | Coordinate Strategy | Metadata Extracted |
|---|---|---|---|
| **Google Takeout JSON** (`Saved Places.json`) | Google Maps Takeout | `geometry.coordinates` `[lon, lat]` or `properties.Location.Geo Coordinates` | `Title`, `Location.Address`, `Location.Business Name`, `Comment`, `URL` |
| **Google Takeout CSV** (`Saved Places.csv`) | Google Maps CSV Takeout | Explicit `Latitude` & `Longitude` columns or URL coordinate regex extraction (`@lat,lon`, `q=lat,lon`) | `Title`, `Note`, `Comment`, `URL`, `Address` |
| **KML** (`.kml`) | Google My Maps / Google Earth | `<Placemark><Point><coordinates>lon,lat,alt</coordinates></Point></Placemark>` | `<name>`, `<description>`, `<address>`, `<ExtendedData>` |
| **GeoJSON** (`.geojson`, `.json`) | GIS / Export tools / bList exports | `geometry.coordinates` `[lon, lat]` | `properties.title`/`name`, `properties.category`, `properties.address`, `properties.notes`, `properties.visited` |

---

## 2. Deterministic Resolution Pipeline

```mermaid
flowchart TD
    A[Parsed Import Record] --> B{Explicit Lat/Lon?}
    B -- Yes --> C[Validate Bounds: lat [-90,90], lon [-180,180]]
    B -- No --> D{URL present?}
    D -- Yes --> E[Regex Parse @lat,lon / ?q=lat,lon]
    E -- Found --> C
    E -- Not Found --> F{Contains Plus Code?}
    D -- No --> F
    F -- Yes --> G[Decode Plus Code] --> C
    F -- No --> H{Address or Title available?}
    H -- Yes --> I[In-Memory Geocoder Cache Check]
    I -- Hit --> C
    I -- Miss --> J[Safe Geocoder Lookup]
    J -- Success --> C
    J -- Failed/Timeout --> K[Add to Warning Report]
    C --> L[Batch SQL Transaction]
```

---

## 3. Database & Axum Architecture

### Endpoint
- `POST /api/import`
- Consumes `multipart/form-data` or JSON body.
- Returns `ImportSummary` with count of imported places, skipped items, and warnings.

### Batch Storage Performance
- Implemented as a single `rusqlite::Transaction` with prepared statements in `src/db.rs`.
- Target execution time: `< 20ms` for 500 records.
