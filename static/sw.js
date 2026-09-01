// bList Service Worker - Offline Caching & PWA Support
const CACHE_NAME = 'blist-app-v5';
const TILE_CACHE_NAME = 'blist-tiles-v1';
const MAX_TILE_CACHE_ITEMS = 300;

// Core shell assets to precache on install
const PRECACHE_ASSETS = [
  '/',
  '/index.html',
  '/style.css',
  '/helpers.js',
  '/app.js',
  '/privacy.html',
  '/manifest.webmanifest',
  '/icons/icon.svg',
  '/icons/icon-maskable.svg',
  '/icons/icon-192.png',
  '/icons/icon-512.png',
  '/icons/icon-maskable-192.png',
  '/icons/icon-maskable-512.png',
  '/icons/apple-touch-icon.png',
  '/icons/favicon.png',
  'https://unpkg.com/leaflet@1.9.4/dist/leaflet.css',
  'https://unpkg.com/leaflet@1.9.4/dist/leaflet.js',
  'https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap',
  'https://unpkg.com/lucide@latest'
];

// Map Tile domains for offline map tile caching
const TILE_DOMAINS = [
  'tile.openstreetmap.org',
  'basemaps.cartocdn.com',
  'cartodb-basemaps',
  'server.arcgisonline.com'
];

// Install Event - Precache App Shell
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then((cache) => {
        return cache.addAll(PRECACHE_ASSETS).catch((err) => {
          console.warn('[SW] Some precache assets failed to load:', err);
        });
      })
      .then(() => self.skipWaiting())
  );
});

// Activate Event - Clean up outdated caches & take control immediately
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames
          .filter((name) => name !== CACHE_NAME && name !== TILE_CACHE_NAME)
          .map((name) => caches.delete(name))
      );
    }).then(() => self.clients.claim())
  );
});

// Helper: Trim tile cache to prevent unbounded storage growth
async function trimCache(cacheName, maxItems) {
  const cache = await caches.open(cacheName);
  const keys = await cache.keys();
  if (keys.length > maxItems) {
    await cache.delete(keys[0]);
    trimCache(cacheName, maxItems);
  }
}

// Fetch Event - Route requests through appropriate caching strategies
self.addEventListener('fetch', (event) => {
  const request = event.request;
  const url = new URL(request.url);

  // Only handle GET requests with caching
  if (request.method !== 'GET') {
    return;
  }

  // 1. API Requests -> Network-First (with offline JSON fallback)
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(
      fetch(request)
        .catch(() => {
          return new Response(
            JSON.stringify({
              success: false,
              error: 'You are currently offline. Connect to the internet to perform this action.',
              offline: true
            }),
            {
              headers: { 'Content-Type': 'application/json' }
            }
          );
        })
    );
    return;
  }

  // 2. Map Tiles -> Cache-First with Network Fallback (Offline Maps!)
  const isMapTile = TILE_DOMAINS.some((domain) => url.hostname.includes(domain));
  if (isMapTile) {
    event.respondWith(
      caches.open(TILE_CACHE_NAME).then(async (cache) => {
        const cachedResponse = await cache.match(request);
        if (cachedResponse) {
          return cachedResponse;
        }
        try {
          const networkResponse = await fetch(request);
          if (networkResponse && networkResponse.status === 200) {
            cache.put(request, networkResponse.clone());
            trimCache(TILE_CACHE_NAME, MAX_TILE_CACHE_ITEMS);
          }
          return networkResponse;
        } catch (err) {
          // Return empty transparent tile on complete network error if not in cache
          return cachedResponse || new Response('', { status: 408 });
        }
      })
    );
    return;
  }

  // 3. Navigation / HTML Document -> Network-First, fallback to cached /index.html
  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .catch(async () => {
          const cache = await caches.open(CACHE_NAME);
          const cached = await cache.match('/index.html') || await cache.match('/');
          return cached || new Response('Offline - Please reconnect to the internet', {
            headers: { 'Content-Type': 'text/html' }
          });
        })
    );
    return;
  }

  // 4. Static Assets (JS, CSS, Fonts, Images, Icons) -> Stale-While-Revalidate
  event.respondWith(
    caches.match(request).then((cachedResponse) => {
      const fetchPromise = fetch(request)
        .then(async (networkResponse) => {
          if (networkResponse && networkResponse.status === 200) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, networkResponse.clone());
          }
          return networkResponse;
        })
        .catch(() => cachedResponse);

      return cachedResponse || fetchPromise;
    })
  );
});
