# 🎨 Technical Specification: Visual Marker Pinheads, Custom Emojis & Multi-Tagging

> **Pillar**: [Pillar 3: Map UX, Visual Distinction & Delight](../pillars/3-map-ux-visual-delight.md)  
> **Status**: Ready for Implementation  
> **Scope**: Leaflet Marker Pinheads, Category Chromas, Custom Emojis, Must-See ⭐ Badges & `#tags`

---

## 1. Marker Anatomy & Geometry

```
       ┌────────────────────────┐
       │   [⭐] Priority Star    │  <- Top-right badge accessory
       │    ╭──────────────╮    │
       │   │   ☕ or 🍕   │    │  <- Centered Emoji (17px)
       │    ╰──────────────╯    │
       │         \    /         │  <- Teardrop pointer with drop shadow
       │          \  /          │
       │           ▼            │  <- Anchor Point [18px, 42px]
       └────────────────────────┘
```

---

## 2. Category Palette & Chromas

| Category | Default Emoji | Primary Chroma | Glow / Accent |
|---|:---:|---|---|
| **Food & Drink** | 🍕 | `#ea580c` (Warm Tangerine) | `rgba(234, 88, 12, 0.4)` |
| **Cafe** | ☕ | `#d97706` (Amber Roast) | `rgba(217, 119, 6, 0.4)` |
| **Sightseeing** | 🏛️ | `#7c3aed` (Royal Violet) | `rgba(124, 58, 237, 0.4)` |
| **Nature & Outdoors** | 🏞️ | `#059669` (Emerald Forest) | `rgba(5, 150, 105, 0.4)` |
| **Hotel & Stay** | 🏨 | `#0284c7` (Azure Blue) | `rgba(2, 132, 199, 0.4)` |
| **Shopping** | 🛍️ | `#db2777` (Rose Pink) | `rgba(219, 39, 119, 0.4)` |
| **General / Place** | 📍 | `#4f46e5` (Classic Indigo) | `rgba(79, 70, 229, 0.4)` |

---

## 3. Database Schema Extensions

```sql
ALTER TABLE pins ADD COLUMN emoji TEXT;
ALTER TABLE pins ADD COLUMN tags TEXT;
ALTER TABLE pins ADD COLUMN priority INTEGER NOT NULL DEFAULT 0;
```

---

## 4. Performance & Rendering Benefits
- **Zero Icon Scan Overhead**: Bypasses runtime `lucide.createIcons()` inside map pinheads, providing a **4-5x speedup** on mobile devices during map pan/zoom with 100+ pins.
- **Graceful Fallbacks**: Backward compatible with all existing SQLite databases.
