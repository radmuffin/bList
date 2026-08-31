# 🗺️ bList — Product & Engineering Documentation

Welcome to the **bList Product Hub**. This directory organizes our target customer personas, strategic pillars, feature specifications, and technical guidelines for building a world-class, privacy-first visual bucket list and trip planner.

---

## 🧭 Navigation & Directory Map

### 👥 [Personas](./personas/)
Deep dives into the real humans using bList, their mental models, frustrations, and daily workflows:
- [**The Social Media & Spot Collector**](./personas/spot-collector.md) — Fast mobile-first curation from Instagram, TikTok, and travel blogs.
- [**The Couple & Group Trip Planners**](./personas/group-planners.md) — Frictionless, account-free trip sharing and collaborative itinerary building.
- [**The Privacy-Conscious Nomad & De-Googler**](./personas/digital-nomad.md) — Data sovereignty, offline reliability, and self-hostable mapping without telemetry.

### 🏛️ [Strategic Pillars & Initiatives](./pillars/)
Our four product pillars defining the roadmap and feature priorities:
1. [**Pillar 1: Frictionless Ingestion & Data Sovereignty**](./pillars/1-ingestion-data-sovereignty.md) — Universal import wizard (Google Takeout, KML, GeoJSON), browser extensions, and PWA share sheet parsing.
2. [**Pillar 2: Trip Itinerary & Daily Route Organization**](./pillars/2-itinerary-route-planning.md) — Day-by-day clustering, Traveling Salesperson (TSP) route optimization, and opening hours indicators.
3. [**Pillar 3: Map UX, Visual Distinction & Delight**](./pillars/3-map-ux-visual-delight.md) — Custom pin emojis/colors, multi-tagging (`#sunset`, `#coffee`), ⭐ priority stars, and offline map packs.
4. [**Pillar 4: Real-time Collaboration & Public Sharing**](./pillars/4-collaboration-sharing.md) — Read-only public showcases, live Server-Sent Events (SSE) sync, and lightweight expense tracking.

### ⚙️ [Technical Guidelines & Architecture](./tech-specs/)
- [**Architecture & Performance Guidelines**](./tech-specs/architecture-guidelines.md) — Rust Axum 0.7 best practices, SQLite WAL mode, Leaflet map rendering optimizations, and PWA offline storage standards.

---

## 🎯 Core Product Principles

1. **Zero-Account, Zero-Friction**: Users must never be blocked by login walls, OAuth permissions, or forgotten passwords to save a place or share a trip.
2. **Sub-100ms Response Times**: Rust Axum backend + SQLite WAL mode provides instant, deterministic execution.
3. **No Heavy Frontend Framework Bloat**: Pure vanilla ES6+ JS and CSS with Leaflet.js ensures fast mobile loading and low memory overhead.
4. **Data Sovereignty & Portability**: Zero vendor lock-in. Full GeoJSON, KML, and JSON exports with seamless 1-click import.
