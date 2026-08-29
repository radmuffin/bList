# 🗺️ bList

A lightweight, fast, deterministic Map Bucket List web application in Rust. Allows users to paste or share links (Google Maps places/businesses, Apple Maps, Instagram, travel blogs), automatically extracts place metadata and coordinates, stores pins in a local SQLite database, and renders them on an interactive Leaflet.js web map.

---

## ⚡ Features

- **Omni-Link Ingestion**:
  - **Google Maps Places & Businesses**: Handles shortened share links (`maps.app.goo.gl`), business place IDs, coordinate tokens (`!3d!4d`, `@lat,lng`), static map centers, and place names.
  - **Apple Maps**: Parses coordinates (`ll=lat,lng`), search queries, and addresses.
  - **Instagram**: Scrapes post metadata (`og:title`, `og:description`, `og:image`) and location cues.
  - **Articles & Web Pages**: Extracts OpenGraph & ICBM/geo metadata or falls back to OpenStreetMap geocoding.
- **Mobile-First UX**:
  - Collapsible panels with a floating Map / List drawer switcher on phones.
  - Full-screen interactive map with custom colored category pins.
  - Tap-to-place manual pins with auto reverse-geocoding.
- **Local SQLite Storage**: Fast, persistent storage with WAL mode (`rusqlite`).
- **GeoJSON & JSON Export**: One-click backups.
- **Docker & CI/CD**: Automated GitHub Actions CI and lightweight multi-stage Dockerfile.

---

## 🚀 Quick Start

```bash
cargo run
```
Open **`http://localhost:3000`** in your browser.
