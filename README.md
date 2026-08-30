# 🗺️ bList — Visual Map Bucket List & Trip Planner

A lightweight, blazing-fast, visual map bucket list and trip planner written in Rust. Paste or share links (Google Maps, Apple Maps, Instagram posts, travel blogs), automatically extract place metadata, and view them on an interactive map.

Designed as a Mobile-First PWA (Progressive Web App) that integrates directly into your phone's native Share Sheet!

---

## ⚡ Features

### 1. 📥 Omni-Link Saving (Deterministic Scraping)
- **Google Maps Places & Businesses**: Handles place IDs, coordinates (`!3dlat!4dlng`), standard coordinates (`@lat,lng`), shortened links (`maps.app.goo.gl`), and falls back to Nominatim to resolve place queries automatically.
- **Apple Maps**: Extracts coordinates from URL queries (`ll=lat,lng`), addresses, and search parameters.
- **Instagram**: Scrapes location signals, post text, and media images.
- **General Articles**: Extracts OpenGraph, schema itemprops, or reverse-geocodes post titles.

### 2. 📱 Native Mobile PWA & iOS/Android wrappers
- **Web Share Target API**: Installed on iOS or Android, **bList appears directly in your phone's native Share Sheet**. Select "Share" inside Google Maps or Safari, and save links instantly!
- **Offline Map Tile Caching**: service workers automatically cache browsed regions so the map continues working offline.
- **Touch-Friendly Drawer Layout**: Sleek bottom drawer sheet with quick view switcher (`[ 🗺️ Map ] [ 📋 List ]`).

### 3. 🎯 Location-Aware & Delightful UX
- **Locate Me**: Single tap GPS lookup displaying your position with a pulsing radar marker.
- **Live Distance Calculations**: Displays real-time distances (`1.4 mi away` / `800 m away`) on cards and marker popups.
- **Sort by Nearest**: Sort your active trip list by proximity to your current location.
- **Dark Mode**: Supports system default theme matching (`prefers-color-scheme: dark`) and manual override toggling (☀️/🌙/💻). Map layer automatically adapts to Esri Dark Canvas.
- **Surprise Me 🎲**: Randomly picks an unvisited spot on your list, flies the map there, and opens the popup.
- **Weather Widget ☀️**: Displays real-time temperature and weather indicators for every saved pin.

---

## 🚀 Quick Start

### Local Development
```bash
# Run tests
cargo test

# Run Axum dev server
cargo run
```
Open **`http://localhost:3000`** in your browser.

---

## ☁️ Deployment & CI/CD

### 1. Production (Fly.io)
The production application is set up for automatic deployments. Every push to the `main` branch compiles and deploys to:
👉 **[https://blist-radmuffin.fly.dev](https://blist-radmuffin.fly.dev)**

- **Configuration**: Managed via [`fly.toml`](fly.toml) with a persistent NVMe volume (`blist_data` on `/data`) ensuring SQLite files are retained across deploys.
- **Setup**: Added your `FLY_API_TOKEN` to GitHub Secrets.

### 2. Staging (Manual Deploy)
To test feature branches on staging:
1. Go to the **Actions** tab on your GitHub repository.
2. Select **"CD - Deploy to Staging"** in the sidebar.
3. Click **"Run workflow"** and input the branch name you'd like to push.
👉 Staging is hosted at: **[https://blist-staging-radmuffin.fly.dev](https://blist-staging-radmuffin.fly.dev)**

For detailed instruction files, see **[`deploy/AWS_DEPLOYMENT.md`](deploy/AWS_DEPLOYMENT.md)** and **[`deploy/GCP_DEPLOYMENT.md`](deploy/GCP_DEPLOYMENT.md)**.
