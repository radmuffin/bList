# 🤖 AGENTS.md — AI Agent Guidance for bList

Welcome, AI Coding Assistant! This document outlines the architecture, constraints, database schemas, and codebase patterns for **bList** (Visual Map Bucket List & Trip Planner) to help you contribute safely and efficiently.

---

## 🗺️ Project Overview & Architecture

bList is a lightweight, zero-AI, deterministic bucket list map web application written in Rust.

- **Language & Runtime**: Rust (2021 edition), Axum (v0.7), and Tokio async runtime.
- **Database**: SQLite via `rusqlite` (bundled feature). It is configured to run in **Write-Ahead Logging (WAL) mode** for optimal concurrent read/write access.
- **Frontend**: Single-Page Application (SPA) built using vanilla CSS, modern ES6+ JS, and **Leaflet.js** for map rendering. No build steps (webpack/vite) are needed for the frontend.
- **Scraper & Geocoder**: Deterministic parsing of scraped page content (`reqwest`, `scraper`) with OpenStreetMap (Nominatim API) geocoding backup.

---

## ⚠️ Critical Coding Guidelines & Gotchas

### 1. Axum Send Bounds & Scraper `Html` Struct
The `scraper::Html` struct parses document trees using non-atomic reference counts (`Cell<usize>`), meaning **it does not implement `Send`**. 
Axum routes require all futures to be `Send`. Therefore:
> [!IMPORTANT]
> Any HTML parsing inside `src/scraper.rs` **must** happen in a isolated synchronous block and be fully dropped before any `.await` statement occurs.
> **Correct Pattern:**
> ```rust
> let document = {
>     let doc = Html::parse_document(&body_text);
>     // extract all necessary metadata into Send-safe structs
>     extracted_meta
> }; // `doc` is dropped here
> // safe to make .await calls after this block
> ```

### 2. SQLite Database Connections & WAL
- Database queries use a shared connection wrapped in `Arc<Mutex<Connection>>` in Axum state.
- Keep transaction locks brief. Since WAL mode is active, reads do not block writes, but concurrent writes will serialize.

### 3. iOS/Android PWA Share Target
- Mobile browsers use `static/manifest.webmanifest` to register the **Web Share Target API**.
- When URLs or text are shared from native apps (Google Maps, Instagram) into bList, they target `/` (handled on page load by `handleIncomingShareTarget()` in `static/app.js`). Keep the parser inside `static/app.js` resilient against various shared text structures.

---

## 🗄️ Database Schemas

### 1. `lists`
Stores user-created trips and custom collections.
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `name` (TEXT NOT NULL)
- `icon` (TEXT NOT NULL DEFAULT '📍')
- `created_at` (TEXT NOT NULL)

### 2. `pins`
Stores saved places mapped to collections.
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `list_id` (INTEGER NOT NULL DEFAULT 1, REFERENCES lists(id))
- `title` (TEXT NOT NULL)
- `description` (TEXT)
- `latitude` (REAL NOT NULL)
- `longitude` (REAL NOT NULL)
- `category` (TEXT NOT NULL DEFAULT 'General')
- `source_url` (TEXT)
- `image_url` (TEXT)
- `address` (TEXT)
- `notes` (TEXT)
- `visited` (INTEGER NOT NULL DEFAULT 0)
- `created_at` (TEXT NOT NULL)

---

## 🚀 Key Commands

- **Build/Test local backend**: `cargo test && cargo build`
- **Run local server**: `cargo run` (accessible locally on port `3000`)
- **Deploy manually to staging**: `fly deploy --app blist-staging-radmuffin`
- **Deploy manually to production**: `fly deploy`
