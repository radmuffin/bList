# 🔗 Pillar 4: Real-time Collaboration & Public Sharing

> **Strategic Goal**: Enable viral trip sharing and frictionless live collaboration without forcing users to register accounts or download bloated apps.

---

## 🎯 Key Initiatives & Features

### 4.1 Public Trip Showcase & Read-Only Embeds
- **Problem**: Travel bloggers and content creators want to share their favorite spots without letting viewers edit or delete pins.
- **Solution**:
  - Two-tiered links:
    1. **Collaborative Join Link** (`/lists/join?token=...`): Full editor privileges.
    2. **Public View-Only Showcase** (`/view/:slug`): Stripped-down aesthetic viewer with embeddable `<iframe>` support.

### 4.2 Live Collaborative Sync via Server-Sent Events (SSE)
- **Problem**: Two people planning a trip simultaneously must refresh the page to see newly added pins.
- **Solution**:
  - Lightweight Axum SSE endpoint (`GET /api/lists/:id/stream`).
  - Broadcasts pin creation, deletion, and visited state updates in real-time.

### 4.3 Lightweight Expense & Split Tally
- **Problem**: Travelers juggle separate apps (Splitwise / Excel) to estimate activity budgets.
- **Solution**:
  - Optional `cost` and `currency` field on each pin.
  - Automatic trip budget total and simple split breakdown in the sidebar footer.
