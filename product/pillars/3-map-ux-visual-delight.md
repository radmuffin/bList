# ✨ Pillar 3: Map UX, Visual Distinction & Delight

> **Strategic Goal**: Deliver the most intuitive, beautiful, and customizable map interface that feels like a polished native app.

---

## 🎯 Key Initiatives & Features

### 3.1 Custom Pin Icons & Emoji Markers on Leaflet
- **Problem**: Default blue map markers look identical and bland on dense maps.
- **Solution**:
  - Render custom category or custom-selected emojis (☕, 🍕, 🏨, 🏔️, 🛍️) directly centered on the Leaflet pinhead marker.
  - Dynamic color palettes per category or custom pin accent colors.

### 3.2 Custom Tags & ⭐ Priority Stars
- **Problem**: Fixed category dropdowns don't capture travel vibes (`#sunset`, `#cheap-eats`, `#indoor`, `#romantic`).
- **Solution**:
  - Free-form multi-tagging input on pin modal.
  - Interactive multi-select filter bar in sidebar.
  - ⭐ "Must-See" flag with top-of-list sorting.

### 3.3 Offline Trip Packager (1-Click Tile Pre-caching)
- **Problem**: Flying abroad or traveling in national parks without cellular coverage renders maps blank.
- **Solution**:
  - Calculate bounding box encompassing all pins in a list.
  - Pre-fetch raster tiles for zoom levels 12-16 into the Service Worker Cache.
  - Store offline pin metadata and photos in IndexedDB.

### 3.4 Foldable Pocket Itinerary (Print & PDF)
- **Problem**: Physical backups are invaluable when phone batteries die.
- **Solution**:
  - Clean `@media print` CSS layout generating a numbered foldable two-column guide with QR codes.
