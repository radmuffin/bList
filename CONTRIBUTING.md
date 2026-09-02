# 🤝 Contributing to bList

First off, thank you for considering contributing to **bList**! 🎉 We welcome contributions from developers, designers, writers, and travelers of all experience levels. Whether you are fixing a typo in the documentation, optimizing a Rust route calculation algorithm, refining mobile CSS, or adding a new link parser, your help is deeply appreciated.

bList is an open-source, lightweight, visual map bucket list and trip planner built with Rust, Axum, SQLite, and vanilla frontend web standards.

---

## 🧭 Table of Contents

- [🤝 Contributing to bList](#-contributing-to-blist)
  - [🧭 Table of Contents](#-table-of-contents)
  - [🌟 Code of Conduct \& Community Principles](#-code-of-conduct--community-principles)
  - [💡 Ways to Contribute](#-ways-to-contribute)
  - [🛠️ Local Development Setup](#️-local-development-setup)
    - [Prerequisites](#prerequisites)
    - [1. Fork and Clone the Repository](#1-fork-and-clone-the-repository)
    - [2. Install Node Dependencies](#2-install-node-dependencies)
    - [3. Run the Development Server](#3-run-the-development-server)
    - [4. Install Git Pre-Push Hook (Recommended)](#4-install-git-pre-push-hook-recommended)
  - [🧪 Testing \& Quality Checks](#-testing--quality-checks)
    - [Fast Affected Test Check (Recommended)](#fast-affected-test-check-recommended)
    - [Frontend Unit \& Accessibility Tests](#frontend-unit--accessibility-tests)
    - [Backend Rust Tests](#backend-rust-tests)
    - [Rust Linting \& Formatting](#rust-linting--formatting)
    - [End-to-End (E2E) Testing in CI](#end-to-end-e2e-testing-in-ci)
  - [📂 Codebase Tour](#-codebase-tour)
  - [⚠️ Critical Architectural Guidelines](#️-critical-architectural-guidelines)
    - [1. Axum Send Bounds \& `scraper::Html`](#1-axum-send-bounds--scraperhtml)
    - [2. SQLite WAL Mode \& Parameterized Queries](#2-sqlite-wal-mode--parameterized-queries)
    - [3. SSRF Protection \& DNS Pinning](#3-ssrf-protection--dns-pinning)
    - [4. Zero-Build Frontend Philosophy](#4-zero-build-frontend-philosophy)
    - [5. DOM Security \& XSS Prevention](#5-dom-security--xss-prevention)
  - [🚀 Submitting a Pull Request](#-submitting-a-pull-request)
    - [Step-by-Step Workflow](#step-by-step-workflow)
    - [Commit Message Guidelines](#commit-message-guidelines)
    - [PR Checklist](#pr-checklist)
  - [💬 Questions \& Getting Help](#-questions--getting-help)

---

## 🌟 Code of Conduct & Community Principles

We are committed to providing a friendly, safe, and welcoming environment for everyone:
- **Respect & Kindness**: Treat all contributors and users with empathy and respect.
- **Constructive Feedback**: Offer helpful, encouraging reviews and guidance.
- **Privacy & Simplicity**: Preserve bList's commitment to user privacy, zero ad tracking, and lean performance.

---

## 💡 Ways to Contribute

You do not need to be a Rust or GIS wizard to contribute! Here are many ways you can help:

1. **🦀 Backend Improvements (Rust / Axum / SQLite)**
   - Enhance scrapers in `src/scraper.rs` for new travel websites, Instagram formats, or mapping providers.
   - Optimize SQLite database queries and indices in `src/db/sqlite.rs`.
   - Add new export/import format converters in `src/importer.rs` and `src/routes/export.rs`.
   - Extend geocoding fallback mechanisms in `src/geocoder.rs`.

2. **🎨 Frontend & UI/UX (HTML5 / CSS3 / Vanilla JavaScript)**
   - Improve responsive mobile drawer layouts, bottom sheets, and touch gestures.
   - Enhance dark/light mode themes and map layer styling.
   - Add new filter chips, itinerary grouping options, or marker customization tools.
   - Boost accessibility (ARIA labels, keyboard navigation, color contrast).

3. **📱 Native Mobile & PWA (Capacitor / Web Share Target)**
   - Test and refine iOS and Android native builds via Capacitor (`android/`, `ios/`).
   - Improve offline map caching in the Service Worker (`static/sw.js`).
   - Enhance native share target resilience when receiving links from various mobile apps.

4. **🧪 Testing & Quality Assurance**
   - Expand unit tests in `tests/frontend.test.js` and `src/routes/tests.rs`.
   - Write Playwright E2E scenarios for critical user flows in `tests/e2e/`.

5. **📖 Documentation & Guides**
   - Fix typos, improve clarity, or add deployment guides for different hosting platforms.
   - Create video walkthroughs or write blog posts about using bList.

---

## 🛠️ Local Development Setup

### Prerequisites

Make sure you have the following tools installed:
- **[Rust](https://www.rust-lang.org/tools/install)** (Stable toolchain 1.75+)
- **[Node.js](https://nodejs.org/)** (v20 or later) and `npm`
- **[Git](https://git-scm.com/)**

### 1. Fork and Clone the Repository

```bash
# Clone your fork
git clone https://github.com/<your-username>/bList.git
cd bList

# Add upstream remote
git remote add upstream https://github.com/radmuffin/bList.git
```

### 2. Install Node Dependencies

```bash
npm install
```

### 3. Run the Development Server

Start the Axum web server and static asset server:

```bash
cargo run
```

Once started, open **`http://localhost:3000`** in your browser. Any changes to `static/` files (HTML, JS, CSS) take effect immediately on page refresh without needing to recompile the backend.

### 4. Install Git Pre-Push Hook (Recommended)

To ensure fast feedback before pushing code, install the lightweight pre-push hook:

```bash
npm run setup:hooks
```

This runs `npm run test:affected` automatically before every `git push`.

---

## 🧪 Testing & Quality Checks

bList is designed with a rapid two-step verification workflow to maintain high velocity and keep testing frictionless.

### Fast Affected Test Check (Recommended)

Before committing or pushing, run the fast affected check (<5s). This script inspects your git diff and runs only the tests impacted by your changes:

```bash
npm run test:affected
```

### Frontend Unit & Accessibility Tests

Runs the full Node.js test runner suite for frontend utilities, helpers, and accessibility assertions:

```bash
npm test
```

### Backend Rust Tests

Runs backend unit, integration, and database tests:

```bash
cargo test
```

### Rust Linting & Formatting

Check that code conforms to Rust idioms and formatting rules:

```bash
# Check formatting
cargo fmt --all -- --check

# Run Clippy linter
cargo clippy --all-targets -- -D warnings
```

### End-to-End (E2E) Testing in CI

To preserve local machine CPU and memory, full multi-browser Playwright test matrix runs (**Desktop Chrome**, **Mobile Pixel**, **Mobile Safari**) are automatically executed in parallel on GitHub Actions runners when you push to your branch or open a pull request.

You can also trigger CI on demand using:
```bash
npm run test:branch-ci
```

---

## 📂 Codebase Tour

Here is a quick map of the repository structure:

```
├── Cargo.toml                  # Rust dependencies & metadata
├── package.json                # Node scripts, Capacitor & dev tools
├── AGENTS.md                   # AI agent guidance & architecture rules
├── CONTRIBUTING.md             # This contributor guide
├── README.md                   # Project overview & screenshots
│
├── src/                        # Rust Backend (Axum + SQLite)
│   ├── main.rs                 # Axum application entrypoint & middleware
│   ├── models.rs               # Data structures (Pin, List, IngestRequest)
│   ├── scraper.rs              # Deterministic link metadata parser
│   ├── geocoder.rs             # OpenStreetMap Nominatim geocoder backup
│   ├── plus_code.rs            # Open Location Code (Plus Code) generator
│   ├── security.rs             # SSRF protection, DNS pinning & safe HTTP client
│   ├── importer.rs             # Google Takeout, CSV, KML, GeoJSON parser
│   ├── metrics.rs              # Prometheus telemetry (/metrics)
│   ├── db/
│   │   ├── mod.rs              # Database interface & traits
│   │   ├── sqlite.rs           # SQLite connection & queries (WAL mode)
│   │   └── in_memory.rs        # In-memory DB implementation for testing
│   └── routes/
│       ├── mod.rs              # Route registry
│       ├── pins.rs             # Pin CRUD & filtering endpoints
│       ├── lists.rs            # Trip list management & sharing endpoints
│       ├── ingest.rs           # Omni-link ingestion & preview endpoints
│       ├── export.rs           # GeoJSON, CSV, JSON export endpoints
│       └── user.rs             # Device sync token association
│
├── static/                     # Frontend Single Page App (Vanilla JS/CSS)
│   ├── index.html              # Main application layout & modals
│   ├── app.js                  # Frontend state, map controller & UI events
│   ├── helpers.js              # Pure helper functions, geometry & formatting
│   ├── style.css               # Modern CSS with dark/light variables
│   ├── sw.js                   # Service Worker (offline tile & asset cache)
│   ├── manifest.webmanifest    # PWA configuration & Web Share Target API
│   └── icons/                  # SVG and PNG app icons
│
├── tests/                      # Frontend & Integration Tests
│   ├── frontend.test.js        # Unit test suite for helper functions
│   ├── accessibility.test.js   # Accessibility (a11y) & DOM contract tests
│   └── e2e/
│       └── app.spec.js         # Playwright end-to-end tests
│
├── deploy/                     # Cloud Deployment & Mobile Release Guides
│   ├── ANDROID_RELEASE_GUIDE.md # Google Play Store packaging & release guide
│   ├── AWS_DEPLOYMENT.md       # AWS EC2 / ECS deployment guide
│   ├── GCP_DEPLOYMENT.md       # Google Cloud Run deployment guide
│   └── docker-compose.prod.yml # Production Docker compose setup
│
└── .github/workflows/          # GitHub Actions CI/CD
    ├── ci.yml                  # Main CI/CD pipeline gating deployment
    ├── test-branch.yml         # 5-job parallel test matrix on branches
    └── deploy-staging.yml      # Staging deployment workflow
```

---

## ⚠️ Critical Architectural Guidelines

When writing code for bList, please adhere to these core design rules:

### 1. Axum Send Bounds & `scraper::Html`
The `scraper::Html` struct parses document trees using `Cell<usize>` reference counting and **does not implement `Send`**. Because Axum handler futures must be `Send`, any HTML parsing in `src/scraper.rs` **must** occur within an isolated synchronous block and be dropped before any `.await` call:

```rust
// Correct Pattern:
let extracted_meta = {
    let doc = Html::parse_document(&html_body);
    // Parse elements into Send-safe structs
    extracted_meta
}; // `doc` is dropped here
// Safe to use .await after this block
```

### 2. SQLite WAL Mode & Parameterized Queries
- SQLite operates in **Write-Ahead Logging (WAL)** mode with foreign keys enabled (`PRAGMA foreign_keys = ON;`).
- Database connections are protected behind `Arc<Mutex<Connection>>`. Keep lock scopes as brief as possible.
- **Always** use parameterized queries (`params![]` or `?`) to eliminate SQL injection risks.

### 3. SSRF Protection & DNS Pinning
- All external HTTP requests (link ingestion, preview scraping, geocoding) must pass through `src/security.rs`.
- `validate_url_with_dns_pin` resolves DNS hostnames and verifies all resolved IP addresses to block private subnets (RFC 1918, loopback, link-local, cloud metadata hostnames like `169.254.169.254`) and prevent Time-of-Check to Time-of-Use (TOCTOU) DNS rebinding attacks.

### 4. Zero-Build Frontend Philosophy
- The frontend intentionally uses vanilla modern JavaScript (ES6+ modules) and CSS without bundlers (webpack, vite, rollup).
- Pure logic functions (formatting, calculations, geometry, filtering) live in `static/helpers.js` and are exported using UMD (`module.exports` for Node tests / `window.Helpers` in browser).

### 5. DOM Security & XSS Prevention
- Any user-controlled text inserted into the DOM must pass through `Utils.escapeHtml()` (or `Helpers.escapeHtml()`).
- Any user-supplied URL rendered into `href` or `src` attributes must be validated with `Utils.sanitizeUrl()`.

---

## 🚀 Submitting a Pull Request

### Step-by-Step Workflow

1. **Create a branch**:
   ```bash
   git checkout -b feat/your-feature-name
   # or: git checkout -b fix/your-bug-fix
   ```
2. **Make your changes** and test locally:
   ```bash
   npm run test:affected
   cargo clippy --all-targets
   cargo fmt --all -- --check
   ```
3. **Commit your changes**:
   ```bash
   git add .
   git commit -m "feat(scraper): add support for Apple Maps Guides links"
   ```
4. **Push to your fork**:
   ```bash
   git push origin feat/your-feature-name
   ```
5. **Open a Pull Request** on GitHub against the `main` branch of `radmuffin/bList`.

### Commit Message Guidelines

We follow standard Conventional Commits:
- `feat:` A new feature
- `fix:` A bug fix
- `docs:` Documentation only changes
- `style:` Formatting, whitespace, or lint fixes
- `refactor:` Code restructuring without behavioral changes
- `test:` Adding or improving tests
- `chore:` Tooling, CI/CD, or dependency updates

### PR Checklist

Before submitting your PR, ensure:
- [ ] `npm run test:affected` passes.
- [ ] `cargo test` passes.
- [ ] `cargo clippy --all-targets` reports no warnings.
- [ ] `cargo fmt --all -- --check` reports clean formatting.
- [ ] New features include corresponding unit or integration tests.
- [ ] Documentation (`README.md` or comments) has been updated if applicable.

---

## 💬 Questions & Getting Help

- **Found a bug or have an idea?** Open an issue on GitHub at [https://github.com/radmuffin/bList/issues](https://github.com/radmuffin/bList/issues).
- **Need guidance on architecture?** Feel free to open a Discussion or draft PR and ask for feedback. We are thrilled to collaborate with you!

Thank you for helping make **bList** the best open-source travel and map bucket list tool! 🌍✈️
