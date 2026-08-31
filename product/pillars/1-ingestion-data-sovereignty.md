# 📥 Pillar 1: Frictionless Ingestion & Data Sovereignty

> **Strategic Goal**: Make saving and importing places into bList 10x faster and easier than any competitor, while guaranteeing zero lock-in and complete data portability.

---

## 🎯 Key Initiatives & Features

### 1.1 Universal Import Wizard (Google Takeout / CSV / KML / GeoJSON / GPX)
- **Problem**: Users with years of saved places in Google Maps, Apple Maps, or Wanderlog hesitate to switch because manual re-entry is impossible.
- **Solution**:
  - Drag-and-drop file upload modal supporting:
    - Google Takeout `Saved Places.json` & `Saved Places.csv`.
    - Standard GIS `KML`, `KMZ`, `GeoJSON`, `GPX`.
  - Background asynchronous geocoding worker pool resolving missing coordinates in batch.
  - Sub-second batch insertion into SQLite with transaction batching.

### 1.2 Desktop Browser Extension (Chrome / Safari / Firefox)
- **Problem**: On desktop, copying URLs from Eater, TripAdvisor, or Reddit is a minor friction point.
- **Solution**:
  - Lightweight Manifest V3 WebExtension.
  - Parses OpenGraph / Schema.org metadata and sends it to the user's active bList collection with 1 click.

### 1.3 Enhanced Share Sheet Parser
- **Problem**: Native apps often share complex multi-line text containing URLs wrapped with descriptions.
- **Solution**:
  - Resilient URL extraction regex and fallback logic in `static/helpers.js` and `src/scraper.rs`.

---

## 🛠️ Technical Architecture & Endpoints

```
POST /api/import/file
Content-Type: multipart/form-data
Body: file=[Google Takeout.json / places.kml], list_id=123
Response: { "imported": 45, "geocoded": 12, "errors": 0 }
```

- **Safety & Limits**: Max file size 10MB; batch transactions in chunks of 100 rows to prevent SQLite write lock timeouts.
