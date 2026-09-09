---
name: blist-directives
description: >-
  Enforces UI design minimalism, typographic quality, zero-tech-debt engineering standards,
  and autonomous commit/push execution workflows for the bList project.
---

# bList Development Directives & Standards

This skill codifies the core user preferences and engineering standards for **bList** (Visual Map Bucket List & Trip Planner).
Always apply these guidelines when designing interfaces, writing code, refactoring, and planning features.

---

## 1. UI Design & Styling Directives (First-Try Perfection)

### A. Zero Fluff & Minimalism ("If Intuitive, Do Not Explain")
- **Never explain functionality that is already self-evident.**
- Strip away descriptive label filler, e.g.:
  - ❌ "16 vibrant categories" -> Just show the category pills/swatches.
  - ❌ "Click to mark as visited..." -> The icon/status pill is self-explanatory.
  - ❌ "Search for places, tags, or notes in your collection" -> Compact placeholder: `Search places, tags, notes...`.
- Keep helper notes, subtext, and instructions to an absolute minimum or omit entirely.
- Ensure buttons, badges, and chips use concise, punchy text (1–3 words max).

### B. Typography & Whitespace Utilization
- **No Truncated Place Titles**: Titles on place cards must never be clipped with ellipsis (`...`). Allow natural wrapping (`white-space: normal; word-break: break-word;`).
- **Whitespace Maximization**: Dynamically scale font size via `getAutoTitleFontSize(title)` so short titles (e.g. *Rome*, *Tokyo*, *Koumchi*) command the tile whitespace boldly (`19px`), while longer titles scale down smoothly (`16.5px`–`12.5px`).
- **Distinctive Display Typography**: Use modern, high-character display fonts (`Outfit`) for titles and brand elements, backed by clean body typography (`Plus Jakarta Sans`), avoiding generic, boring system fallbacks.

### C. Compact, Non-Intrusive Layout & Contrast
- Header elements, search/omni bar, and filter chips must fit neatly without wrapping awkwardly or causing horizontal overflows on standard mobile viewports (390×844 Pixel/iPhone).
- **Light & Dark Mode Contrast Integrity**: Always test and enforce strict WCAG contrast in both themes. Never hardcode light text on light mode card placeholders or gradients. Keep distance tags, addresses, and badges razor-sharp and legible.

---

## 2. Architecture & Backend Standards (Zero Tech Debt)

### A. Axum Send Bounds & Scraper Html Struct
- `scraper::Html` is `!Send`. Any HTML parsing in `src/scraper.rs` must happen in an isolated synchronous scope and be fully dropped before any `.await` statement occurs.

### B. SQLite Transactions & WAL Mode
- Maintain WAL mode with foreign keys enabled.
- Database queries use `Arc<Mutex<Connection>>` in Axum state. Keep transaction locks brief.
- Always use parameterized queries (`params![]` / `?`) to prevent SQL injection.
- Multi-device sync uses anonymous tokens (`X-User-Token`, `owner_token`, `device_lists`).

### C. SSRF & Security Protection
- All external HTTP requests (ingest, metadata preview, geocoding) must be validated through `src/security.rs` (`validate_url_for_ssrf`, `validate_url_with_dns_pin`, `build_safe_http_client`) to block private IP ranges, cloud metadata endpoints, and DNS rebinding attacks.

---

## 3. Frontend Standards (Zero-Build Vanilla ES6+)

- Strictly zero-build: no Webpack, Vite, Rollup, or npm bundlers for client-side code.
- Native browser ES6 JavaScript with Leaflet.js for mapping.
- All user-controlled text inserted into the DOM must pass through `Utils.escapeHtml()` and all URLs through `Utils.sanitizeUrl()`.
- Place pure helper logic and calculations in `static/helpers.js` and export via UMD for unit testing in `tests/frontend.test.js`.

---

## 4. Autonomous Execution, Commit & Push Protocol

1. **Quality Verification**:
   - Format: `cargo fmt --all -- --check`
   - Lint: `cargo clippy --all-targets` (must be 0 warnings)
   - Test: `cargo test --all-targets`
   - Affected runner: `npm run test:affected`
   - Frontend unit suite: `npm test`

2. **Autonomous Push to `main` at Task Completion (DEFAULT)**:
   - **At the end of a task, commit and push directly to `origin/main` by default**, unless the user explicitly requested not to push.
   - Do not pause, hesitate, or ask for extra confirmation before pushing completed work.

3. **Mandatory UI Changes & Active CI Surveillance**:
   - Whenever modifying UI layout, CSS styles, or DOM hierarchy (`static/index.html`, `static/style.css`, `static/app.js`), audit mobile clearance, touch target sizes, and pointer event handling.
   - After pushing changes, agents **MUST actively monitor GitHub Actions CI** (`gh run list --limit 1` / `gh run watch`) until completion (`conclusion: success`).
   - If CI fails, immediately diagnose (`gh run view <id> --log-failed`), resolve the issue, and push a fix autonomously.
