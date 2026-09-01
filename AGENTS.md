# 🤖 AGENTS.md — AI Agent Guidance for bList

Welcome, AI Coding Assistant! This document outlines the architecture, constraints, database schemas, security posture, and codebase patterns for **bList** (Visual Map Bucket List & Trip Planner) to help you contribute safely and efficiently.

---

## 🗺️ Project Overview & Architecture

bList is a lightweight, zero-AI, deterministic bucket list map web application written in Rust.

- **Language & Runtime**: Rust (2021 edition), Axum (v0.7), and Tokio async runtime.
- **Database**: SQLite via `rusqlite` (bundled feature) running in **Write-Ahead Logging (WAL) mode** with foreign key constraints enabled.
- **Frontend**: Single-Page Application (SPA) built using vanilla CSS, modern ES6+ JS, and **Leaflet.js** for map rendering. No build steps (webpack/vite) are needed for the frontend.
- **Scraper & Geocoder**: Deterministic parsing of scraped page content (`reqwest`, `scraper`) with OpenStreetMap (Nominatim API) geocoding backup.
- **Multi-Device Sync**: Anonymous cryptographic sync tokens (`X-User-Token` / `blist_device_token`) allow syncing across devices and sharing collaborative trip lists via random UUIDv4 share tokens.

---

## ⚠️ Critical Coding Guidelines & Gotchas

### 1. Axum Send Bounds & Scraper `Html` Struct
The `scraper::Html` struct parses document trees using non-atomic reference counts (`Cell<usize>`), meaning **it does not implement `Send`**. 
Axum routes require all futures to be `Send`. Therefore:
> [!IMPORTANT]
> Any HTML parsing inside `src/scraper.rs` **must** happen in an isolated synchronous block and be fully dropped before any `.await` statement occurs.
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
- Always use parameterized queries (`params![]` / `?`) to prevent SQL injection.

### 3. SSRF & Security Headers
- All external HTTP requests (ingest, metadata preview, geocoding) must be validated through `src/security.rs` (`validate_url_for_ssrf` / `build_safe_http_client`) to block private IPv4 (RFC 1918, 169.254.x), IPv6 (ULA, loopback), internal cloud metadata hostnames, and unsafe redirects.
- Security response headers (`X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN`, `Referrer-Policy: strict-origin-when-cross-origin`) are attached globally via Axum middleware in `src/main.rs`.

### 4. iOS/Android PWA Share Target & UI Specificity
- Mobile browsers use `static/manifest.webmanifest` to register the **Web Share Target API**.
- When URLs or text are shared from native apps (Google Maps, Instagram) into bList, they target `/` (handled on page load by `handleIncomingShareTarget()` in `static/app.js`). Keep the parser inside `static/app.js` resilient against various shared text structures.
- **UI Elements**: Never duplicate interactive DOM elements (like header buttons) between mobile and desktop unless strictly necessary. Ensure CSS selector specificity does not use `!important` on generic classes (like `.btn`) to avoid breaking `.desktop-only` / `.mobile-only` visibility utilities.
- All user-controlled text inserted into the DOM must pass through `Utils.escapeHtml()` and all URLs through `Utils.sanitizeUrl()`.

---

## 🗄️ Database Schemas

### 1. `lists`
Stores user-created trips and custom collections.
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `name` (TEXT NOT NULL)
- `icon` (TEXT NOT NULL DEFAULT '📍')
- `created_at` (TEXT NOT NULL)
- `owner_token` (TEXT NOT NULL DEFAULT '')
- `share_token` (TEXT NOT NULL DEFAULT '')

### 2. `pins`
Stores saved places mapped to collections.
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `list_id` (INTEGER NOT NULL DEFAULT 1 REFERENCES lists(id) ON DELETE CASCADE)
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

### 3. `device_lists`
Maps device user tokens to lists for multi-device sync, row-level access control, and collaboration.
- `user_token` (TEXT NOT NULL)
- `list_id` (INTEGER NOT NULL REFERENCES lists(id) ON DELETE CASCADE)
- PRIMARY KEY (`user_token`, `list_id`)

---

## 🚀 Key Commands & CI/CD Pipeline

- **⚡ Run affected tests only**: `npm run test:affected` (evaluates git diffs and runs only impacted frontend/backend suites)
- **🪝 Install git pre-push hook**: `npm run setup:hooks` (runs `npm run test:affected` automatically before every `git push`)
- **Build/Check local backend**: `cargo check && cargo test`
- **Run linter**: `cargo clippy --all-targets`
- **Check formatting**: `cargo fmt --all -- --check`
- **Run frontend unit & a11y tests**: `npm test`
- **Run E2E Playwright tests**: `npm run test:e2e`
- **Run local server**: `cargo run` (accessible locally on port `3000`)
- **Docker build**: Multi-stage build with `cargo-chef` dependency caching (`docker build -t blist .`)
- **Deploy locally via Fly CLI**: `fly deploy --local-only`
- **CI/CD Workflow (`.github/workflows/ci.yml`)**:
  - Runs `backend` and `frontend-e2e` in parallel across all matrix checks.
  - Caches Playwright browser binaries in `~/.cache/ms-playwright`.
  - Deployment to Fly.io (`deploy` job) strictly gates on all tests passing on the `main` branch.
- **Deploy manually to staging**: Run the "CD - Deploy to Staging" workflow dispatch in GitHub Actions or `fly deploy --app blist-staging-radmuffin --local-only`.
