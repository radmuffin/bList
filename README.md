# 🗺️ bList — Visual Map Bucket List & Trip Planner

A lightweight, blazing-fast, visual map bucket list and trip planner written in Rust. Paste or share links (Google Maps, Apple Maps, Instagram posts, travel blogs), automatically extract place metadata, and view them on an interactive map.

Designed as a Mobile-First PWA (Progressive Web App) that integrates directly into your phone's native Share Sheet!

---

## 📸 App Preview

<div align="center">
  <img src="./screenshots/desktop-light.jpg" alt="bList Desktop Light Mode" width="85%" />
  <p><em>Desktop Experience: Interactive Map with Custom Pins, Progress Tracking & Omni-Link Ingestion</em></p>
</div>

<p align="center">
  <img src="./screenshots/desktop-dark.jpg" alt="bList Desktop Dark Mode" width="48%" />
  <img src="./screenshots/mobile-view.jpg" alt="bList Mobile PWA" width="48%" />
</p>

---

## ⚡ Features

### 1. 📥 Omni-Link Saving (Deterministic Scraping)
- **Google Maps Places & Businesses**: Handles place IDs, coordinates (`!3dlat!4dlng`), standard coordinates (`@lat,lng`), shortened links (`maps.app.goo.gl`), and falls back to Nominatim to resolve place queries automatically.
- **Apple Maps**: Extracts coordinates from URL queries (`ll=lat,lng`), addresses, and search parameters.
- **Instagram**: Scrapes location signals, post text, and media images.
- **General Articles**: Extracts OpenGraph, schema itemprops, or reverse-geocodes post titles.
- **SSRF Hardened**: All outbound requests are validated against private IP ranges, cloud metadata endpoints, and malicious redirects.

### 2. 📱 Native Mobile PWA & Multi-Device Sync
- **Web Share Target API**: Installed on iOS or Android, **bList appears directly in your phone's native Share Sheet**. Select "Share" inside Google Maps or Safari, and save links instantly!
- **Instant Device Sync**: Link your phone and desktop via QR Code or Sync Key—no passwords or personal accounts required.
- **Trip Sharing & Collaboration**: Share custom trips with a unique collaboration link (`/lists/join`).
- **Offline Map Tile Caching**: Service workers automatically cache browsed regions so the map continues working offline.

### 3. 🎯 Location-Aware & Delightful UX
- **Locate Me**: Single tap GPS lookup displaying your position with a pulsing radar marker.
- **Live Distance Calculations**: Displays real-time distances (`1.4 mi away` / `800 m away`) on cards and marker popups.
- **Multi-Stop Route Navigation**: Calculate total trip route distances and open turn-by-turn multi-stop routes directly in Google Maps.
- **Plus Codes (Open Location Codes)**: Automatic Plus Code generation and copy tools for precise geolocation without street addresses.
- **Sort by Nearest**: Sort your active trip list by proximity to your current location.
- **Dark Mode**: Supports system default theme matching (`prefers-color-scheme: dark`) and manual override toggling (☀️/🌙/💻). Map layer automatically adapts to Esri Dark Canvas.
- **Surprise Me 🎲**: Randomly picks an unvisited spot on your list, flies the map there, and opens the popup.
- **Weather Widget ☀️**: Displays real-time temperature and weather indicators for every saved pin.

---

## 🚀 Quick Start

### Local Development
```bash
# Run backend tests
cargo test

# Run frontend unit & accessibility tests
npm test

# Run Playwright E2E tests
npm run test:e2e

# Run Axum dev server
cargo run
```
Open **`http://localhost:3000`** in your browser.

---

## ☁️ Deployment & CI/CD

### 1. Unified CI/CD Pipeline
GitHub Actions automatically runs a unified testing and deployment pipeline on push to `main` ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)):
- **Parallel Testing**: `backend` (Rust tests & checks) and `frontend-e2e` (Playwright & unit tests) run concurrently.
- **Gated Deployments**: The Fly.io deployment step is strictly conditional on all tests passing.
- **Caching**: Docker builds use `cargo-chef` multi-stage dependency caching and cached Playwright browser binaries.

### 2. Production (Fly.io)
👉 **[https://blist-radmuffin.fly.dev](https://blist-radmuffin.fly.dev)**

- **Configuration**: Managed via [`fly.toml`](fly.toml) with a persistent NVMe volume (`blist_data` on `/data`) ensuring SQLite files are retained across deploys.
- **Local CLI Deployment**: `fly deploy --local-only` builds on your local Docker engine and pushes the container image directly to Fly.

### 3. Staging (Manual Deploy)
To test feature branches on staging:
1. Go to the **Actions** tab on your GitHub repository.
2. Select **"CD - Deploy to Staging"** in the sidebar.
3. Click **"Run workflow"** and input the branch name you'd like to push.
👉 Staging is hosted at: **[https://blist-staging-radmuffin.fly.dev](https://blist-staging-radmuffin.fly.dev)**

For detailed instruction files, see **[`deploy/AWS_DEPLOYMENT.md`](deploy/AWS_DEPLOYMENT.md)** and **[`deploy/GCP_DEPLOYMENT.md`](deploy/GCP_DEPLOYMENT.md)**.
