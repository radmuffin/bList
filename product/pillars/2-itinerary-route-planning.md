# 🗺️ Pillar 2: Trip Itinerary & Daily Route Organization

> **Strategic Goal**: Transform bList from a static bucket list into a high-utility day-by-day travel planner with automated route optimization.

---

## 🎯 Key Initiatives & Features

### 2.1 Day-by-Day Grouping & Timeline Planner
- **Problem**: 30+ pins scattered on a map create cognitive overload without a chronological itinerary.
- **Solution**:
  - Assign pins to `Day 1`, `Day 2`, `Day 3`, or `General / Unassigned`.
  - Filter map pins by active day or show the entire trip with day color-coding.
  - Drag-and-drop reordering of stops in the sidebar.

### 2.2 1-Click Route Order Optimizer (TSP 2-Opt Heuristic)
- **Problem**: Manually ordering 6 stops to minimize zigzagging across town is frustrating.
- **Solution**:
  - Client-side or backend 2-Opt Traveling Salesperson heuristic algorithm.
  - Reorders the list for minimal total distance in <5ms.
  - Displays total route distance (miles/km) and estimated travel time.

### 2.3 Opening Hours & Live "Open Now" Detector
- **Problem**: Showing up to a landmark or cafe when it's closed ruins travel days.
- **Solution**:
  - Extract structured opening hours from website metadata or OpenStreetMap tags.
  - Display green/red status chips (🟢 Open Now, 🔴 Closed).

---

## 🛠️ Database Schema Extensions

```sql
ALTER TABLE pins ADD COLUMN day_group INTEGER DEFAULT 0; -- 0: Unscheduled, 1: Day 1, etc.
ALTER TABLE pins ADD COLUMN custom_order INTEGER DEFAULT 0;
ALTER TABLE pins ADD COLUMN opening_hours TEXT DEFAULT '';
```
