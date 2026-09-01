// ============================================================================
// bList - Visual Map Bucket List & Trip Planner
// Frontend Architecture: Namespaced Modular Engine
// ============================================================================

(function () {
  'use strict';

  // ==========================================================================
  // 1. Constants & Configuration
  // ==========================================================================
  const CONFIG = {
    MAP_LAYERS: {
      osm: {
        name: 'Streets (OSM)',
        url: 'https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png',
        options: {
          maxZoom: 19,
          attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap</a> contributors'
        }
      },
      dark: {
        name: 'Dark Mode',
        url: 'https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png',
        options: {
          maxZoom: 19,
          attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap</a> contributors',
          className: 'dark-map-tiles'
        }
      }
    },

    API_BASE_URL: (function () {
      try {
        const customHost = localStorage.getItem('blist_api_host');
        if (customHost) return customHost.replace(/\/+$/, '');

        const isNative =
          (window.Capacitor && window.Capacitor.isNativePlatform && window.Capacitor.isNativePlatform()) ||
          window.location.protocol === 'capacitor:' ||
          window.location.protocol === 'ionic:' ||
          window.location.protocol === 'file:' ||
          (window.location.hostname === 'localhost' && window.location.port === '');

        if (isNative) {
          return 'https://blist-radmuffin.fly.dev';
        }
      } catch (_) {}
      return '';
    })(),

    CATEGORY_ICONS: {
      'Food & Drink': 'utensils',
      'Cafe': 'coffee',
      'Sightseeing': 'landmark',
      'Nature & Outdoors': 'trees',
      'Hotel & Stay': 'bed',
      'Shopping': 'shopping-bag',
      'General': 'map-pin',
      'Social': 'instagram',
      'Place': 'map-pin'
    },

    CATEGORY_EMOJIS: {
      'Food & Drink': '🍔',
      'Cafe': '☕',
      'Sightseeing': '🏛️',
      'Nature & Outdoors': '🏞️',
      'Hotel & Stay': '🏨',
      'Shopping': '🛍️',
      'General': '📍',
      'Social': '📸',
      'Place': '📍'
    },

    CATEGORY_COLORS: {
      'Food & Drink': '#ea580c',
      'Cafe': '#854d0e',
      'Sightseeing': '#7c3aed',
      'Nature & Outdoors': '#16a34a',
      'Hotel & Stay': '#0284c7',
      'Shopping': '#db2777',
      'General': '#2563eb',
      'Social': '#e11d48',
      'Place': '#2563eb'
    },

    WEATHER_CODE_MAP: {
      0: { icon: '☀️', text: 'Clear' },
      1: { icon: '🌤️', text: 'Mainly Clear' },
      2: { icon: '⛅', text: 'Partly Cloudy' },
      3: { icon: '☁️', text: 'Overcast' },
      45: { icon: '🌫️', text: 'Fog' },
      48: { icon: '🌫️', text: 'Freezing Fog' },
      51: { icon: '🌦️', text: 'Light Drizzle' },
      53: { icon: '🌦️', text: 'Drizzle' },
      55: { icon: '🌧️', text: 'Heavy Drizzle' },
      61: { icon: '🌧️', text: 'Slight Rain' },
      63: { icon: '🌧️', text: 'Rain' },
      65: { icon: '🌧️', text: 'Heavy Rain' },
      71: { icon: '🌨️', text: 'Light Snow' },
      73: { icon: '🌨️', text: 'Snow' },
      75: { icon: '❄️', text: 'Heavy Snow' },
      79: { icon: '🌦️', text: 'Rain Showers' },
      80: { icon: '🌧️', text: 'Showers' },
      81: { icon: '⛈️', text: 'Heavy Showers' },
      82: { icon: '⛈️', text: 'Heavy Showers' },
      85: { icon: '🌨️', text: 'Snow Showers' },
      86: { icon: '❄️', text: 'Heavy Snow Showers' },
      95: { icon: '⛈️', text: 'Thunderstorm' },
      96: { icon: '⛈️', text: 'Thunderstorm with Hail' },
      99: { icon: '⛈️', text: 'Heavy Thunderstorm' }
    }
  };

  // ==========================================================================
  // 2. Centralized Reactive State
  // ==========================================================================
  const State = {
    map: null,
    currentTileLayer: null,
    currentLayerName: 'osm',
    markers: {},
    markerLayer: null,
    routePolyline: null,
    isRouteActive: false,
    allPins: [],
    lists: [],
    currentListFilter: 'all', // 'all', 'bucket', 'visited', or list_id as string
    selectedCategory: 'All',
    selectedStatus: 'all', // 'all', 'bucket', 'visited'
    selectedTag: null,
    priorityOnly: false,
    openNowOnly: false,
    selectedDay: null,
    searchQuery: '',
    currentSort: 'newest', // 'newest', 'nearest', 'az', 'category'
    currentMobileView: 'map', // 'map' or 'list'
    currentUserLocation: null,
    userLocationMarker: null,
    weatherCache: {}
  };

  // ==========================================================================
  // 3. Utility Helpers
  // ==========================================================================
  const H = window.bListHelpers || {};
  const Utils = {
    escapeHtml(str) {
      if (typeof H.escapeHtml === 'function') return H.escapeHtml(str);
      if (!str) return '';
      return String(str).replace(/[&<>"']/g, (m) => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;'
      })[m]);
    },

    formatFileSize(bytes) {
      if (typeof H.formatFileSize === 'function') return H.formatFileSize(bytes);
      if (!bytes || bytes < 1024) return (bytes || 0) + ' B';
      if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
      return (bytes / 1048576).toFixed(1) + ' MB';
    },

    calculateDistance(lat1, lon1, lat2, lon2) {
      if (typeof H.calculateDistance === 'function') return H.calculateDistance(lat1, lon1, lat2, lon2);
      const R = 6371;
      const dLat = ((lat2 - lat1) * Math.PI) / 180;
      const dLon = ((lon2 - lon1) * Math.PI) / 180;
      const a =
        Math.sin(dLat / 2) * Math.sin(dLat / 2) +
        Math.cos((lat1 * Math.PI) / 180) *
          Math.cos((lat2 * Math.PI) / 180) *
          Math.sin(dLon / 2) *
          Math.sin(dLon / 2);
      return R * (2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a)));
    },

    formatDistance(dKm) {
      if (typeof H.formatDistance === 'function') return H.formatDistance(dKm);
      if (dKm < 1) return `${Math.round(dKm * 1000)} m away`;
      return `${(dKm * 0.621371).toFixed(1)} mi away (${dKm.toFixed(1)} km)`;
    },

    getListNameForPin(pin) {
      if (!pin || !pin.list_id) return null;
      const found = State.lists.find((l) => l.id === pin.list_id);
      return found && found.id !== 1 ? found : null;
    },

    isValidHttpUrl(url) {
      if (typeof H.isValidHttpUrl === 'function') return H.isValidHttpUrl(url);
      if (!url || typeof url !== 'string') return false;
      try {
        const parsed = new URL(url.trim());
        return parsed.protocol === 'http:' || parsed.protocol === 'https:';
      } catch (_) {
        return false;
      }
    },

    sanitizeUrl(url) {
      if (typeof H.sanitizeUrl === 'function') return H.sanitizeUrl(url);
      if (!url || typeof url !== 'string') return '';
      const trimmed = url.trim();
      if (!trimmed) return '';
      const lower = trimmed.toLowerCase().replace(/[\s - ]/g, '');
      if (
        lower.startsWith('javascript:') ||
        lower.startsWith('data:') ||
        lower.startsWith('vbscript:') ||
        lower.startsWith('file:')
      ) {
        return '';
      }
      try {
        const parsed = new URL(trimmed);
        return parsed.protocol === 'http:' || parsed.protocol === 'https:' ? parsed.toString() : '';
      } catch (_) {
        return trimmed.startsWith('/') && !trimmed.startsWith('//') ? trimmed : '';
      }
    }
  };

  // ==========================================================================
  // 4. Toast Notification Manager
  // ==========================================================================
  const ToastManager = {
    show(message, type = 'info', duration = 3000) {
      const container = document.getElementById('toast-container');
      if (!container) return;

      const toast = document.createElement('div');
      const typeClass =
        type === 'error' ? 'toast-error' : type === 'success' ? 'toast-success' : '';
      toast.className = `toast ${typeClass}`.trim();
      toast.innerText = message;

      container.appendChild(toast);
      setTimeout(() => {
        if (toast.parentNode) {
          toast.parentNode.removeChild(toast);
        }
      }, duration);
    }
  };

  // ==========================================================================
  // 4b. Offline Mode & Mutation Sync Manager
  // ==========================================================================
  const OfflineManager = {
    init() {
      this.updateStatus();
      window.addEventListener('online', () => {
        this.updateStatus();
        this.syncQueue();
      });
      window.addEventListener('offline', () => {
        this.updateStatus();
      });
    },

    updateStatus() {
      const isOnline = navigator.onLine;
      const badge = document.getElementById('offline-badge');
      if (badge) {
        badge.classList.toggle('hidden', isOnline);
      }
      if (!isOnline) {
        ToastManager.show('⚡ Offline Mode: Working with cached places & map tiles', 'info');
      }
    },

    getQueue() {
      try {
        return JSON.parse(localStorage.getItem('blist_offline_queue') || '[]');
      } catch (_) {
        return [];
      }
    },

    saveQueue(queue) {
      localStorage.setItem('blist_offline_queue', JSON.stringify(queue));
    },

    enqueue(mutation) {
      const queue = this.getQueue();
      queue.push(mutation);
      this.saveQueue(queue);
    },

    async syncQueue() {
      const queue = this.getQueue();
      if (queue.length === 0) return;

      ToastManager.show(`🔄 Reconnected! Syncing ${queue.length} offline changes...`, 'info');
      const remaining = [];

      for (const item of queue) {
        try {
          if (item.type === 'toggleVisited') {
            await ApiClient.toggleVisited(item.id);
          } else if (item.type === 'createPin') {
            await ApiClient.createPin(item.payload);
          } else if (item.type === 'deletePin') {
            await ApiClient.deletePin(item.id);
          }
        } catch (_) {
          remaining.push(item);
        }
      }

      this.saveQueue(remaining);
      if (remaining.length === 0) {
        ToastManager.show('✨ All offline changes synced with server!', 'success');
        try {
          const freshPins = await ApiClient.fetchPins();
          if (freshPins && freshPins.length > 0) {
            State.allPins = freshPins;
            UIManager.renderAll();
          }
        } catch (_) {}
      }
    }
  };

  // ==========================================================================
  // 5. Theme Management Engine
  // ==========================================================================
  const ThemeManager = {
    init() {
      const savedTheme = localStorage.getItem('blist_theme') || 'auto';
      this.apply(savedTheme, false);

      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handleSystemThemeChange = () => {
        const currentPref = localStorage.getItem('blist_theme') || 'auto';
        if (currentPref === 'auto') {
          this.apply('auto', false);
        }
      };

      if (mediaQuery.addEventListener) {
        mediaQuery.addEventListener('change', handleSystemThemeChange);
      } else if (mediaQuery.addListener) {
        mediaQuery.addListener(handleSystemThemeChange);
      }
    },

    getEffectiveTheme(themeSetting) {
      if (themeSetting === 'dark' || themeSetting === 'light') {
        return themeSetting;
      }
      return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light';
    },

    apply(theme, isUserAction = false) {
      const validThemes = ['light', 'dark', 'auto'];
      if (!validThemes.includes(theme)) theme = 'auto';

      const effectiveTheme = this.getEffectiveTheme(theme);

      document.documentElement.setAttribute('data-theme', theme);
      document.documentElement.setAttribute('data-resolved-theme', effectiveTheme);

      document.querySelectorAll('.theme-opt, .theme-opt-pill').forEach((btn) => {
        btn.classList.toggle('active', btn.dataset.themeVal === theme);
      });

      const themeBtn = document.getElementById('theme-toggle-btn');
      const themeIcon = document.getElementById('theme-btn-icon');
      if (themeBtn) {
        if (theme === 'auto') {
          themeBtn.title = `Theme: Auto (${effectiveTheme === 'dark' ? 'Dark' : 'Light'})`;
          if (themeIcon) themeIcon.setAttribute('data-lucide', 'monitor');
        } else if (theme === 'dark') {
          themeBtn.title = 'Theme: Dark';
          if (themeIcon) themeIcon.setAttribute('data-lucide', 'moon');
        } else {
          themeBtn.title = 'Theme: Light';
          if (themeIcon) themeIcon.setAttribute('data-lucide', 'sun');
        }
      }

      MapController.syncTheme(effectiveTheme, isUserAction);
      if (window.lucide) window.lucide.createIcons();
    },

    set(theme) {
      localStorage.setItem('blist_theme', theme);
      this.apply(theme, true);

      const menu = document.getElementById('theme-menu');
      if (menu) menu.classList.add('hidden');
      const moreMenu = document.getElementById('mobile-more-menu');
      if (moreMenu) moreMenu.classList.add('hidden');

      const toastMsg =
        theme === 'auto'
          ? '💻 Auto theme (matches system)'
          : theme === 'dark'
          ? '🌙 Dark Mode enabled'
          : '☀️ Light Mode enabled';
      ToastManager.show(toastMsg);
    },

    toggle() {
      const current = localStorage.getItem('blist_theme') || 'auto';
      const effective = this.getEffectiveTheme(current);
      const next = effective === 'dark' ? 'light' : 'dark';
      this.set(next);
    },

    toggleMenu() {
      const menu = document.getElementById('theme-menu');
      if (menu) menu.classList.toggle('hidden');
    }
  };

  // ==========================================================================
  // 6. Robust API Client
  // ==========================================================================
  const ApiClient = {
    getUrl(path) {
      if (path.startsWith('http://') || path.startsWith('https://')) return path;
      const base = CONFIG.API_BASE_URL || '';
      return `${base}${path.startsWith('/') ? '' : '/'}${path}`;
    },

    getUserToken() {
      let token = localStorage.getItem('blist_user_token');
      if (!token || !token.trim()) {
        token =
          'usr_' +
          (window.crypto && crypto.randomUUID
            ? crypto.randomUUID().replace(/-/g, '')
            : Math.random().toString(36).substring(2) + Date.now().toString(36));
        localStorage.setItem('blist_user_token', token);
      }
      return token;
    },

    async request(url, options = {}) {
      try {
        const fullUrl = this.getUrl(url);
        const headers = Object.assign(
          {
            'x-user-token': this.getUserToken()
          },
          options.headers || {}
        );
        const reqOptions = Object.assign({}, options, { headers });

        const res = await fetch(fullUrl, reqOptions);
        let data;
        try {
          data = await res.json();
        } catch (_) {
          data = null;
        }

        if (!res.ok) {
          const errMsg =
            (data && data.error) ||
            (data && data.message) ||
            `Server returned HTTP error ${res.status}`;
          throw new Error(errMsg);
        }

        return data;
      } catch (err) {
        if (!navigator.onLine) {
          throw new Error('You appear to be offline. Please check your internet connection.');
        }
        throw err;
      }
    },

    async fetchAppInfo() {
      try {
        const res = await fetch(this.getUrl('/api/info'));
        if (res.ok) {
          return await res.json();
        }
      } catch (_) {}
      return {
        name: 'bList',
        version: '0.1.0',
        repository: 'https://github.com/radmuffin/bList',
        issues_url: 'https://github.com/radmuffin/bList/issues',
        license: 'MIT'
      };
    },

    async fetchLists() {
      try {
        const json = await this.request('/api/lists');
        if (json && json.success && json.data) {
          localStorage.setItem('blist_cached_lists', JSON.stringify(json.data));
          return json.data;
        }
      } catch (err) {
        const cached = localStorage.getItem('blist_cached_lists');
        if (cached) {
          try {
            return JSON.parse(cached);
          } catch (_) {}
        }
      }
      const cached = localStorage.getItem('blist_cached_lists');
      if (cached) {
        try {
          return JSON.parse(cached);
        } catch (_) {}
      }
      return [];
    },

    async createList(payload) {
      return this.request('/api/lists', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    },

    async joinList(share_token) {
      return this.request('/api/lists/join', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ share_token })
      });
    },

    async fetchUserProfile() {
      try {
        const json = await this.request('/api/user/profile');
        if (json && json.success && json.data) {
          return json.data;
        }
      } catch (_) {}
      return null;
    },

    async updateUserProfile(payload) {
      return this.request('/api/user/profile', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    },

    async fetchCollaborators(listId) {
      try {
        const json = await this.request(`/api/lists/${listId}/collaborators`);
        if (json && json.success && json.data) {
          return json.data;
        }
      } catch (_) {}
      return [];
    },

    async fetchPins() {
      try {
        const json = await this.request('/api/pins');
        if (json && json.success && json.data) {
          localStorage.setItem('blist_cached_pins', JSON.stringify(json.data));
          return json.data;
        }
      } catch (err) {
        const cached = localStorage.getItem('blist_cached_pins');
        if (cached) {
          try {
            return JSON.parse(cached);
          } catch (_) {}
        }
      }
      const cached = localStorage.getItem('blist_cached_pins');
      if (cached) {
        try {
          return JSON.parse(cached);
        } catch (_) {}
      }
      return [];
    },

    async createPin(payload) {
      return this.request('/api/pins', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    },

    async updatePin(id, payload) {
      return this.request(`/api/pins/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    },

    async toggleVisited(id) {
      return this.request(`/api/pins/${id}/visited`, {
        method: 'PATCH'
      });
    },

    async deletePin(id) {
      return this.request(`/api/pins/${id}`, {
        method: 'DELETE'
      });
    },

    async ingestPin(url, listId = null) {
      const payload = { url };
      if (listId) payload.list_id = listId;
      return this.request('/api/pins/ingest', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    },

    async importPlaces(payload) {
      return this.request('/api/import', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    },

    async fetchWeather(lat, lon) {
      const key = `${lat.toFixed(2)},${lon.toFixed(2)}`;
      if (State.weatherCache[key]) {
        return State.weatherCache[key];
      }

      try {
        const res = await fetch(
          `https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}&current_weather=true`
        );
        const data = await res.json();
        if (data && data.current_weather) {
          const cw = data.current_weather;
          const codeInfo =
            CONFIG.WEATHER_CODE_MAP[cw.weathercode] || { icon: '🌤️', text: 'Weather' };
          const tempC = Math.round(cw.temperature);
          const tempF = Math.round((cw.temperature * 9) / 5 + 32);

          const weatherInfo = {
            icon: codeInfo.icon,
            text: codeInfo.text,
            tempC,
            tempF,
            display: `${codeInfo.icon} ${tempF}°F / ${tempC}°C`
          };

          State.weatherCache[key] = weatherInfo;
          return weatherInfo;
        }
      } catch (_) {}
      return null;
    },

    async reverseGeocode(lat, lon) {
      try {
        const res = await fetch(
          `https://nominatim.openstreetmap.org/reverse?format=json&lat=${lat}&lon=${lon}`
        );
        return await res.json();
      } catch (_) {
        return null;
      }
    },

    async exportData(format = 'geojson') {
      const res = await fetch(this.getUrl(`/api/export/${format}`));
      return await res.json();
    }
  };

  // ==========================================================================
  // 7. Map Controller & Layer Manager
  // ==========================================================================
  const MapController = {
    init() {
      const isLayerLocked = localStorage.getItem('blist_layer_locked') === 'true';
      const savedLayer = localStorage.getItem('blist_map_layer');
      const effectiveTheme = ThemeManager.getEffectiveTheme(
        localStorage.getItem('blist_theme') || 'auto'
      );

      if (isLayerLocked && savedLayer && CONFIG.MAP_LAYERS[savedLayer]) {
        State.currentLayerName = savedLayer;
      } else {
        State.currentLayerName = effectiveTheme === 'dark' ? 'dark' : 'osm';
      }

      State.map = L.map('map', {
        zoomControl: true,
        tap: true,
        maxZoom: 19,
        doubleClickZoom: false
      }).setView([20.0, 0.0], 2);

      State.markerLayer = L.layerGroup().addTo(State.map);

      this.applyLayer(State.currentLayerName, false);

      let lastModalOpenTime = 0;
      const triggerManualPin = (latlng) => {
        const now = Date.now();
        if (now - lastModalOpenTime < 600) return;
        lastModalOpenTime = now;
        const lat = latlng.lat.toFixed(6);
        const lon = latlng.lng.toFixed(6);
        ModalManager.openManualPinModal(lat, lon);
      };

      // Double-click / double-tap on map to add place manually
      State.map.on('dblclick', (e) => {
        if (e.originalEvent && e.originalEvent.target && e.originalEvent.target.closest && e.originalEvent.target.closest('.leaflet-marker-icon, .leaflet-popup, .leaflet-control')) {
          return;
        }
        triggerManualPin(e.latlng);
      });

      // Tap and hold (long-press) / Right-click on map to add place manually
      State.map.on('contextmenu', (e) => {
        if (e.originalEvent) {
          e.originalEvent.preventDefault();
          if (e.originalEvent.target && e.originalEvent.target.closest && e.originalEvent.target.closest('.leaflet-marker-icon, .leaflet-popup, .leaflet-control')) {
            return;
          }
        }
        triggerManualPin(e.latlng);
      });

      // Touch hold / long-press handler for mobile touchscreens
      let touchTimer = null;
      let touchStartPos = null;
      const mapEl = document.getElementById('map');
      if (mapEl) {
        mapEl.addEventListener('touchstart', (e) => {
          if (e.touches.length !== 1) {
            clearTimeout(touchTimer);
            touchTimer = null;
            return;
          }
          if (e.target && e.target.closest && e.target.closest('.leaflet-marker-icon, .leaflet-popup, .leaflet-control, .fab, .mobile-view-toggle, .bottom-nav')) {
            clearTimeout(touchTimer);
            touchTimer = null;
            return;
          }
          const touch = e.touches[0];
          touchStartPos = { x: touch.clientX, y: touch.clientY };
          clearTimeout(touchTimer);
          touchTimer = setTimeout(() => {
            if (!State.map || !touchStartPos) return;
            const containerRect = mapEl.getBoundingClientRect();
            const point = L.point(
              touchStartPos.x - containerRect.left,
              touchStartPos.y - containerRect.top
            );
            const latlng = State.map.containerPointToLatLng(point);
            if (navigator.vibrate) {
              try { navigator.vibrate(40); } catch (_) {}
            }
            triggerManualPin(latlng);
          }, 500);
        }, { passive: true });

        mapEl.addEventListener('touchmove', (e) => {
          if (!touchTimer || !touchStartPos || e.touches.length !== 1) {
            clearTimeout(touchTimer);
            touchTimer = null;
            return;
          }
          const touch = e.touches[0];
          const dist = Math.hypot(touch.clientX - touchStartPos.x, touch.clientY - touchStartPos.y);
          if (dist > 10) {
            // User is panning or scrolling the map
            clearTimeout(touchTimer);
            touchTimer = null;
          }
        }, { passive: true });

        const cancelTouch = () => {
          clearTimeout(touchTimer);
          touchTimer = null;
          touchStartPos = null;
        };

        mapEl.addEventListener('touchend', cancelTouch, { passive: true });
        mapEl.addEventListener('touchcancel', cancelTouch, { passive: true });
      }
    },

    applyLayer(layerKey, persistManualChoice = true) {
      const layerConf = CONFIG.MAP_LAYERS[layerKey] || CONFIG.MAP_LAYERS.osm;

      if (State.currentTileLayer && State.map) {
        State.map.removeLayer(State.currentTileLayer);
      }

      if (State.map) {
        State.currentTileLayer = L.tileLayer(layerConf.url, layerConf.options).addTo(State.map);
      }
      State.currentLayerName = layerKey;

      if (persistManualChoice) {
        localStorage.setItem('blist_layer_locked', 'true');
        localStorage.setItem('blist_map_layer', layerKey);
      }

      document.querySelectorAll('.layer-opt').forEach((btn) => {
        btn.classList.toggle('active', btn.dataset.layer === layerKey);
      });
    },

    syncTheme(effectiveTheme, isUserAction = false) {
      if (!State.map) return;
      if (isUserAction) {
        // User explicitly toggled theme in UI -> adapt map layer directly
        localStorage.removeItem('blist_layer_locked');
        if (effectiveTheme === 'dark') {
          this.applyLayer('dark', false);
        } else {
          this.applyLayer('osm', false);
        }
      } else {
        const isLayerLocked = localStorage.getItem('blist_layer_locked') === 'true';
        if (!isLayerLocked) {
          if (effectiveTheme === 'dark' && State.currentLayerName === 'osm') {
            this.applyLayer('dark', false);
          } else if (effectiveTheme === 'light' && State.currentLayerName === 'dark') {
            this.applyLayer('osm', false);
          }
        }
      }
    },

    switchLayer(layerKey) {
      this.applyLayer(layerKey, true);
      const menu = document.getElementById('layer-menu');
      if (menu) menu.classList.add('hidden');
    },

    toggleLayerMenu() {
      const menu = document.getElementById('layer-menu');
      if (menu) menu.classList.toggle('hidden');
    },

    renderMarkers() {
      if (!State.map) return;

      if (!State.markerLayer) {
        State.markerLayer = L.layerGroup().addTo(State.map);
      }
      State.markerLayer.clearLayers();
      State.markers = {};

      const filtered = FilterManager.getFilteredPins();
      const markersToAdd = [];

      filtered.forEach((pin) => {
        const emoji = pin.emoji || CONFIG.CATEGORY_EMOJIS[pin.category] || '📍';
        const pinColor = CONFIG.CATEGORY_COLORS[pin.category] || '#2563eb';
        const isPriority = !!pin.priority;
        const isVisited = !!pin.visited;

        const customIcon = L.divIcon({
          className: 'custom-pin-container',
          html: `
            <div class="custom-pin-marker ${isVisited ? 'visited-pin' : ''} ${isPriority ? 'is-priority' : ''}" 
                 style="--pin-color: ${pinColor};" 
                 id="marker-elem-${pin.id}">
              <span class="pin-emoji-inner">${Utils.escapeHtml(emoji)}</span>
            </div>
          `,
          iconSize: [34, 34],
          iconAnchor: [17, 34],
          popupAnchor: [0, -34]
        });

        const marker = L.marker([pin.latitude, pin.longitude], { icon: customIcon });

        marker.on('click', () => {
          this.loadAndRenderPopup(marker, pin);
        });

        State.markers[pin.id] = marker;
        markersToAdd.push(marker);
      });

      markersToAdd.forEach((m) => State.markerLayer.addLayer(m));
      this.updateRouteLine();
    },

    toggleRouteLine() {
      State.isRouteActive = !State.isRouteActive;
      const fab = document.getElementById('route-fab');
      if (fab) fab.classList.toggle('active', State.isRouteActive);

      this.updateRouteLine();
      if (State.isRouteActive) {
        ToastManager.show('🚗 Trip Route enabled! Connecting places in sequence.', 'info');
      } else {
        ToastManager.show('Route view hidden', 'info');
      }
    },

    updateRouteLine() {
      if (!State.map) return;
      if (State.routePolyline) {
        State.map.removeLayer(State.routePolyline);
        State.routePolyline = null;
      }

      const badge = document.getElementById('route-info-badge');
      const textSpan = document.getElementById('route-distance-text');

      if (!State.isRouteActive) {
        if (badge) badge.classList.add('hidden');
        return;
      }

      const filtered = FilterManager.getFilteredPins();
      if (filtered.length < 2) {
        if (badge) {
          badge.classList.remove('hidden');
          if (textSpan) textSpan.textContent = filtered.length === 1 ? '1 place on route' : 'No places to connect';
        }
        return;
      }

      const latLngs = filtered.map((p) => [p.latitude, p.longitude]);

      let totalKm = 0;
      for (let i = 0; i < filtered.length - 1; i++) {
        totalKm += Utils.calculateDistance(
          filtered[i].latitude,
          filtered[i].longitude,
          filtered[i + 1].latitude,
          filtered[i + 1].longitude
        );
      }

      const mi = (totalKm * 0.621371).toFixed(1);
      const km = totalKm.toFixed(1);

      if (badge && textSpan) {
        badge.classList.remove('hidden');
        textSpan.textContent = `🚗 Route (${filtered.length} stops): ${mi} mi (${km} km)`;
      }

      State.routePolyline = L.polyline(latLngs, {
        color: '#2563eb',
        weight: 4,
        opacity: 0.85,
        dashArray: '8, 8',
        lineCap: 'round',
        lineJoin: 'round'
      }).addTo(State.map);
    },

    async loadAndRenderPopup(marker, pin) {
      let distanceStr = '';
      if (State.currentUserLocation) {
        const dKm = Utils.calculateDistance(
          State.currentUserLocation.lat,
          State.currentUserLocation.lng,
          pin.latitude,
          pin.longitude
        );
        distanceStr = Utils.formatDistance(dKm);
      }

      const assignedList = Utils.getListNameForPin(pin);
      const weather = await ApiClient.fetchWeather(pin.latitude, pin.longitude);

      const popupHtml = UIManager.renderPopupHtml(pin, { distanceStr, weather, assignedList });

      marker.bindPopup(popupHtml, { maxWidth: 340, minWidth: 280 }).openPopup();
      if (window.lucide) window.lucide.createIcons();
    },

    flyToPin(id) {
      const pin = State.allPins.find((p) => p.id === id);
      if (!pin || !State.map) return;

      State.map.flyTo([pin.latitude, pin.longitude], 15, { duration: 1.2 });
      if (State.markers[id]) {
        setTimeout(() => {
          this.loadAndRenderPopup(State.markers[id], pin);
        }, 600);
      }
    },

    resetMapView() {
      if (!State.map) return;
      if (State.allPins.length > 0) {
        const validMarkers = Object.values(State.markers);
        if (validMarkers.length > 0) {
          const group = new L.featureGroup(validMarkers);
          State.map.fitBounds(group.getBounds().pad(0.15));
          return;
        }
      }
      State.map.setView([20.0, 0.0], 2);
    },

    updateUserLocationMarker() {
      if (!State.currentUserLocation || !State.map) return;

      if (State.userLocationMarker) {
        State.map.removeLayer(State.userLocationMarker);
      }

      const userIcon = L.divIcon({
        className: 'user-location-marker-container',
        html: `<div class="user-location-pulse"></div>`,
        iconSize: [20, 20],
        iconAnchor: [10, 10]
      });

      State.userLocationMarker = L.marker(
        [State.currentUserLocation.lat, State.currentUserLocation.lng],
        {
          icon: userIcon,
          zIndexOffset: 1000
        }
      ).addTo(State.map);

      State.userLocationMarker.bindPopup('<b>📍 You are here</b>');
    }
  };

  // ==========================================================================
  // 8. Filtering & Sorting Logic
  // ==========================================================================
  const FilterManager = {
    getFilteredPins() {
      const H = window.bListHelpers || {};
      if (typeof H.filterPins === 'function') {
        const filtered = H.filterPins(State.allPins, {
          listFilter: State.currentListFilter,
          status: State.selectedStatus,
          category: State.selectedCategory,
          priorityOnly: State.priorityOnly,
          openNowOnly: State.openNowOnly,
          tag: State.selectedTag,
          dayGroup: State.selectedDay,
          search: State.searchQuery
        });
        if (typeof H.sortPins === 'function') {
          return H.sortPins(filtered, State.currentSort, State.currentUserLocation);
        }
        return filtered;
      }
      return State.allPins;
    },

    selectList(listId) {
      if (listId === '__new_list__') {
        const listSelect = document.getElementById('list-select');
        if (listSelect) {
          listSelect.value = State.currentListFilter;
        }
        ModalManager.openNewListModal();
        return;
      }
      State.currentListFilter = String(listId);
      UIManager.renderAll();
      MapController.resetMapView();
    },

    syncFilterTabsUI() {
      const allTab = document.querySelector('.filter-tab[data-status="all"]');
      const bucketTab = document.querySelector('.filter-tab[data-status="bucket"]');
      const visitedTab = document.querySelector('.filter-tab[data-status="visited"]');
      const priorityTab = document.getElementById('tab-priority-filter');
      const openNowTab = document.getElementById('tab-open-now-filter');

      if (priorityTab) priorityTab.classList.toggle('active', Boolean(State.priorityOnly));
      if (openNowTab) openNowTab.classList.toggle('active', Boolean(State.openNowOnly));
      if (bucketTab) bucketTab.classList.toggle('active', State.selectedStatus === 'bucket');
      if (visitedTab) visitedTab.classList.toggle('active', State.selectedStatus === 'visited');

      const isAllActive = State.selectedStatus === 'all' && !State.priorityOnly && !State.openNowOnly;
      if (allTab) allTab.classList.toggle('active', isAllActive);
    },

    setStatusFilter(status) {
      if (status === 'all') {
        State.selectedStatus = 'all';
        State.priorityOnly = false;
        State.openNowOnly = false;
      } else {
        State.selectedStatus = status;
      }
      this.syncFilterTabsUI();
      UIManager.renderPinList();
      MapController.renderMarkers();
      UIManager.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    togglePriorityFilter() {
      State.priorityOnly = !State.priorityOnly;
      this.syncFilterTabsUI();
      UIManager.renderPinList();
      MapController.renderMarkers();
      UIManager.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    toggleOpenNowFilter() {
      State.openNowOnly = !State.openNowOnly;
      this.syncFilterTabsUI();
      UIManager.renderPinList();
      MapController.renderMarkers();
      UIManager.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    setTagFilter(tag) {
      State.selectedTag = State.selectedTag === tag ? null : tag;
      UIManager.renderTagsBar();
      UIManager.renderPinList();
      MapController.renderMarkers();
      UIManager.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    setDayFilter(day) {
      State.selectedDay = State.selectedDay === day ? null : day;
      UIManager.renderDaysBar();
      UIManager.renderPinList();
      MapController.renderMarkers();
      UIManager.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    setCategoryFilter(cat) {
      State.selectedCategory = cat;
      UIManager.renderCategories();
      UIManager.renderPinList();
      MapController.renderMarkers();
      UIManager.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    handleSearch(query) {
      State.searchQuery = query;
      UIManager.renderPinList();
      MapController.renderMarkers();
      UIManager.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    handleSortChange(sortVal) {
      State.currentSort = sortVal;
      if (State.currentSort === 'nearest' && !State.currentUserLocation) {
        FeatureActions.locateUser();
      }
      UIManager.renderPinList();
      if (window.lucide) window.lucide.createIcons();
    }
  };

  // ==========================================================================
  // 9. UI & Component Rendering Engine
  // ==========================================================================
  const UIManager = {
    renderAll() {
      this.renderListsUI();
      this.renderCategories();
      this.renderTagsBar();
      this.renderDaysBar();
      this.renderPinList();
      MapController.renderMarkers();
      this.updateCounts();
      this.updateTripProgress();
      if (typeof ProfileManager !== 'undefined' && ProfileManager.renderCollaboratorsForCurrentList) {
        ProfileManager.renderCollaboratorsForCurrentList();
      }
      if (window.lucide) window.lucide.createIcons();
    },

    renderTagsBar() {
      const container = document.getElementById('tags-bar');
      if (!container) return;

      const tagsSet = new Set();
      State.allPins.forEach((pin) => {
        if (pin.tags) {
          const parts = pin.tags.split(/[\s,]+/);
          parts.forEach((t) => {
            const clean = t.trim().replace(/^#/, '');
            if (clean) tagsSet.add(clean);
          });
        }
      });

      if (tagsSet.size === 0) {
        container.classList.add('hidden');
        return;
      }

      container.classList.remove('hidden');
      const tags = Array.from(tagsSet).sort();

      container.innerHTML = `
        <button class="tag-chip ${!State.selectedTag ? 'active' : ''}" onclick="setTagFilter(null)">#all</button>
        ${tags
          .map(
            (tag) => `
          <button class="tag-chip ${State.selectedTag === tag ? 'active' : ''}" data-tag="${Utils.escapeHtml(tag)}" onclick="setTagFilter(this.dataset.tag)">
            #${Utils.escapeHtml(tag)}
          </button>
        `
          )
          .join('')}
      `;
    },

    renderDaysBar() {
      const container = document.getElementById('days-bar');
      if (!container) return;

      const hasDays = State.allPins.some((p) => p.day_group && p.day_group > 0);
      if (!hasDays) {
        container.classList.add('hidden');
        return;
      }

      container.classList.remove('hidden');
      const activeDays = [1, 2, 3, 4, 5, 6, 7].filter((d) => State.allPins.some((p) => p.day_group === d));

      container.innerHTML = `
        <button class="day-pill ${State.selectedDay === null ? 'active' : ''}" onclick="setDayFilter(null)">All Days</button>
        ${activeDays
          .map(
            (day) => `
          <button class="day-pill ${State.selectedDay === day ? 'active' : ''}" onclick="setDayFilter(${day})">
            Day ${day}
          </button>
        `
          )
          .join('')}
      `;
    },

    updateTripProgress() {
      const card = document.getElementById('trip-progress-card');
      const label = document.getElementById('trip-progress-label');
      const stats = document.getElementById('trip-progress-stats');
      const bar = document.getElementById('trip-progress-bar');
      if (!card || !label || !stats || !bar) return;

      let activePins = State.allPins;
      let listTitle = 'Bucket List Progress';

      if (State.currentListFilter !== 'all' && State.currentListFilter !== 'bucket' && State.currentListFilter !== 'visited') {
        const listId = parseInt(State.currentListFilter, 10);
        const list = State.lists.find((l) => l.id === listId);
        if (list) {
          activePins = State.allPins.filter((p) => p.list_id === listId);
          listTitle = `${list.icon || '📍'} ${list.name}`;
        }
      }

      const total = activePins.length;
      const visited = activePins.filter((p) => p.visited === 1 || p.visited === true).length;
      const percentage = total > 0 ? Math.round((visited / total) * 100) : 0;

      label.textContent = listTitle;
      if (total === 0) {
        stats.textContent = '0 places';
        card.classList.remove('trip-completed');
      } else if (visited === total && total >= 2) {
        stats.textContent = `🏆 ${visited}/${total} Visited (100%) • List Complete! 🎉`;
        card.classList.add('trip-completed');
      } else if (visited === total && total === 1) {
        stats.textContent = `🏆 1/1 Visited (100%) 🎉`;
        card.classList.remove('trip-completed');
      } else {
        stats.textContent = `${visited} / ${total} visited (${percentage}%)`;
        card.classList.remove('trip-completed');
      }
      bar.style.width = `${percentage}%`;
    },

    renderBadgesHtml(pin, { distanceStr, weather, assignedList }) {
      const emoji = pin.emoji || CONFIG.CATEGORY_EMOJIS[pin.category] || '';
      let tagsHtml = '';
      if (pin.tags) {
        const parts = pin.tags.split(/[\s,]+/);
        tagsHtml = parts
          .filter((t) => t.trim().length > 0)
          .map((t) => {
            const clean = t.trim().replace(/^#/, '');
            return `<span class="pin-badge badge-tag" data-tag="${Utils.escapeHtml(clean)}" onclick="event.stopPropagation(); setTagFilter(this.dataset.tag)">#${Utils.escapeHtml(clean)}</span>`;
          })
          .join('');
      }

      let hoursHtml = '';
      if (pin.opening_hours) {
        const getStatusFn = (window.bListHelpers && window.bListHelpers.getOpeningStatus) ||
                            (window.Helpers && window.Helpers.getOpeningStatus);
        if (getStatusFn) {
          const op = getStatusFn(pin.opening_hours);
          if (op && op.label) {
            hoursHtml = `<span class="hours-status-badge ${op.badgeClass}" title="${Utils.escapeHtml(op.details)}"><span class="hours-status-dot"></span>${Utils.escapeHtml(op.label)}</span>`;
          }
        }
      }

      return `
        ${pin.priority ? '<span class="pin-badge badge-priority">⭐ Must-See</span>' : ''}
        ${pin.day_group && pin.day_group > 0 ? `<span class="pin-badge badge-day">📅 Day ${pin.day_group}</span>` : ''}
        ${hoursHtml}
        <span class="pin-badge ${pin.visited ? 'badge-visited' : ''}">
          ${pin.visited ? '✅ Visited' : `${emoji} ${Utils.escapeHtml(pin.category || 'Place')}`}
        </span>
        ${
          assignedList
            ? `<span class="pin-badge badge-list">${assignedList.icon || '📁'} ${Utils.escapeHtml(
                assignedList.name
              )}</span>`
            : ''
        }
        ${tagsHtml}
        ${distanceStr ? `<span class="pin-badge badge-distance">📍 ${distanceStr}</span>` : ''}
        <span class="pin-badge badge-weather ${weather ? '' : 'hidden'}" id="weather-badge-${pin.id}">
          ${weather ? `${weather.icon} ${weather.tempF}°F` : ''}
        </span>
      `;
    },

    renderActionsHtml(pin, isPopup = false) {
      const directionsUrl = `https://www.google.com/maps/dir/?api=1&destination=${pin.latitude},${pin.longitude}`;
      const safeSourceUrl = pin.source_url ? Utils.sanitizeUrl(pin.source_url) : '';

      return `
        <div class="${isPopup ? 'popup-footer' : 'pin-card-footer'}" onclick="event.stopPropagation()">
          <button type="button" class="btn-card-status-pill ${pin.visited ? 'is-visited' : ''}" onclick="toggleVisited(${pin.id})" title="${pin.visited ? 'Mark as to visit' : 'Mark as visited'}">
            <i data-lucide="${pin.visited ? 'check-circle-2' : 'circle'}"></i>
            <span>${pin.visited ? 'Visited' : 'Bucket List'}</span>
          </button>

          <div class="card-action-btns">
            <a href="${directionsUrl}" target="_blank" rel="noopener noreferrer" class="btn-card-directions" title="Get Directions">
              <i data-lucide="navigation"></i>
              <span>Directions</span>
            </a>
            ${
              safeSourceUrl
                ? `<a href="${Utils.escapeHtml(safeSourceUrl)}" target="_blank" rel="noopener noreferrer" class="btn-icon-sm" title="Original Link">
                    <i data-lucide="external-link" style="width: 14px; height: 14px;"></i>
                   </a>`
                : ''
            }
            <button type="button" class="btn-icon-sm" onclick="sharePin(${pin.id})" title="Share Place">
              <i data-lucide="share-2" style="width: 14px; height: 14px;"></i>
            </button>
            <button type="button" class="btn-icon-sm" onclick="openEditPinModal(${pin.id})" title="Edit Place">
              <i data-lucide="edit-3" style="width: 14px; height: 14px;"></i>
            </button>
            <button type="button" class="btn-icon-sm delete-btn" onclick="deletePin(${pin.id})" title="Delete Place">
              <i data-lucide="trash-2" style="width: 14px; height: 14px;"></i>
            </button>
          </div>
        </div>
      `;
    },

    renderPopupHtml(pin, { distanceStr, weather, assignedList }) {
      const safeImg = pin.image_url ? Utils.sanitizeUrl(pin.image_url) : '';
      const rawPlusCode = (window.bListHelpers && window.bListHelpers.encodePlusCode)
        ? window.bListHelpers.encodePlusCode(pin.latitude, pin.longitude)
        : '';
      const displayPlusCode = (window.bListHelpers && window.bListHelpers.formatDisplayPlusCode)
        ? window.bListHelpers.formatDisplayPlusCode(rawPlusCode, pin.address)
        : rawPlusCode;
      const streetAddress = (window.bListHelpers && window.bListHelpers.formatStreetAddress)
        ? window.bListHelpers.formatStreetAddress(pin.address, pin.title)
        : (pin.address || '');

      return `
        <div class="pin-popup">
          ${
            safeImg
              ? `<img src="${Utils.escapeHtml(safeImg)}" class="popup-img" alt="${Utils.escapeHtml(
                  pin.title
                )}" onerror="this.style.display='none'">`
              : ''
          }
          <div class="popup-body">
            <div class="popup-badges-row">
              ${this.renderBadgesHtml(pin, { distanceStr, weather, assignedList })}
            </div>
            
            <div class="popup-title">${Utils.escapeHtml(pin.title)}</div>
            ${
              streetAddress
                ? `<div class="popup-address"><i data-lucide="map-pin" style="width: 12px; height: 12px;"></i> ${Utils.escapeHtml(
                    streetAddress
                  )}</div>`
                : ''
            }
            ${
              displayPlusCode
                ? `<div class="popup-plus-code-row" data-plus-code="${Utils.escapeHtml(
                    displayPlusCode
                  )}" onclick="event.stopPropagation(); copyPlusCode(this.dataset.plusCode)" title="Click to copy Plus Code (${Utils.escapeHtml(displayPlusCode)})">
                    <span class="plus-code-label"><i data-lucide="compass"></i></span>
                    <code class="plus-code-val">${Utils.escapeHtml(displayPlusCode)}</code>
                    <span class="plus-code-copy-btn"><i data-lucide="copy"></i></span>
                  </div>`
                : ''
            }
            ${pin.notes ? `<div class="popup-notes">"${Utils.escapeHtml(pin.notes)}"</div>` : ''}

            ${this.renderActionsHtml(pin, true)}
          </div>
        </div>
      `;
    },

    renderListsUI() {
      const listSelect = document.getElementById('list-select');
      const formListSelect = document.getElementById('form-list-id');

      if (listSelect) {
        const currentVal = State.currentListFilter;
        let optionsHtml = `
          <option value="all" ${currentVal === 'all' ? 'selected' : ''}>📁 All Places (${State.allPins.length})</option>
        `;

        State.lists.forEach((l) => {
          const count = State.allPins.filter((p) => p.list_id === l.id).length;
          optionsHtml += `
            <option value="${l.id}" ${currentVal === String(l.id) ? 'selected' : ''}>
              ${l.icon || '📍'} ${Utils.escapeHtml(l.name)} (${count})
            </option>
          `;
        });

        optionsHtml += `
          <option value="__new_list__">➕ Create New List / Trip...</option>
        `;

        listSelect.innerHTML = optionsHtml;
      }

      if (formListSelect) {
        const currentVal = formListSelect.value;
        formListSelect.innerHTML = State.lists
          .map(
            (l) => `
          <option value="${l.id}">${l.icon || '📍'} ${Utils.escapeHtml(l.name)}</option>
        `
          )
          .join('');

        if (currentVal && State.lists.some((l) => String(l.id) === currentVal)) {
          formListSelect.value = currentVal;
        }
      }
    },

    renderCategories() {
      const container = document.getElementById('categories-bar');
      if (!container) return;

      const rawCategories = [...new Set(State.allPins.map((p) => p.category).filter(Boolean))];
      if (rawCategories.length <= 1) {
        container.style.display = 'none';
        return;
      }
      container.style.display = 'flex';

      const categories = ['All', ...rawCategories];

      container.innerHTML = categories
        .map(
          (cat) => `
        <button class="cat-chip ${State.selectedCategory === cat ? 'active' : ''}" data-category="${Utils.escapeHtml(
            cat
          )}">
          ${Utils.escapeHtml(cat)}
        </button>
      `
        )
        .join('');

      if (!container._hasCategoryListener) {
        container.addEventListener('click', (e) => {
          const btn = e.target.closest('.cat-chip');
          if (btn && btn.dataset.category) {
            FilterManager.setCategoryFilter(btn.dataset.category);
          }
        });
        container._hasCategoryListener = true;
      }
    },

    renderPinList() {
      const container = document.getElementById('pin-list');
      if (!container) return;

      const filtered = FilterManager.getFilteredPins();

      if (filtered.length === 0) {
        container.innerHTML = `
          <div class="empty-state">
            <div class="empty-icon">📍</div>
            <h3>No places found</h3>
            <p>Try clearing your search query, or selecting another trip/category filter.</p>
          </div>
        `;
        return;
      }

      container.innerHTML = filtered
        .map((pin) => {
          let distanceStr = '';
          if (State.currentUserLocation) {
            const dKm = Utils.calculateDistance(
              State.currentUserLocation.lat,
              State.currentUserLocation.lng,
              pin.latitude,
              pin.longitude
            );
            distanceStr = Utils.formatDistance(dKm);
          }

          const assignedList = Utils.getListNameForPin(pin);
          const weatherCached =
            State.weatherCache[`${pin.latitude.toFixed(2)},${pin.longitude.toFixed(2)}`];
          const safeThumb = pin.image_url ? Utils.sanitizeUrl(pin.image_url) : '';
          const streetAddress = (window.bListHelpers && window.bListHelpers.formatStreetAddress)
            ? window.bListHelpers.formatStreetAddress(pin.address, pin.title)
            : (pin.address || '');

          return `
                  <div class="pin-card ${pin.visited ? 'visited-card' : ''}" onclick="handlePinCardClick(${pin.id})" id="card-pin-${pin.id}">
                    ${
                      safeThumb
                        ? `<img src="${Utils.escapeHtml(safeThumb)}" class="pin-card-thumb" alt="${Utils.escapeHtml(
                            pin.title
                          )}" onerror="this.style.display='none'">`
                        : ''
                    }
                    <div class="pin-card-body">
                      <div class="badges-row">
                        ${this.renderBadgesHtml(pin, { distanceStr, weather: weatherCached, assignedList })}
                      </div>
                      <div class="pin-card-title">${Utils.escapeHtml(pin.title)}</div>
                      ${
                        streetAddress
                          ? `<div class="pin-card-address">
                              <i data-lucide="map-pin" style="width: 12px; height: 12px; flex-shrink: 0;"></i>
                              <span>${Utils.escapeHtml(streetAddress)}</span>
                            </div>`
                          : ''
                      }
                      ${
                        pin.notes
                          ? `<div class="pin-card-notes">
                              "${Utils.escapeHtml(pin.notes)}"
                            </div>`
                          : ''
                      }
                      ${this.renderActionsHtml(pin, false)}
                    </div>
                  </div>
                `;
        })
        .join('');
    },

    updateCounts() {
      const total = State.allPins.length;
      const visited = State.allPins.filter((p) => p.visited).length;
      const bucket = total - visited;

      const elAll = document.getElementById('count-all');
      const elBucket = document.getElementById('count-bucket');
      const elVisited = document.getElementById('count-visited');
      const elMobile = document.getElementById('mobile-count');

      if (elAll) elAll.innerText = total;
      if (elBucket) elBucket.innerText = bucket;
      if (elVisited) elVisited.innerText = visited;
      if (elMobile) elMobile.innerText = FilterManager.getFilteredPins().length;
    },

    preloadWeatherForVisiblePins() {
      const pins = FilterManager.getFilteredPins().slice(0, 20);
      pins.forEach((pin) => {
        ApiClient.fetchWeather(pin.latitude, pin.longitude).then((w) => {
          if (w) {
            const badge = document.getElementById(`weather-badge-${pin.id}`);
            if (badge) {
              badge.innerText = `${w.icon} ${w.tempF}°F`;
              badge.classList.remove('hidden');
            }
          }
        });
      });
    },

    toggleSidebar() {
      const sidebar = document.getElementById('sidebar');
      if (!sidebar) return;

      if (window.innerWidth <= 768) {
        if (sidebar.classList.contains('mobile-open')) {
          this.showMobileView('map');
        } else {
          this.showMobileView('list');
        }
      } else {
        sidebar.classList.toggle('collapsed');
        setTimeout(() => State.map && State.map.invalidateSize(), 300);
      }
    },

    showMobileView(view) {
      State.currentMobileView = view;
      const sidebar = document.getElementById('sidebar');
      const backdrop = document.getElementById('sidebar-backdrop');
      const btnMap = document.getElementById('btn-show-map');
      const btnList = document.getElementById('btn-show-list');
      const headerAddBtn = document.getElementById('btn-add-place');

      if (view === 'list') {
        if (sidebar) sidebar.classList.add('mobile-open');
        if (backdrop) backdrop.classList.add('active');
        if (btnMap) btnMap.classList.remove('active');
        if (btnList) btnList.classList.add('active');
        if (headerAddBtn) {
          const textSpan = headerAddBtn.querySelector('.btn-text');
          if (textSpan) textSpan.textContent = 'New List';
          headerAddBtn.title = 'Create New List or Trip';
          headerAddBtn.setAttribute('aria-label', 'Create New List or Trip');
        }
      } else {
        if (sidebar) sidebar.classList.remove('mobile-open');
        if (backdrop) backdrop.classList.remove('active');
        if (btnMap) btnMap.classList.add('active');
        if (btnList) btnList.classList.remove('active');
        if (headerAddBtn) {
          const textSpan = headerAddBtn.querySelector('.btn-text');
          if (textSpan) textSpan.textContent = 'Add';
          headerAddBtn.title = 'Add Place';
          headerAddBtn.setAttribute('aria-label', 'Add Place');
        }
        setTimeout(() => State.map && State.map.invalidateSize(), 150);
      }
    },

    toggleMobileQuickAdd(forceOpen) {
      const bar = document.getElementById('header-mobile-bar');
      if (!bar) return;
      const shouldOpen = forceOpen !== undefined ? forceOpen : !bar.classList.contains('expanded');
      if (shouldOpen) {
        bar.classList.add('expanded');
        const input = document.getElementById('save-url-input-mobile');
        if (input) {
          setTimeout(() => input.focus(), 150);
        }
      } else {
        bar.classList.remove('expanded');
      }
    },

    closeMobileQuickAdd() {
      const bar = document.getElementById('header-mobile-bar');
      if (bar) bar.classList.remove('expanded');
    },

    handleHeaderAddClick() {
      if (State.currentMobileView === 'list') {
        ModalManager.openNewListModal();
      } else if (window.innerWidth <= 768) {
        this.toggleMobileQuickAdd();
      } else {
        ModalManager.openManualPinModal();
      }
    }
  };

  // ==========================================================================
  // 10. Modal Dialog Manager
  // ==========================================================================
  const ModalManager = {
    async openManualPinModal(lat = '', lon = '') {
      UIManager.closeMobileQuickAdd();
      document.getElementById('modal-title').innerText = 'Add Place';
      document.getElementById('form-pin-id').value = '';
      document.getElementById('form-title').value = '';
      document.getElementById('form-emoji').value = '';
      document.getElementById('form-tags').value = '';
      document.getElementById('form-priority').checked = false;
      document.getElementById('form-day-group').value = '0';
      document.getElementById('form-opening-hours').value = '';
      document.getElementById('form-lat').value = lat;
      document.getElementById('form-lon').value = lon;
      document.getElementById('form-category').value = 'Place';
      document.getElementById('form-visited').value = 'false';
      document.getElementById('form-address').value = '';
      document.getElementById('form-image').value = '';
      document.getElementById('form-source').value = '';
      document.getElementById('form-notes').value = '';

      const formList = document.getElementById('form-list-id');
      if (formList) {
        if (
          State.currentListFilter !== 'all' &&
          State.currentListFilter !== 'bucket' &&
          State.currentListFilter !== 'visited'
        ) {
          formList.value = State.currentListFilter;
        } else if (State.lists.length > 0) {
          formList.value = State.lists[0].id;
        }
      }

      const moreOptions = document.getElementById('pin-more-options');
      if (moreOptions) moreOptions.open = false;

      const submitBtn = document.getElementById('btn-submit-pin');
      if (submitBtn) {
        submitBtn.disabled = false;
        submitBtn.innerText = 'Save Place';
      }

      if (lat && lon) {
        const geoData = await ApiClient.reverseGeocode(lat, lon);
        if (geoData && geoData.display_name) {
          document.getElementById('form-address').value = geoData.display_name;
          if (!document.getElementById('form-title').value) {
            const parts = geoData.display_name.split(',');
            document.getElementById('form-title').value = parts[0].trim();
          }
        }
      }

      const deleteBtn = document.getElementById('btn-delete-pin-modal');
      if (deleteBtn) deleteBtn.classList.add('hidden');

      this.updateModalPlusCodePreview();
      if (window.lucide) window.lucide.createIcons();
      document.getElementById('pin-modal').classList.remove('hidden');
    },

    openEditPinModal(id) {
      const pin = State.allPins.find((p) => p.id === id);
      if (!pin) return;

      document.getElementById('modal-title').innerText = 'Edit Place';
      document.getElementById('form-pin-id').value = pin.id;
      document.getElementById('form-title').value = pin.title;
      document.getElementById('form-emoji').value = pin.emoji || '';
      document.getElementById('form-tags').value = pin.tags || '';
      document.getElementById('form-priority').checked = Boolean(pin.priority);
      document.getElementById('form-day-group').value = String(pin.day_group || 0);
      document.getElementById('form-opening-hours').value = pin.opening_hours || '';
      document.getElementById('form-lat').value = pin.latitude;
      document.getElementById('form-lon').value = pin.longitude;
      document.getElementById('form-category').value = pin.category || 'Place';
      document.getElementById('form-visited').value = pin.visited ? 'true' : 'false';
      document.getElementById('form-address').value = pin.address || '';
      document.getElementById('form-image').value = pin.image_url || '';
      document.getElementById('form-source').value = pin.source_url || '';
      document.getElementById('form-notes').value = pin.notes || '';

      const moreOptions = document.getElementById('pin-more-options');
      if (moreOptions) {
        moreOptions.open = Boolean(
          pin.emoji ||
          pin.tags ||
          pin.priority ||
          (pin.day_group && parseInt(pin.day_group, 10) > 0) ||
          pin.opening_hours ||
          pin.image_url ||
          pin.source_url ||
          pin.notes
        );
      }

      const submitBtn = document.getElementById('btn-submit-pin');
      if (submitBtn) {
        submitBtn.disabled = false;
        submitBtn.innerText = 'Update Place';
      }

      const deleteBtn = document.getElementById('btn-delete-pin-modal');
      if (deleteBtn) deleteBtn.classList.remove('hidden');

      const formList = document.getElementById('form-list-id');
      if (formList) {
        formList.value = pin.list_id || (State.lists[0] ? State.lists[0].id : 1);
      }

      this.updateModalPlusCodePreview();
      if (window.lucide) window.lucide.createIcons();
      document.getElementById('pin-modal').classList.remove('hidden');
    },

    handleDeleteFromPinModal() {
      const idVal = document.getElementById('form-pin-id').value;
      const pinId = parseInt(idVal, 10);
      if (!pinId) return;
      this.closePinModal();
      FeatureActions.deletePin(pinId);
    },

    updateModalPlusCodePreview() {
      const latVal = parseFloat(document.getElementById('form-lat')?.value);
      const lonVal = parseFloat(document.getElementById('form-lon')?.value);
      const addressVal = document.getElementById('form-address')?.value || '';
      const plusCodeDisplay = document.getElementById('form-plus-code-display');
      const plusCodeVal = document.getElementById('form-plus-code-val');

      if (plusCodeDisplay && plusCodeVal) {
        if (!isNaN(latVal) && !isNaN(lonVal) && window.bListHelpers) {
          const rawCode = window.bListHelpers.encodePlusCode(latVal, lonVal);
          const displayCode = window.bListHelpers.formatDisplayPlusCode(rawCode, addressVal);
          if (displayCode) {
            plusCodeVal.textContent = displayCode;
            plusCodeDisplay.classList.remove('hidden');
            return;
          }
        }
        plusCodeDisplay.classList.add('hidden');
      }
    },

    closePinModal() {
      const modal = document.getElementById('pin-modal');
      if (modal) modal.classList.add('hidden');
    },

    async handlePinFormSubmit(e) {
      e.preventDefault();
      const id = document.getElementById('form-pin-id').value;
      const selectedListId =
        parseInt(document.getElementById('form-list-id').value, 10) || 1;
      const submitBtn = document.getElementById('btn-submit-pin');

      const title = document.getElementById('form-title').value.trim();
      const address = document.getElementById('form-address').value.trim();
      let lat = parseFloat(document.getElementById('form-lat').value);
      let lon = parseFloat(document.getElementById('form-lon').value);

      if (!title) {
        ToastManager.show('Please enter a place name.', 'error');
        return;
      }

      // Auto-geocode if coordinates were not filled
      if (isNaN(lat) || isNaN(lon)) {
        const query = address || title;
        if (query) {
          if (submitBtn) {
            submitBtn.disabled = true;
            submitBtn.innerText = 'Finding location...';
          }
          try {
            const geoRes = await ApiClient.request(`/api/geocode?q=${encodeURIComponent(query)}`);
            if (geoRes && geoRes.success && geoRes.data) {
              lat = geoRes.data.latitude;
              lon = geoRes.data.longitude;
              document.getElementById('form-lat').value = lat;
              document.getElementById('form-lon').value = lon;
              if (!address && geoRes.data.display_name) {
                document.getElementById('form-address').value = geoRes.data.display_name;
              }
            } else {
              ToastManager.show('Could not find location automatically. Please enter coordinates in More Options.', 'error');
              if (submitBtn) {
                submitBtn.disabled = false;
                submitBtn.innerText = id ? 'Update Place' : 'Save Place';
              }
              const moreOptions = document.getElementById('pin-more-options');
              if (moreOptions) moreOptions.open = true;
              return;
            }
          } catch (err) {
            ToastManager.show('Could not locate address. Please check coordinates in More Options.', 'error');
            if (submitBtn) {
              submitBtn.disabled = false;
              submitBtn.innerText = id ? 'Update Place' : 'Save Place';
            }
            return;
          }
        } else {
          ToastManager.show('Please provide a place name or coordinates.', 'error');
          return;
        }
      }

      const payload = {
        list_id: selectedListId,
        title,
        emoji: document.getElementById('form-emoji')?.value.trim() || null,
        tags: document.getElementById('form-tags')?.value.trim() || null,
        priority: document.getElementById('form-priority')?.checked || false,
        day_group: parseInt(document.getElementById('form-day-group')?.value, 10) || 0,
        opening_hours: document.getElementById('form-opening-hours')?.value.trim() || null,
        latitude: lat,
        longitude: lon,
        category: document.getElementById('form-category').value,
        visited: document.getElementById('form-visited').value === 'true',
        address: address || null,
        image_url: document.getElementById('form-image').value.trim() || null,
        source_url: document.getElementById('form-source').value.trim() || null,
        notes: document.getElementById('form-notes').value.trim() || null
      };

      if (submitBtn) {
        submitBtn.disabled = true;
        submitBtn.innerText = 'Saving...';
      }

      try {
        let json;
        if (id) {
          json = await ApiClient.updatePin(id, payload);
        } else {
          json = await ApiClient.createPin(payload);
        }

        if (json.success && json.data) {
          this.closePinModal();
          const savedPin = json.data;

          if (id) {
            const idx = State.allPins.findIndex((p) => p.id === parseInt(id, 10));
            if (idx !== -1) State.allPins[idx] = savedPin;
          } else {
            State.allPins.unshift(savedPin);
          }

          UIManager.renderAll();
          ToastManager.show(id ? 'Place updated!' : 'Place added!', 'success');

          if (window.innerWidth <= 768) {
            UIManager.showMobileView('map');
          }
          MapController.flyToPin(savedPin.id);
        } else {
          ToastManager.show(json.error || 'Failed to save place', 'error');
        }
      } catch (err) {
        ToastManager.show(err.message || 'Error connecting to server', 'error');
      } finally {
        if (submitBtn) {
          submitBtn.disabled = false;
          submitBtn.innerText = id ? 'Update Place' : 'Save Place';
        }
      }
    },

    openNewListModal() {
      const modal = document.getElementById('new-list-modal');
      const nameInput = document.getElementById('new-list-name');
      if (nameInput) nameInput.value = '';
      this.selectListIcon('📍');
      if (modal) modal.classList.remove('hidden');
    },

    closeNewListModal() {
      const modal = document.getElementById('new-list-modal');
      if (modal) modal.classList.add('hidden');
    },

    selectListIcon(emoji) {
      const input = document.getElementById('new-list-icon');
      if (input) input.value = emoji;
      document.querySelectorAll('#emoji-picker .emoji-opt').forEach((btn) => {
        btn.classList.toggle('active', btn.innerText.trim() === emoji);
      });
    },

    async handleCreateListSubmit(e) {
      e.preventDefault();
      const nameInput = document.getElementById('new-list-name');
      const iconInput = document.getElementById('new-list-icon');
      const name = nameInput ? nameInput.value.trim() : '';
      const icon = iconInput ? iconInput.value || '📍' : '📍';

      if (!name) return;

      try {
        const json = await ApiClient.createList({ name, icon });
        if (json.success && json.data) {
          State.lists.push(json.data);
          this.closeNewListModal();
          FilterManager.selectList(json.data.id);
          ToastManager.show(`Created trip "${icon} ${name}"!`, 'success');
        } else {
          ToastManager.show(json.error || 'Failed to create list', 'error');
        }
      } catch (err) {
        ToastManager.show(err.message || 'Failed to connect to server', 'error');
      }
    },

    openShareListModal() {
      const modal = document.getElementById('share-list-modal');
      const shareInput = document.getElementById('share-link-input');
      const bannerTitle = document.getElementById('share-list-banner-title');
      const bannerIcon = document.getElementById('share-list-banner-icon');

      let targetList;
      if (
        State.currentListFilter !== 'all' &&
        State.currentListFilter !== 'bucket' &&
        State.currentListFilter !== 'visited'
      ) {
        targetList = State.lists.find((l) => String(l.id) === String(State.currentListFilter));
      } else if (State.lists.length > 0) {
        targetList = State.lists[0];
      }

      if (!targetList) {
        ToastManager.show('Please select or create a trip to share.', 'info');
        return;
      }

      this.currentShareList = targetList;
      if (bannerTitle) bannerTitle.textContent = targetList.name;
      if (bannerIcon) bannerIcon.textContent = targetList.icon || '📍';

      const joinUrl = `${window.location.origin}/?join=${encodeURIComponent(targetList.share_token)}`;
      if (shareInput) {
        shareInput.value = joinUrl;
      }

      // Hide QR code by default until requested
      const qrBox = document.getElementById('share-list-qr-box');
      const qrLabel = document.getElementById('qr-list-toggle-label');
      if (qrBox) qrBox.classList.add('hidden');
      if (qrLabel) qrLabel.textContent = 'Show QR Code for Phone Scan';

      if (modal) modal.classList.remove('hidden');
    },

    closeShareListModal() {
      const modal = document.getElementById('share-list-modal');
      if (modal) modal.classList.add('hidden');
    },

    async copyShareLink() {
      const shareInput = document.getElementById('share-link-input');
      if (!shareInput || !shareInput.value) return;

      if (navigator.clipboard && navigator.clipboard.writeText) {
        try {
          await navigator.clipboard.writeText(shareInput.value);
          ToastManager.show('📋 Collaboration link copied to clipboard!', 'success');
          return;
        } catch (_) {}
      }

      shareInput.select();
      document.execCommand('copy');
      ToastManager.show('📋 Link copied to clipboard!', 'success');
    },

    shareListVia(platform) {
      const list = this.currentShareList || State.lists[0];
      if (!list) return;
      const joinUrl = `${window.location.origin}/?join=${encodeURIComponent(list.share_token)}`;
      const title = `Join my "${list.icon} ${list.name}" travel list on bList!`;
      const text = `Join my trip collection "${list.icon} ${list.name}" on bList and collaborate with me:\n${joinUrl}`;

      const links = window.bListHelpers && typeof window.bListHelpers.generateShareLinks === 'function'
        ? window.bListHelpers.generateShareLinks(joinUrl, {
            title,
            text,
            smsText: `Join my trip collection "${list.icon} ${list.name}" on bList: ${joinUrl}`,
            emailBody: `Hey!\n\nI'd love for you to collaborate with me on my travel collection "${list.icon} ${list.name}".\n\nJoin and view our map here:\n${joinUrl}\n\nHappy travels! 🗺️`
          })
        : { sms: `sms:?&body=${encodeURIComponent(text)}` };

      if (platform === 'sms') {
        window.location.href = links.sms;
      } else if (platform === 'whatsapp') {
        window.open(links.whatsapp, '_blank', 'noopener,noreferrer');
      } else if (platform === 'messenger') {
        window.open(`https://www.facebook.com/dialog/send?link=${encodeURIComponent(joinUrl)}&app_id=291494419107518&redirect_uri=${encodeURIComponent(joinUrl)}`, '_blank', 'noopener,noreferrer');
      } else if (platform === 'instagram') {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).catch(() => {});
        }
        ToastManager.show('📸 Trip invite copied! Opening Instagram...', 'info');
        setTimeout(() => window.open('https://instagram.com/', '_blank', 'noopener,noreferrer'), 600);
      } else if (platform === 'twitter') {
        window.open(links.twitter, '_blank', 'noopener,noreferrer');
      } else if (platform === 'email') {
        window.location.href = links.email;
      }
    },

    toggleShareListQrCode() {
      const box = document.getElementById('share-list-qr-box');
      const img = document.getElementById('share-list-qr-img');
      const label = document.getElementById('qr-list-toggle-label');
      if (!box || !img) return;

      const isHidden = box.classList.contains('hidden');
      if (isHidden) {
        const input = document.getElementById('share-link-input');
        const joinUrl = input ? input.value : `${window.location.origin}/`;
        const qr = window.bListHelpers && typeof window.bListHelpers.generateQrSvg === 'function'
          ? window.bListHelpers.generateQrSvg(joinUrl, { size: 200, margin: 2 })
          : { dataUrl: '' };
        img.src = qr.dataUrl;
        box.classList.remove('hidden');
        if (label) label.textContent = 'Hide QR Code';
      } else {
        box.classList.add('hidden');
        if (label) label.textContent = 'Show QR Code for Phone Scan';
      }
    },

    openSharePlaceModal(pinId) {
      const pin = State.allPins.find((p) => p.id === pinId);
      if (!pin) return;

      this.currentSharePin = pin;
      const modal = document.getElementById('share-place-modal');
      const bannerTitle = document.getElementById('share-place-banner-title');
      const bannerSub = document.getElementById('share-place-banner-sub');
      const bannerIcon = document.getElementById('share-place-banner-icon');
      const linkInput = document.getElementById('share-place-link-input');

      if (bannerTitle) bannerTitle.textContent = pin.title;
      if (bannerSub) bannerSub.textContent = pin.address || (pin.category ? `Category: ${pin.category}` : 'Saved Place');
      if (bannerIcon) bannerIcon.textContent = pin.emoji || '📍';

      const isExternalSource = pin.source_url && !pin.source_url.includes(window.location.host);
      const placeUrl = isExternalSource
        ? pin.source_url
        : `${window.location.origin}/?lat=${pin.latitude}&lng=${pin.longitude}&title=${encodeURIComponent(pin.title)}${pin.address ? `&address=${encodeURIComponent(pin.address)}` : ''}${pin.category ? `&category=${encodeURIComponent(pin.category)}` : ''}`;

      if (linkInput) linkInput.value = placeUrl;

      // Hide QR by default
      const qrBox = document.getElementById('share-place-qr-box');
      const qrLabel = document.getElementById('qr-place-toggle-label');
      if (qrBox) qrBox.classList.add('hidden');
      if (qrLabel) qrLabel.textContent = 'Show QR Code for Phone Scan';

      if (modal) modal.classList.remove('hidden');
    },

    closeSharePlaceModal() {
      const modal = document.getElementById('share-place-modal');
      if (modal) modal.classList.add('hidden');
    },

    async copySharePlaceLink() {
      const input = document.getElementById('share-place-link-input');
      if (!input || !input.value) return;

      if (navigator.clipboard && navigator.clipboard.writeText) {
        try {
          await navigator.clipboard.writeText(input.value);
          ToastManager.show('📋 Place link copied to clipboard!', 'success');
          return;
        } catch (_) {}
      }

      input.select();
      document.execCommand('copy');
      ToastManager.show('📋 Place link copied!', 'success');
    },

    sharePlaceVia(platform) {
      const pin = this.currentSharePin;
      if (!pin) return;

      const input = document.getElementById('share-place-link-input');
      const placeUrl = input ? input.value : `${window.location.origin}/`;
      const title = `${pin.title} | bList`;
      const text = `Check out ${pin.title}${pin.address ? ` (${pin.address})` : ''} on bList:\n${placeUrl}`;

      const links = window.bListHelpers && typeof window.bListHelpers.generateShareLinks === 'function'
        ? window.bListHelpers.generateShareLinks(placeUrl, {
            title,
            text,
            smsText: `Check out ${pin.title}${pin.address ? ` (${pin.address})` : ''}: ${placeUrl}`,
            emailBody: `Hey!\n\nI thought you'd love this spot from my travel bucket list:\n\n📍 ${pin.title}${pin.address ? `\nAddress: ${pin.address}` : ''}\n\nView on map:\n${placeUrl}\n\nHappy travels!`
          })
        : { sms: `sms:?&body=${encodeURIComponent(text)}` };

      if (platform === 'sms') {
        window.location.href = links.sms;
      } else if (platform === 'whatsapp') {
        window.open(links.whatsapp, '_blank', 'noopener,noreferrer');
      } else if (platform === 'messenger') {
        window.open(`https://www.facebook.com/dialog/send?link=${encodeURIComponent(placeUrl)}&app_id=291494419107518&redirect_uri=${encodeURIComponent(placeUrl)}`, '_blank', 'noopener,noreferrer');
      } else if (platform === 'instagram') {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).catch(() => {});
        }
        ToastManager.show('📸 Place details copied! Opening Instagram...', 'info');
        setTimeout(() => window.open('https://instagram.com/', '_blank', 'noopener,noreferrer'), 600);
      } else if (platform === 'twitter') {
        window.open(links.twitter, '_blank', 'noopener,noreferrer');
      } else if (platform === 'email') {
        window.location.href = links.email;
      }
    },

    toggleSharePlaceQrCode() {
      const box = document.getElementById('share-place-qr-box');
      const img = document.getElementById('share-place-qr-img');
      const label = document.getElementById('qr-place-toggle-label');
      if (!box || !img) return;

      const isHidden = box.classList.contains('hidden');
      if (isHidden) {
        const input = document.getElementById('share-place-link-input');
        const placeUrl = input ? input.value : `${window.location.origin}/`;
        const qr = window.bListHelpers && typeof window.bListHelpers.generateQrSvg === 'function'
          ? window.bListHelpers.generateQrSvg(placeUrl, { size: 200, margin: 2 })
          : { dataUrl: '' };
        img.src = qr.dataUrl;
        box.classList.remove('hidden');
        if (label) label.textContent = 'Hide QR Code';
      } else {
        box.classList.add('hidden');
        if (label) label.textContent = 'Show QR Code for Phone Scan';
      }
    },

    openSyncModal() {
      const modal = document.getElementById('sync-modal');
      const syncInput = document.getElementById('sync-link-input');
      const qrImg = document.getElementById('sync-qr-img');
      const userToken = ApiClient.getUserToken();

      const syncUrl = `${window.location.origin}/?sync_token=${encodeURIComponent(userToken)}`;
      if (syncInput) {
        syncInput.value = syncUrl;
      }
      if (qrImg) {
        if (window.bListHelpers && typeof window.bListHelpers.generateQrSvg === 'function') {
          const qr = window.bListHelpers.generateQrSvg(syncUrl, { size: 180, margin: 2 });
          qrImg.src = qr.dataUrl;
        } else {
          qrImg.src = `https://api.qrserver.com/v1/create-qr-code/?size=180x180&data=${encodeURIComponent(syncUrl)}`;
        }
      }

      if (modal) modal.classList.remove('hidden');
    },

    closeSyncModal() {
      const modal = document.getElementById('sync-modal');
      if (modal) modal.classList.add('hidden');
    },

    async copySyncLink() {
      const syncInput = document.getElementById('sync-link-input');
      if (!syncInput || !syncInput.value) return;

      if (navigator.clipboard && navigator.clipboard.writeText) {
        try {
          await navigator.clipboard.writeText(syncInput.value);
          ToastManager.show('📱 Sync link copied to clipboard!', 'success');
          return;
        } catch (_) {}
      }

      syncInput.select();
      document.execCommand('copy');
      ToastManager.show('📱 Sync link copied to clipboard!', 'success');
    },

    async handleRestoreKeySubmit() {
      const input = document.getElementById('restore-key-input');
      const token = input ? input.value.trim() : '';
      if (!token) {
        ToastManager.show('Please enter a valid Sync Key.', 'error');
        return;
      }
      localStorage.setItem('blist_user_token', token);
      ToastManager.show('🔄 Restoring your account session...', 'success');
      this.closeSyncModal();
      setTimeout(() => {
        window.location.reload();
      }, 500);
    },

    switchAboutTab(tabName = 'explorer') {
      const tabs = ['explorer', 'features', 'tech', 'creator'];
      tabs.forEach((t) => {
        const btn = document.getElementById(`tab-btn-${t}`);
        const panel = document.getElementById(`about-panel-${t}`);
        if (btn) {
          const isActive = t === tabName;
          btn.classList.toggle('active', isActive);
          btn.setAttribute('aria-selected', isActive ? 'true' : 'false');
        }
        if (panel) {
          panel.classList.toggle('hidden', t !== tabName);
          panel.classList.toggle('active', t === tabName);
        }
      });

      const body = document.querySelector('#about-modal .about-modal-body');
      if (body) {
        body.scrollTo({ top: 0, behavior: 'smooth' });
      }

      if (window.lucide) window.lucide.createIcons();
    },

    async openAboutModal() {
      const modal = document.getElementById('about-modal');
      if (!modal) return;

      modal.classList.remove('hidden');
      if (window.lucide) window.lucide.createIcons();

      // Reset to default tab
      this.switchAboutTab('explorer');

      // Populate badges & milestones
      this.renderBadges();

      // Ensure profile preview in about modal is synced
      ProfileManager.renderHeaderProfile();

      // Ensure inspiration card is populated on first open
      if (!this._currentInspiration && window.bListHelpers) {
        this.rollInspiration();
      }

      try {
        const info = await ApiClient.fetchAppInfo();
        if (info && info.version) {
          const ver = info.version.startsWith('v') ? info.version : `v${info.version}`;
          const badge = document.getElementById('about-version-badge');
          const metaVer = document.getElementById('about-meta-version');
          const sideVer = document.getElementById('sidebar-version-tag');
          if (badge) badge.textContent = ver;
          if (metaVer) metaVer.textContent = ver;
          if (sideVer) sideVer.textContent = ver;
        }
      } catch (_) {}
    },

    renderBadges() {
      const grid = document.getElementById('about-badges-grid');
      const countPill = document.getElementById('about-badges-count');
      if (!grid || !window.bListHelpers || typeof window.bListHelpers.calculateBadges !== 'function') return;

      const results = window.bListHelpers.calculateBadges({
        pins: State.allPins || [],
        lists: State.lists || [],
        isSynced: !!State.userToken,
        easterEggUnlocked: !!this._easterEggUnlocked
      });

      if (countPill) {
        countPill.textContent = `${results.unlockedCount} / ${results.totalBadges} Unlocked`;
      }

      grid.innerHTML = results.badges.map((b) => {
        const statusClass = b.unlocked ? 'unlocked' : 'locked';
        const statusTag = b.unlocked
          ? '<span class="badge-unlocked-tag">✓ Unlocked</span>'
          : `<span class="badge-item-desc" style="font-weight: 700;">${b.current}/${b.target}</span>`;

        return `
          <div class="badge-item-card ${statusClass}">
            <div class="badge-item-icon" aria-hidden="true">${b.emoji}</div>
            <div class="badge-item-info">
              <div class="badge-item-header">
                <span class="badge-item-name">${Utils.escapeHtml(b.name)}</span>
                ${statusTag}
              </div>
              <p class="badge-item-desc">${Utils.escapeHtml(b.description)}</p>
              ${!b.unlocked ? `
                <div class="badge-item-progress-wrap">
                  <div class="badge-mini-track">
                    <div class="badge-mini-fill" style="width: ${b.percentage}%;"></div>
                  </div>
                </div>
              ` : ''}
            </div>
          </div>
        `;
      }).join('');
    },

    closeAboutModal() {
      const modal = document.getElementById('about-modal');
      if (modal) modal.classList.add('hidden');
      if (window.location.hash === '#about') {
        history.replaceState(null, document.title, window.location.pathname + window.location.search);
      }
    },

    openShareBListModal() {
      // Seamlessly dismiss About modal if open
      this.closeAboutModal();

      const modal = document.getElementById('share-blist-modal');
      if (!modal) return;

      const appUrl = window.location.origin + '/';
      const input = document.getElementById('share-blist-url-input');
      if (input) input.value = appUrl;

      // Check native share support
      const nativeContainer = document.getElementById('native-share-container');
      if (nativeContainer) {
        if (navigator.share) {
          nativeContainer.classList.remove('hidden');
        } else {
          nativeContainer.classList.add('hidden');
        }
      }

      modal.classList.remove('hidden');
      if (window.lucide) window.lucide.createIcons();
    },

    closeShareBListModal() {
      const modal = document.getElementById('share-blist-modal');
      if (modal) modal.classList.add('hidden');
    },

    async copyBListLink() {
      const appUrl = window.location.origin + '/';
      const btnText = document.getElementById('copy-blist-btn-text');
      
      let copied = false;
      if (navigator.clipboard && navigator.clipboard.writeText) {
        try {
          await navigator.clipboard.writeText(appUrl);
          copied = true;
        } catch (_) {}
      }

      if (!copied) {
        const input = document.getElementById('share-blist-url-input');
        if (input) {
          input.select();
          document.execCommand('copy');
          copied = true;
        }
      }

      if (btnText) {
        const originalText = btnText.textContent;
        btnText.textContent = '✓ Copied!';
        setTimeout(() => {
          btnText.textContent = originalText;
        }, 2000);
      }

      ToastManager.show('📋 bList link copied to clipboard!', 'success');
    },

    async shareBListViaNative() {
      const appUrl = window.location.origin + '/';
      const shareData = {
        title: 'bList - Visual Map Bucket List & Trip Planner',
        text: 'Save places from Instagram & Maps, organize trips, and explore your travel bucket list on a clean interactive map with bList! 🗺️✨',
        url: appUrl
      };

      if (navigator.share) {
        try {
          await navigator.share(shareData);
          ToastManager.show('✨ Shared bList successfully!', 'success');
        } catch (err) {
          if (err.name !== 'AbortError') {
            await this.copyBListLink();
          }
        }
      } else {
        await this.copyBListLink();
      }
    },

    async shareBListVia(platform) {
      const appUrl = window.location.origin + '/';
      const links = window.bListHelpers && typeof window.bListHelpers.generateShareLinks === 'function'
        ? window.bListHelpers.generateShareLinks(appUrl)
        : {
            sms: `sms:?&body=${encodeURIComponent(`Check out bList for travel maps & bucket lists: ${appUrl}`)}`,
            whatsapp: `https://api.whatsapp.com/send?text=${encodeURIComponent(`Check out bList: ${appUrl}`)}`,
            messenger: `fb-messenger://share/?link=${encodeURIComponent(appUrl)}`,
            twitter: `https://twitter.com/intent/tweet?text=${encodeURIComponent(`Check out bList — visual travel bucket list & map trip planner! 🗺️✨ ${appUrl}`)}`,
            email: `mailto:?subject=${encodeURIComponent('Check out bList!')}&body=${encodeURIComponent(`Hey!\n\nI thought you would love bList for saving places and organizing travel bucket lists on a map:\n\n${appUrl}`)}`
          };

      if (platform === 'sms') {
        window.location.href = links.sms;
      } else if (platform === 'whatsapp') {
        window.open(links.whatsapp, '_blank', 'noopener,noreferrer');
      } else if (platform === 'messenger') {
        window.open(`https://www.facebook.com/dialog/send?link=${encodeURIComponent(appUrl)}&app_id=291494419107518&redirect_uri=${encodeURIComponent(appUrl)}`, '_blank', 'noopener,noreferrer');
      } else if (platform === 'instagram') {
        // Copy text & link for Instagram Story / DM
        const igText = `Check out bList for travel bucket lists! 🗺️ ${appUrl}`;
        if (navigator.clipboard && navigator.clipboard.writeText) {
          try {
            await navigator.clipboard.writeText(igText);
          } catch (_) {}
        }
        ToastManager.show('📸 Link copied! Opening Instagram to share...', 'info');
        setTimeout(() => {
          window.open('https://instagram.com/', '_blank', 'noopener,noreferrer');
        }, 600);
      } else if (platform === 'twitter') {
        window.open(links.twitter, '_blank', 'noopener,noreferrer');
      } else if (platform === 'email') {
        window.location.href = links.email;
      }
    },

    toggleShareQrCode() {
      const box = document.getElementById('share-qr-box');
      const img = document.getElementById('share-qr-img');
      const label = document.getElementById('qr-toggle-label');
      if (!box || !img) return;

      const isHidden = box.classList.contains('hidden');
      if (isHidden) {
        const appUrl = window.location.origin + '/';
        if (window.bListHelpers && typeof window.bListHelpers.generateQrSvg === 'function') {
          const qr = window.bListHelpers.generateQrSvg(appUrl, { size: 240, margin: 2 });
          img.src = qr.dataUrl;
        } else {
          img.src = `https://api.qrserver.com/v1/create-qr-code/?size=240x240&data=${encodeURIComponent(appUrl)}&margin=8`;
        }
        box.classList.remove('hidden');
        if (label) label.textContent = 'Hide QR Code';
      } else {
        box.classList.add('hidden');
        if (label) label.textContent = 'Show QR Code for Phone Scan';
      }
    },

    // Interactive Easter Egg & Whimsical Logo Clicker
    _aboutLogoClicks: 0,
    handleAboutLogoClick(event) {
      this._aboutLogoClicks = (this._aboutLogoClicks || 0) + 1;
      const clicks = this._aboutLogoClicks;

      const trigger = document.getElementById('about-logo-trigger');
      if (trigger) {
        trigger.style.transform = `scale(${1 + (clicks % 4) * 0.15}) rotate(${((clicks * 25) % 90) - 45}deg)`;
        setTimeout(() => {
          trigger.style.transform = '';
        }, 300);
      }

      if (clicks === 1) {
        ToastManager.show('🧭 Ready for adventure!', 'info');
      } else if (clicks === 3) {
        ToastManager.show('🗺️ Wanderlust tingling!', 'info');
        this.spawnConfettiBurst();
      } else if (clicks >= 5) {
        this._easterEggUnlocked = true;
        const secretBadge = document.getElementById('about-secret-badge');
        if (secretBadge) secretBadge.style.display = 'inline-flex';
        ToastManager.show('🏆 Master Wanderer Unlocked! Zero-AI, 100% Pure Deterministic Map Magic!', 'success');
        this.spawnConfettiBurst(20);
        this._aboutLogoClicks = 0;
        this.renderBadges();
      }
    },

    spawnConfettiBurst(count = 10) {
      const emojis = ['✨', '📍', '✈️', '🍜', '🗺️', '🎒', '🏔️', '🧭', '🗼', '🏖️', '🏮', '🛵', '☕'];
      for (let i = 0; i < count; i++) {
        const span = document.createElement('span');
        span.className = 'confetti-particle';
        span.textContent = emojis[Math.floor(Math.random() * emojis.length)];
        span.style.left = `${Math.random() * 80 + 10}vw`;
        span.style.top = `${Math.random() * 40 + 30}vh`;
        span.style.setProperty('--dx', `${(Math.random() - 0.5) * 100}px`);
        span.style.setProperty('--rot', `${(Math.random() - 0.5) * 180}deg`);
        document.body.appendChild(span);
        setTimeout(() => {
          if (span.parentNode) span.parentNode.removeChild(span);
        }, 1600);
      }
    },

    // Whimsical Inspiration Wonder Generator
    _currentInspiration: null,
    rollInspiration() {
      if (!window.bListHelpers || typeof window.bListHelpers.getRandomInspiration !== 'function') return;
      const currentIdx = this._currentInspiration ? this._currentInspiration.index : -1;
      const next = window.bListHelpers.getRandomInspiration(currentIdx);
      if (!next) return;

      this._currentInspiration = next;
      const emojiEl = document.getElementById('inspire-emoji');
      const catEl = document.getElementById('inspire-cat');
      const titleEl = document.getElementById('inspire-title');
      const addrEl = document.getElementById('inspire-addr');
      const descEl = document.getElementById('inspire-desc');
      const notesEl = document.getElementById('inspire-notes');

      if (emojiEl) emojiEl.textContent = next.emoji;
      if (catEl) catEl.textContent = next.category;
      if (titleEl) titleEl.textContent = next.title;
      if (addrEl) addrEl.textContent = `📍 ${next.address}`;
      if (descEl) descEl.textContent = next.description;
      if (notesEl) notesEl.innerHTML = `💡 <em>${Utils.escapeHtml(next.notes)}</em>`;

      const card = document.getElementById('inspiration-card');
      if (card) {
        card.style.animation = 'none';
        card.offsetHeight; // Trigger reflow
        card.style.animation = 'bounceIn 0.35s ease';
      }
    },

    async addCurrentInspirationToMap() {
      if (!this._currentInspiration) return;
      const item = this._currentInspiration;
      const activeListId = State.selectedListId && State.selectedListId > 0 ? State.selectedListId : 1;

      const payload = {
        list_id: activeListId,
        title: item.title,
        description: item.description,
        latitude: item.latitude,
        longitude: item.longitude,
        category: item.category,
        emoji: item.emoji,
        address: item.address,
        notes: item.notes,
        visited: false
      };

      try {
        await ApiClient.createPin(payload);
        ToastManager.show(`📍 "${item.title}" saved to your bucket list!`, 'success');
        this.closeAboutModal();
        await App.loadData();
        // Fly to new place on map
        MapController.flyToCoordinates(item.latitude, item.longitude, 12);
      } catch (err) {
        ToastManager.show(`Could not save place: ${err.message}`, 'error');
      }
    },

    handleBackdropClick(e, modalId) {
      if (e.target.id === modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
          if (modalId === 'about-modal') {
            this.closeAboutModal();
          } else if (modalId === 'share-blist-modal') {
            this.closeShareBListModal();
          } else {
            modal.classList.add('hidden');
          }
        }
      }
    }
  };

  // ==========================================================================
  // 11. Feature Actions Engine
  // ==========================================================================
  const FeatureActions = {
    surpriseMe() {
      const pool = State.allPins.filter((p) => !p.visited);
      const candidates = pool.length > 0 ? pool : State.allPins;

      if (candidates.length === 0) {
        ToastManager.show('No places to choose from! Save some links first 🗺️', 'error');
        return;
      }

      const headerBtn = document.getElementById('surprise-btn-header');
      const mapBtn = document.getElementById('btn-surprise-map');
      if (headerBtn) headerBtn.style.transform = 'scale(0.92)';
      if (mapBtn) mapBtn.style.transform = 'scale(0.92)';
      setTimeout(() => {
        if (headerBtn) headerBtn.style.transform = '';
        if (mapBtn) mapBtn.style.transform = '';
      }, 200);

      const picked = candidates[Math.floor(Math.random() * candidates.length)];

      if (window.innerWidth <= 768) {
        UIManager.showMobileView('map');
      }

      if (State.map) {
        State.map.flyTo([picked.latitude, picked.longitude], 16, { duration: 1.5 });
      }

      setTimeout(() => {
        if (State.markers[picked.id]) {
          MapController.loadAndRenderPopup(State.markers[picked.id], picked);

          const elem = document.getElementById(`marker-elem-${picked.id}`);
          if (elem) {
            elem.classList.add('surprise-pin');
            setTimeout(() => elem.classList.remove('surprise-pin'), 4000);
          }
        }

        const card = document.getElementById(`card-pin-${picked.id}`);
        if (card) {
          card.scrollIntoView({ behavior: 'smooth', block: 'center' });
          card.classList.add('highlight-surprise');
          setTimeout(() => card.classList.remove('highlight-surprise'), 3000);
        }

        ToastManager.show(`🎲 Surprise Pick: "${picked.title}"!`);
      }, 800);
    },

    locateUser() {
      if (!navigator.geolocation) {
        ToastManager.show('⚠️ Geolocation is not supported by your browser', 'error');
        return;
      }

      const fab = document.getElementById('locate-fab');
      if (fab) fab.classList.add('fab-highlight');

      navigator.geolocation.getCurrentPosition(
        (position) => {
          if (fab) fab.classList.remove('fab-highlight');
          State.currentUserLocation = {
            lat: position.coords.latitude,
            lng: position.coords.longitude
          };

          MapController.updateUserLocationMarker();
          if (State.map) {
            State.map.flyTo(
              [State.currentUserLocation.lat, State.currentUserLocation.lng],
              14,
              { duration: 1.2 }
            );
          }
          ToastManager.show('📍 Location found!');
          UIManager.renderPinList();
          if (window.lucide) window.lucide.createIcons();
        },
        (error) => {
          if (fab) fab.classList.remove('fab-highlight');
          let msg = 'Could not access GPS coordinates';
          if (error.code === 1) msg = 'Location permission denied';
          else if (error.code === 2) msg = 'Position unavailable';
          else if (error.code === 3) msg = 'Location request timed out';
          ToastManager.show('⚠️ ' + msg, 'error');
        },
        { enableHighAccuracy: true, timeout: 10000 }
      );
    },

    async sharePin(id) {
      ModalManager.openSharePlaceModal(id);
    },

    async handleSaveLinkSubmit(e, inputId = 'save-url-input') {
      e.preventDefault();
      const input = document.getElementById(inputId) || document.getElementById('save-url-input');
      if (!input) return;
      const url = input.value.trim();
      if (!url) return;

      const overlay = document.getElementById('loading-overlay');
      if (overlay) overlay.classList.remove('hidden');

      let targetListId = (State.lists && State.lists.length > 0) ? State.lists[0].id : null;
      if (
        State.currentListFilter !== 'all' &&
        State.currentListFilter !== 'bucket' &&
        State.currentListFilter !== 'visited'
      ) {
        targetListId = parseInt(State.currentListFilter, 10) || targetListId;
      }

      try {
        const json = await ApiClient.ingestPin(url, targetListId);
        if (overlay) overlay.classList.add('hidden');

        if (json.success && json.data) {
          input.value = '';
          UIManager.closeMobileQuickAdd();
          State.allPins.unshift(json.data);

          UIManager.renderAll();
          ToastManager.show(`✨ Saved "${json.data.title}"!`, 'success');

          if (window.innerWidth <= 768) {
            UIManager.showMobileView('map');
          }
          MapController.flyToPin(json.data.id);
        } else {
          ToastManager.show(
            json.error || 'Failed to extract place details. Try adding manually with "+ Add Place"!',
            'error'
          );
        }
      } catch (err) {
        if (overlay) overlay.classList.add('hidden');
        ToastManager.show(err.message || 'Error connecting to server. Please check your network.', 'error');
      }
    },

    async toggleVisited(id) {
      const idx = State.allPins.findIndex((p) => p.id === id);
      if (idx !== -1) {
        State.allPins[idx].visited = State.allPins[idx].visited ? 0 : 1;
        localStorage.setItem('blist_cached_pins', JSON.stringify(State.allPins));
        UIManager.renderAll();
        const pin = State.allPins[idx];
        ToastManager.show(
          pin.visited ? `✅ Visited "${pin.title}"!` : `🎯 Added "${pin.title}" back to Bucket List!`,
          'success'
        );

        // Check if toggling visited completed a trip list
        if (pin.visited && pin.list_id) {
          const listPins = State.allPins.filter((p) => (p.list_id || 1) === pin.list_id);
          if (listPins.length >= 2 && listPins.every((p) => p.visited === 1 || p.visited === true)) {
            const list = State.lists.find((l) => l.id === pin.list_id);
            const listName = list ? list.name : 'Trip List';
            setTimeout(() => {
              ToastManager.show(`🎉 "${listName}" 100% Completed! You earned the "Mission Complete" badge! 💯`, 'success');
              ModalManager.spawnConfettiBurst(15);
            }, 600);
          }
        }
      }

      try {
        if (!navigator.onLine) {
          OfflineManager.enqueue({ type: 'toggleVisited', id });
          return;
        }
        const json = await ApiClient.toggleVisited(id);
        if (json && json.success && json.data) {
          if (idx !== -1) {
            State.allPins[idx] = json.data;
            localStorage.setItem('blist_cached_pins', JSON.stringify(State.allPins));
            UIManager.renderAll();
          }
        }
      } catch (err) {
        if (!navigator.onLine) {
          OfflineManager.enqueue({ type: 'toggleVisited', id });
        } else {
          ToastManager.show('Note: Offline change queued for sync', 'info');
        }
      }
    },

    async deletePin(id) {
      const pin = State.allPins.find((p) => p.id === id);
      const title = pin ? pin.title : 'this place';
      if (!confirm(`Remove "${title}" from your bucket list?`)) return;

      try {
        const json = await ApiClient.deletePin(id);
        if (json.success) {
          State.allPins = State.allPins.filter((p) => p.id !== id);
          UIManager.renderAll();
          ToastManager.show('Place deleted');
        }
      } catch (err) {
        ToastManager.show(err.message || 'Failed to delete place', 'error');
      }
    },

    async exportData(format = 'geojson') {
      try {
        const data = await ApiClient.exportData(format);
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `blist_${new Date().toISOString().split('T')[0]}.${format === 'geojson' ? 'geojson' : 'json'}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        ToastManager.show('Export file downloaded!');
      } catch (err) {
        ToastManager.show(err.message || 'Failed to export data', 'error');
      }
    },

    openGoogleMapsRoute() {
      const filtered = FilterManager.getFilteredPins();
      if (!filtered || filtered.length < 2) {
        ToastManager.show('Add at least 2 places to create multi-stop directions', 'error');
        return;
      }

      const url = (window.bListHelpers && typeof window.bListHelpers.generateGoogleMapsRouteUrl === 'function')
        ? window.bListHelpers.generateGoogleMapsRouteUrl(filtered)
        : null;

      if (url) {
        window.open(url, '_blank', 'noopener,noreferrer');
        ToastManager.show(`🚗 Opening ${Math.min(filtered.length, 10)}-stop route in Google Maps!`, 'success');
      } else {
        ToastManager.show('Could not generate multi-stop route directions', 'error');
      }
    }
  };

  // ==========================================================================
  // 12. Global Click Listeners & Service Worker Registration
  // ==========================================================================
  function setupGlobalClickHandlers() {
    document.addEventListener('click', (e) => {
      // Close Theme menu if clicking outside
      const themeWrapper = document.getElementById('theme-switcher-wrapper');
      const themeMenu = document.getElementById('theme-menu');
      if (themeWrapper && !themeWrapper.contains(e.target) && themeMenu) {
        themeMenu.classList.add('hidden');
      }

      // Close Layer menu if clicking outside
      const layerWrapper = document.getElementById('layer-control-wrapper');
      const layerMenu = document.getElementById('layer-menu');
      if (layerWrapper && !layerWrapper.contains(e.target) && layerMenu) {
        layerMenu.classList.add('hidden');
      }

      // Close Mobile More menu if clicking outside
      const moreWrapper = document.getElementById('mobile-more-wrapper');
      const moreMenu = document.getElementById('mobile-more-menu');
      if (moreWrapper && !moreWrapper.contains(e.target) && moreMenu) {
        moreMenu.classList.add('hidden');
      }

      // Close Mobile Quick-Add bar if clicking outside
      const quickAddBar = document.getElementById('header-mobile-bar');
      const addHeaderBtn = document.getElementById('add-place-btn-header');
      if (
        quickAddBar &&
        quickAddBar.classList.contains('expanded') &&
        !quickAddBar.contains(e.target) &&
        addHeaderBtn &&
        !addHeaderBtn.contains(e.target)
      ) {
        quickAddBar.classList.remove('expanded');
      }
    });

    // Close any open modals or menus on Escape key
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        const moreMenu = document.getElementById('mobile-more-menu');
        if (moreMenu) moreMenu.classList.add('hidden');
        const themeMenu = document.getElementById('theme-menu');
        if (themeMenu) themeMenu.classList.add('hidden');
        const layerMenu = document.getElementById('layer-menu');
        if (layerMenu) layerMenu.classList.add('hidden');
        const quickAddBar = document.getElementById('header-mobile-bar');
        if (quickAddBar) quickAddBar.classList.remove('expanded');

        const openModals = document.querySelectorAll('.modal-overlay:not(.hidden)');
        openModals.forEach((m) => {
          if (m.id === 'about-modal') {
            ModalManager.closeAboutModal();
          } else {
            m.classList.add('hidden');
          }
        });
      }
    });

    // Live Plus Code preview updates in place form
    ['form-lat', 'form-lon', 'form-address'].forEach((id) => {
      const el = document.getElementById(id);
      if (el) {
        el.addEventListener('input', () => ModalManager.updateModalPlusCodePreview());
      }
    });
  }

  function registerServiceWorker() {
    if ('serviceWorker' in navigator && window.location.protocol.startsWith('http')) {
      window.addEventListener('load', () => {
        navigator.serviceWorker.register('/sw.js').catch((err) => {
          console.warn('[PWA] Service Worker registration failed:', err);
        });
      });
    }
  }

  async function handleIncomingJoinLink() {
    const params = new URLSearchParams(window.location.search);
    let joinToken = params.get('join');

    if (!joinToken) {
      const rawText = params.get('url') || params.get('text') || params.get('title') || '';
      const parsed = window.bListHelpers && typeof window.bListHelpers.parseShareTargetPayload === 'function'
        ? window.bListHelpers.parseShareTargetPayload(rawText)
        : null;
      if (parsed && parsed.isJoinLink && parsed.joinToken) {
        joinToken = parsed.joinToken;
      }
    }

    if (!joinToken || !joinToken.trim()) return;

    try {
      const res = await ApiClient.joinList(joinToken.trim());
      if (res && res.success && res.data) {
        const list = res.data;
        ToastManager.show(`🎉 Joined shared collection "${list.icon} ${list.name}"!`, 'success');
        const lists = await ApiClient.fetchLists();
        State.lists = lists;
        FilterManager.selectList(list.id);

        if (typeof ProfileManager !== 'undefined' && (!ProfileManager.profile.name || !ProfileManager.profile.name.trim())) {
          setTimeout(() => {
            ProfileManager.openModal();
            ToastManager.show(`👋 Welcome to ${list.name}! Set your name & avatar so travel buddies know who's exploring.`, 'info');
          }, 800);
        }
      } else {
        ToastManager.show((res && res.error) || 'Could not join shared list.', 'error');
      }
    } catch (err) {
      ToastManager.show(err.message || 'Failed to join shared list', 'error');
    }

    params.delete('join');
    params.delete('url');
    params.delete('text');
    params.delete('title');
    const newSearch = params.toString();
    const newUrl = window.location.pathname + (newSearch ? '?' + newSearch : '') + window.location.hash;
    window.history.replaceState({}, document.title, newUrl);
  }

  async function handleIncomingSyncLink() {
    const params = new URLSearchParams(window.location.search);
    let syncToken = params.get('sync_token');

    if (!syncToken) {
      const rawText = params.get('url') || params.get('text') || params.get('title') || '';
      const parsed = window.bListHelpers && typeof window.bListHelpers.parseShareTargetPayload === 'function'
        ? window.bListHelpers.parseShareTargetPayload(rawText)
        : null;
      if (parsed && parsed.isSyncLink && parsed.syncToken) {
        syncToken = parsed.syncToken;
      }
    }

    if (!syncToken || !syncToken.trim()) return;

    localStorage.setItem('blist_user_token', syncToken.trim());
    ToastManager.show('📱 Linked and synced with your device session!', 'success');

    params.delete('sync_token');
    params.delete('url');
    params.delete('text');
    params.delete('title');
    const newSearch = params.toString();
    const newUrl = window.location.pathname + (newSearch ? '?' + newSearch : '') + window.location.hash;
    window.history.replaceState({}, document.title, newUrl);
  }

  async function handleIncomingShareTarget(explicitText = null) {
    let sharedPayload = explicitText;
    if (!sharedPayload) {
      const params = new URLSearchParams(window.location.search);
      sharedPayload = params.get('url') || params.get('text') || params.get('title');
      if (params.has('url') || params.has('text') || params.has('title')) {
        params.delete('url');
        params.delete('text');
        params.delete('title');
        const newSearch = params.toString();
        const newUrl = window.location.pathname + (newSearch ? '?' + newSearch : '') + window.location.hash;
        window.history.replaceState({}, document.title, newUrl);
      }
    }

    if (!sharedPayload || !sharedPayload.trim()) return;

    // Use parseShareTargetPayload helper to handle structured and wrapped text
    const parsed = window.bListHelpers && typeof window.bListHelpers.parseShareTargetPayload === 'function'
      ? window.bListHelpers.parseShareTargetPayload(sharedPayload)
      : null;

    if (parsed && (parsed.isJoinLink || parsed.isSyncLink)) {
      // Already handled by handleIncomingJoinLink / handleIncomingSyncLink
      return;
    }

    const linkToSave = (parsed && (parsed.url || parsed.title || parsed.text))
      ? (parsed.url || parsed.title || parsed.text)
      : sharedPayload.trim();

    if (!linkToSave || !linkToSave.trim()) return;

    const isUrl = parsed && parsed.isUrlCandidate;
    ToastManager.show(isUrl ? '📥 Processing shared location link...' : `🔍 Finding location for "${linkToSave}"...`, 'info');

    const overlay = document.getElementById('loading-overlay');
    if (overlay) overlay.classList.remove('hidden');

    try {
      const targetListId = (State.currentListFilter && !isNaN(parseInt(State.currentListFilter, 10)))
        ? parseInt(State.currentListFilter, 10)
        : (State.lists[0] ? State.lists[0].id : 1);

      const json = await ApiClient.ingestPin(linkToSave, targetListId);
      if (overlay) overlay.classList.add('hidden');

      if (json && json.success && json.data) {
        State.allPins.unshift(json.data);
        UIManager.renderAll();
        ToastManager.show(`✨ Saved "${json.data.title}"!`, 'success');
        if (window.innerWidth <= 768) {
          UIManager.showMobileView('map');
        }
        MapController.flyToPin(json.data.id);
      } else {
        ToastManager.show((json && json.error) || 'Failed to extract location. Tap "+ Add Place" to add manually.', 'error');
      }
    } catch (err) {
      if (overlay) overlay.classList.add('hidden');
      ToastManager.show(err.message || 'Error processing shared link', 'error');
    }
  }

  // ==========================================================================
  // 12. Universal Importer & Route Optimizer Engines
  // ==========================================================================
  const ImportModalController = {
    selectedFile: null,
    fileContent: null,

    openModal() {
      UIManager.closeMobileQuickAdd();
      this.clearFile();
      const select = document.getElementById('import-dest-list');
      if (select) {
        select.innerHTML = '';
        if (State.lists && State.lists.length > 0) {
          State.lists.forEach((l) => {
            const opt = document.createElement('option');
            opt.value = String(l.id);
            opt.textContent = `${l.icon || '📁'} ${l.name}`;
            select.appendChild(opt);
          });
        } else {
          const opt = document.createElement('option');
          opt.value = '1';
          opt.textContent = '📍 My Bucket List';
          select.appendChild(opt);
        }
        const newOpt = document.createElement('option');
        newOpt.value = 'new';
        newOpt.textContent = '✨ + Create New Trip Collection...';
        select.appendChild(newOpt);

        if (
          State.currentListFilter !== 'all' &&
          State.currentListFilter !== 'bucket' &&
          State.currentListFilter !== 'visited'
        ) {
          select.value = State.currentListFilter;
        }
      }
      this.handleDestListChange();

      const summaryBox = document.getElementById('import-summary-box');
      if (summaryBox) {
        summaryBox.className = 'import-summary-box hidden';
        summaryBox.innerHTML = '';
      }
      const progressContainer = document.getElementById('import-progress-container');
      if (progressContainer) progressContainer.classList.add('hidden');

      const modal = document.getElementById('import-modal');
      if (modal) modal.classList.remove('hidden');
      if (window.lucide) window.lucide.createIcons();
    },

    closeModal() {
      const modal = document.getElementById('import-modal');
      if (modal) modal.classList.add('hidden');
    },

    handleDestListChange() {
      const select = document.getElementById('import-dest-list');
      const newGroup = document.getElementById('import-new-list-group');
      if (select && newGroup) {
        if (select.value === 'new') {
          newGroup.classList.remove('hidden');
        } else {
          newGroup.classList.add('hidden');
        }
      }
    },

    handleFileSelect(e) {
      const file = e.target.files && e.target.files[0];
      if (!file) return;
      this.loadFile(file);
    },

    loadFile(file) {
      if (!file) return;

      if (file.size > 25 * 1024 * 1024) {
        ToastManager.show('Selected file is too large (25 MB max limit).', 'error');
        this.clearFile();
        return;
      }

      this.selectedFile = file;
      const dropContent = document.getElementById('dropzone-content');
      const fileInfo = document.getElementById('dropzone-file-info');
      const nameEl = document.getElementById('selected-file-name');
      const sizeEl = document.getElementById('selected-file-size');
      const startBtn = document.getElementById('btn-start-import');

      if (nameEl) nameEl.textContent = file.name;
      if (sizeEl) sizeEl.textContent = Utils.formatFileSize(file.size);

      if (dropContent) dropContent.classList.add('hidden');
      if (fileInfo) fileInfo.classList.remove('hidden');

      const reader = new FileReader();
      reader.onload = (event) => {
        this.fileContent = event.target.result;
        if (startBtn) {
          startBtn.disabled = false;
          startBtn.innerHTML = '<i data-lucide="upload" class="btn-icon"></i><span>Start Import</span>';
          startBtn.onclick = () => this.executeImport();
          if (window.lucide) window.lucide.createIcons();
        }
      };
      reader.onerror = () => {
        ToastManager.show('Failed to read selected file. Please try another file.', 'error');
        this.clearFile();
      };
      reader.onabort = () => {
        this.clearFile();
      };
      reader.readAsText(file);
    },

    clearFile(e) {
      if (e) e.stopPropagation();
      this.selectedFile = null;
      this.fileContent = null;
      const input = document.getElementById('import-file-input');
      if (input) input.value = '';
      const dropContent = document.getElementById('dropzone-content');
      const fileInfo = document.getElementById('dropzone-file-info');
      const startBtn = document.getElementById('btn-start-import');

      if (dropContent) dropContent.classList.remove('hidden');
      if (fileInfo) fileInfo.classList.add('hidden');
      if (startBtn) startBtn.disabled = true;
    },

    setupDropzoneListeners() {
      const dropzone = document.getElementById('import-dropzone');
      if (!dropzone || dropzone._hasDropListeners) return;

      ['dragenter', 'dragover'].forEach((eventName) => {
        dropzone.addEventListener(eventName, (e) => {
          e.preventDefault();
          e.stopPropagation();
          dropzone.classList.add('dragover');
        });
      });

      ['dragleave', 'drop'].forEach((eventName) => {
        dropzone.addEventListener(eventName, (e) => {
          e.preventDefault();
          e.stopPropagation();
          dropzone.classList.remove('dragover');
        });
      });

      dropzone.addEventListener('drop', (e) => {
        const dt = e.dataTransfer;
        const files = dt.files;
        if (files && files.length > 0) {
          ImportModalController.loadFile(files[0]);
        }
      });

      dropzone._hasDropListeners = true;
    },

    async executeImport() {
      if (!this.fileContent) {
        ToastManager.show('Please select a file to import.', 'error');
        return;
      }

      const destSelect = document.getElementById('import-dest-list');
      const newNameInput = document.getElementById('import-new-list-name');
      const catSelect = document.getElementById('import-default-category');
      const startBtn = document.getElementById('btn-start-import');
      const progressContainer = document.getElementById('import-progress-container');
      const progressLabel = document.getElementById('import-progress-label');
      const progressBar = document.getElementById('import-progress-bar');
      const summaryBox = document.getElementById('import-summary-box');

      let listId = null;
      let newListName = null;

      if (destSelect.value === 'new') {
        newListName = newNameInput ? newNameInput.value.trim() : '';
        if (!newListName) {
          ToastManager.show('Please enter a name for the new collection.', 'error');
          if (newNameInput) newNameInput.focus();
          return;
        }
      } else {
        listId = parseInt(destSelect.value, 10);
      }

      const defaultCategory = catSelect ? catSelect.value : 'General';

      if (startBtn) {
        startBtn.disabled = true;
        startBtn.innerHTML = '<span class="spinner-sm"></span> Importing...';
      }
      if (progressContainer) progressContainer.classList.remove('hidden');
      if (progressLabel) progressLabel.textContent = 'Parsing places and resolving GPS coordinates...';
      if (progressBar) progressBar.style.width = '60%';

      try {
        const payload = {
          list_id: listId,
          new_list_name: newListName,
          default_category: defaultCategory,
          raw_data: this.fileContent
        };

        const res = await ApiClient.importPlaces(payload);
        if (progressBar) progressBar.style.width = '100%';

        if (res && res.success && res.data) {
          const summary = res.data;
          ToastManager.show(`🎉 Successfully imported ${summary.imported_count} places into "${summary.list_name}"!`, 'success');

          if (summaryBox) {
            summaryBox.className = 'import-summary-box success';
            summaryBox.classList.remove('hidden');
            let warningsHtml = '';
            if (summary.warnings && summary.warnings.length > 0) {
              warningsHtml = `
                <div style="margin-top: 8px; font-weight: 600; color: var(--warning-text);">Warnings (${summary.warnings.length}):</div>
                <ul class="import-warnings-list">
                  ${summary.warnings.map((w) => `<li>${Utils.escapeHtml(w)}</li>`).join('')}
                </ul>
              `;
            }

            summaryBox.innerHTML = `
              <div class="import-summary-title">✅ Import Completed</div>
              <div class="import-summary-stats">
                Processed <strong>${summary.total_processed}</strong> places &bull; 
                Imported <strong>${summary.imported_count}</strong> &bull; 
                Skipped <strong>${summary.skipped_count}</strong>
              </div>
              ${warningsHtml}
            `;
          }

          // Refresh state and switch to active list
          await App.loadData();
          if (summary.list_id) {
            FilterManager.selectList(summary.list_id);
          }
          if (startBtn) {
            startBtn.disabled = false;
            startBtn.innerHTML = '<span>Done</span>';
            startBtn.onclick = () => this.closeModal();
          }
        } else {
          throw new Error((res && res.error) || 'Import failed');
        }
      } catch (err) {
        ToastManager.show(err.message || 'Import failed. Please check file format.', 'error');
        if (progressContainer) progressContainer.classList.add('hidden');
        if (startBtn) {
          startBtn.disabled = false;
          startBtn.innerHTML = '<i data-lucide="upload" class="btn-icon"></i><span>Start Import</span>';
        }
        if (window.lucide) window.lucide.createIcons();
      }
    }
  };

  const RouteOptimizer = {
    async optimizeCurrentRoute() {
      const pins = FilterManager.getFilteredPins();
      if (pins.length < 3) {
        ToastManager.show('Need at least 3 places to optimize a route.', 'info');
        return;
      }

      ToastManager.show('⚡ Calculating shortest sequence (2-Opt TSP)...', 'info');

      // Solve TSP with nearest neighbor + 2-opt
      const optimizeFn = (window.bListHelpers && window.bListHelpers.optimizeTour2Opt) ||
                         (window.Helpers && window.Helpers.optimizeTour2Opt);
      const tour = optimizeFn ? optimizeFn(pins) : pins;

      // Update custom_order in local state and persist
      for (let idx = 0; idx < tour.length; idx++) {
        const pin = tour[idx];
        pin.custom_order = idx;
        const statePin = State.allPins.find((p) => p.id === pin.id);
        if (statePin) statePin.custom_order = idx;
        ApiClient.updatePin(pin.id, { custom_order: idx }).catch(() => {});
      }

      if (!State.isRouteActive) {
        MapController.toggleRouteLine();
      } else {
        MapController.updateRouteLine();
      }
      UIManager.renderPinList();

      ToastManager.show('✨ Route optimized! Places ordered for the fastest itinerary.', 'success');
    }
  };

  // ==========================================================================
  // 12b. Mobile Swipe Gesture Navigation
  // ==========================================================================
  const SwipeNavigationManager = {
    touchStartX: 0,
    touchStartY: 0,
    touchStartTime: 0,
    sidebarEl: null,

    init() {
      this.sidebarEl = document.getElementById('sidebar');
      this.setupGlobalSwipe();
      this.setupSidebarDrag();
      this.setupModalSwipe();
    },

    setupGlobalSwipe() {
      document.addEventListener('touchstart', (e) => {
        if (e.touches.length !== 1) return;
        this.touchStartX = e.touches[0].clientX;
        this.touchStartY = e.touches[0].clientY;
        this.touchStartTime = Date.now();
      }, { passive: true });

      document.addEventListener('touchend', (e) => {
        if (e.changedTouches.length !== 1 || window.innerWidth > 768) return;
        const endX = e.changedTouches[0].clientX;
        const endY = e.changedTouches[0].clientY;
        const elapsed = Date.now() - this.touchStartTime;
        if (elapsed > 700) return; // Ignore long presses/stalls

        const gesture = window.bListHelpers && typeof window.bListHelpers.detectSwipeGesture === 'function'
          ? window.bListHelpers.detectSwipeGesture({
              startX: this.touchStartX,
              startY: this.touchStartY,
              endX,
              endY,
              minDistance: 45,
              maxPerpendicular: 60,
              edgeThreshold: 45,
              screenWidth: window.innerWidth
            })
          : null;

        if (!gesture || !gesture.isSwipe) return;

        const isSidebarOpen = this.sidebarEl && this.sidebarEl.classList.contains('mobile-open');

        // Edge swipe right on map -> Open list/places drawer
        if (!isSidebarOpen && gesture.direction === 'right' && gesture.isLeftEdge) {
          UIManager.showMobileView('list');
        }

        // Swipe left on open sidebar -> Close drawer and return to map
        if (isSidebarOpen && gesture.direction === 'left') {
          UIManager.showMobileView('map');
        }
      }, { passive: true });
    },

    setupSidebarDrag() {
      if (!this.sidebarEl) return;
      let startX = 0;
      let currentX = 0;
      let isTracking = false;

      this.sidebarEl.addEventListener('touchstart', (e) => {
        if (e.touches.length !== 1 || window.innerWidth > 768) return;
        if (!this.sidebarEl.classList.contains('mobile-open')) return;
        startX = e.touches[0].clientX;
        currentX = startX;
        isTracking = true;
      }, { passive: true });

      this.sidebarEl.addEventListener('touchmove', (e) => {
        if (!isTracking || e.touches.length !== 1 || window.innerWidth > 768) return;
        currentX = e.touches[0].clientX;
        const diffX = currentX - startX;
        if (diffX < 0) {
          // Dragging left (towards closing)
          this.sidebarEl.style.transition = 'none';
          this.sidebarEl.style.transform = `translateX(${diffX}px)`;
        }
      }, { passive: true });

      const handleTouchEnd = () => {
        if (!isTracking) return;
        isTracking = false;
        this.sidebarEl.style.transition = '';
        const diffX = currentX - startX;
        if (diffX < -55) {
          // Swiped past threshold -> close drawer
          UIManager.showMobileView('map');
          this.sidebarEl.style.transform = '';
        } else {
          // Spring back open
          this.sidebarEl.style.transform = '';
        }
      };

      this.sidebarEl.addEventListener('touchend', handleTouchEnd, { passive: true });
      this.sidebarEl.addEventListener('touchcancel', handleTouchEnd, { passive: true });
    },

    setupModalSwipe() {
      // Swiping down on mobile modals / bottom sheets to dismiss
      const modalOverlays = document.querySelectorAll('.modal-overlay, .modal');
      modalOverlays.forEach((modal) => {
        let modalStartY = 0;
        let modalStartX = 0;
        let isModalTracking = false;
        const card = modal.querySelector('.modal-card, .modal-content') || modal;

        card.addEventListener('touchstart', (e) => {
          if (e.touches.length !== 1 || window.innerWidth > 768) return;
          const rect = card.getBoundingClientRect();
          const relativeY = e.touches[0].clientY - rect.top;
          // Trigger when drag starts in upper header area or drag handles
          if (relativeY <= 90 || e.target.closest('.modal-header') || e.target.closest('.modal-drag-handle')) {
            modalStartY = e.touches[0].clientY;
            modalStartX = e.touches[0].clientX;
            isModalTracking = true;
          }
        }, { passive: true });

        card.addEventListener('touchmove', (e) => {
          if (!isModalTracking || e.touches.length !== 1 || window.innerWidth > 768) return;
          const currentY = e.touches[0].clientY;
          const diffY = currentY - modalStartY;
          if (diffY > 0) {
            // Dragging downward
            card.style.transition = 'none';
            card.style.transform = `translateY(${diffY}px)`;
          }
        }, { passive: true });

        const handleModalTouchEnd = (e) => {
          if (!isModalTracking) return;
          isModalTracking = false;
          card.style.transition = '';
          const endY = e.changedTouches && e.changedTouches.length ? e.changedTouches[0].clientY : modalStartY;
          const endX = e.changedTouches && e.changedTouches.length ? e.changedTouches[0].clientX : modalStartX;
          const deltaY = endY - modalStartY;
          const deltaX = Math.abs(endX - modalStartX);

          // Swipe down (> 55px downwards, vertical movement dominates)
          if (deltaY > 55 && deltaY > deltaX * 1.25) {
            card.style.transform = '';
            modal.classList.add('hidden');
            if (modal.id === 'about-modal' && window.location.hash === '#about') {
              history.replaceState(null, document.title, window.location.pathname + window.location.search);
            }
          } else {
            // Snap back up
            card.style.transform = '';
          }
        };

        card.addEventListener('touchend', handleModalTouchEnd, { passive: true });
        card.addEventListener('touchcancel', handleModalTouchEnd, { passive: true });
      });
    }
  };

  // ==========================================================================
  // 12c. User Profile & Smol SVG Avatar Customizer
  // ==========================================================================
  const ProfileManager = {
    profile: {
      name: '',
      avatar: '🧭',
      color: '#3b82f6'
    },
    draftAvatar: '🧭',
    draftColor: '#3b82f6',

    init() {
      const savedName = localStorage.getItem('blist_user_name') || '';
      const savedAvatar = localStorage.getItem('blist_user_avatar') || '🧭';
      const savedColor = localStorage.getItem('blist_user_color') || '#3b82f6';

      this.profile = {
        name: savedName,
        avatar: savedAvatar,
        color: savedColor
      };
      this.draftAvatar = savedAvatar;
      this.draftColor = savedColor;

      this.renderHeaderProfile();
      this.syncWithBackend();
    },

    async syncWithBackend() {
      const remote = await ApiClient.fetchUserProfile();
      if (remote) {
        if (remote.name || remote.avatar || remote.color) {
          this.profile.name = remote.name || this.profile.name;
          this.profile.avatar = remote.avatar || this.profile.avatar;
          this.profile.color = remote.color || this.profile.color;
          this.draftAvatar = this.profile.avatar;
          this.draftColor = this.profile.color;

          localStorage.setItem('blist_user_name', this.profile.name);
          localStorage.setItem('blist_user_avatar', this.profile.avatar);
          localStorage.setItem('blist_user_color', this.profile.color);

          this.renderHeaderProfile();
        }
      }
    },

    renderHeaderProfile() {
      const circle = document.getElementById('header-avatar-circle');
      const mobileCircle = document.getElementById('mobile-avatar-circle');
      const nameEl = document.getElementById('header-user-name');
      const gen = window.bListHelpers && typeof window.bListHelpers.generateAvatarSvg === 'function'
        ? window.bListHelpers.generateAvatarSvg({
            avatar: this.profile.avatar,
            color: this.profile.color,
            name: this.profile.name,
            size: 28
          })
        : null;

      [circle, mobileCircle].forEach((el) => {
        if (el) {
          if (gen && gen.svg) {
            el.innerHTML = gen.svg;
          } else {
            el.textContent = this.profile.avatar || '🧭';
          }
        }
      });

      if (nameEl) {
        nameEl.textContent = this.profile.name.trim() ? this.profile.name.trim() : 'Profile';
      }

      // Also update About modal profile preview if present
      const aboutAvatar = document.getElementById('about-profile-avatar-preview');
      const aboutName = document.getElementById('about-profile-name');
      if (aboutAvatar) {
        const aboutGen = window.bListHelpers && typeof window.bListHelpers.generateAvatarSvg === 'function'
          ? window.bListHelpers.generateAvatarSvg({
              avatar: this.profile.avatar,
              color: this.profile.color,
              name: this.profile.name,
              size: 48
            })
          : null;
        if (aboutGen && aboutGen.svg) {
          aboutAvatar.innerHTML = aboutGen.svg;
        } else {
          aboutAvatar.textContent = this.profile.avatar || '🧭';
        }
      }
      if (aboutName) {
        aboutName.textContent = this.profile.name.trim() ? this.profile.name.trim() : 'Explorer';
      }
    },

    openModal() {
      // Seamlessly close About modal if open
      ModalManager.closeAboutModal();

      this.draftAvatar = this.profile.avatar || '🧭';
      this.draftColor = this.profile.color || '#3b82f6';

      const input = document.getElementById('profile-name-input');
      if (input) {
        input.value = this.profile.name || '';
      }

      this.renderPresetGrid();
      this.renderColorPalette();
      this.updatePreview();

      const modal = document.getElementById('user-profile-modal');
      if (modal) modal.classList.remove('hidden');
    },

    closeModal() {
      const modal = document.getElementById('user-profile-modal');
      if (modal) modal.classList.add('hidden');
    },

    renderPresetGrid() {
      const grid = document.getElementById('avatar-preset-grid');
      if (!grid) return;
      const presets = (window.bListHelpers && window.bListHelpers.AVATAR_PRESETS) || [
        '🧭', '🏕️', '✈️', '🍜', '🗼', '🎒', '🦊', '🐻',
        '🐬', '🦉', '🚀', '🏄', '🎨', '🚴', '⛵', '🦁'
      ];

      grid.innerHTML = presets.map((icon) => {
        const isActive = this.draftAvatar === icon ? 'active' : '';
        return `<button type="button" class="avatar-preset-chip ${isActive}" onclick="selectProfileAvatar('${icon}')" aria-label="Avatar ${icon}">${icon}</button>`;
      }).join('');
    },

    renderColorPalette() {
      const palette = document.getElementById('avatar-color-palette');
      if (!palette) return;
      const colors = (window.bListHelpers && window.bListHelpers.AVATAR_COLORS) || [
        '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#6366f1'
      ];

      palette.innerHTML = colors.map((col) => {
        const isActive = this.draftColor === col ? 'active' : '';
        return `<button type="button" class="avatar-color-dot ${isActive}" style="background-color: ${col};" onclick="selectProfileColor('${col}')" aria-label="Color ${col}"></button>`;
      }).join('');
    },

    selectAvatar(icon) {
      this.draftAvatar = icon;
      this.renderPresetGrid();
      this.updatePreview();
    },

    selectColor(col) {
      this.draftColor = col;
      this.renderColorPalette();
      this.updatePreview();
    },

    handleNameInput() {
      this.updatePreview();
    },

    updatePreview() {
      const previewBox = document.getElementById('profile-avatar-preview');
      const previewName = document.getElementById('profile-preview-name');
      const input = document.getElementById('profile-name-input');
      const currentName = input ? input.value.trim() : '';

      const gen = window.bListHelpers && typeof window.bListHelpers.generateAvatarSvg === 'function'
        ? window.bListHelpers.generateAvatarSvg({
            avatar: this.draftAvatar,
            color: this.draftColor,
            name: currentName,
            size: 64
          })
        : null;

      if (previewBox && gen && gen.svg) {
        previewBox.innerHTML = gen.svg;
      }
      if (previewName) {
        previewName.textContent = currentName || 'Explorer';
      }
    },

    async save() {
      const input = document.getElementById('profile-name-input');
      const name = input ? input.value.trim() : '';

      this.profile = {
        name,
        avatar: this.draftAvatar,
        color: this.draftColor
      };

      localStorage.setItem('blist_user_name', this.profile.name);
      localStorage.setItem('blist_user_avatar', this.profile.avatar);
      localStorage.setItem('blist_user_color', this.profile.color);

      this.renderHeaderProfile();
      this.closeModal();

      ToastManager.show('✨ Explorer profile & avatar updated!', 'success');

      try {
        await ApiClient.updateUserProfile(this.profile);
        this.renderCollaboratorsForCurrentList();
      } catch (_) {}
    },

    async renderCollaboratorsForCurrentList() {
      const bar = document.getElementById('list-collaborators-bar');
      const stack = document.getElementById('collab-avatar-stack');
      if (!bar || !stack) return;

      const currentListId = State.currentListFilter;
      if (!currentListId || currentListId === 'all' || isNaN(Number(currentListId))) {
        bar.classList.add('hidden');
        return;
      }

      const collabs = await ApiClient.fetchCollaborators(Number(currentListId));
      if (!collabs || collabs.length <= 1) {
        bar.classList.add('hidden');
        return;
      }

      bar.classList.remove('hidden');
      stack.innerHTML = collabs.map((c) => {
        const gen = window.bListHelpers && typeof window.bListHelpers.generateAvatarSvg === 'function'
          ? window.bListHelpers.generateAvatarSvg({
              avatar: c.avatar || '🧭',
              color: c.color || '#3b82f6',
              name: c.name,
              size: 24
            })
          : null;

        const roleText = c.is_owner ? ' (Owner)' : '';
        const titleAttr = Utils.escapeHtml(`${c.name || 'Traveler'}${roleText}`);
        const svgContent = gen && gen.svg ? gen.svg : `<span>${Utils.escapeHtml(c.avatar || '👤')}</span>`;

        return `<div class="collab-avatar-item" title="${titleAttr}">${svgContent}</div>`;
      }).join('');
    }
  };

  const App = {
    async loadData() {
      try {
        const lists = await ApiClient.fetchLists();
        State.lists = lists.length > 0 ? lists : [{ id: 1, name: 'My Bucket List', icon: '📍', created_at: '' }];
      } catch (_) {
        State.lists = [{ id: 1, name: 'My Bucket List', icon: '📍', created_at: '' }];
      }

      try {
        State.allPins = await ApiClient.fetchPins();
        UIManager.renderAll();
        UIManager.preloadWeatherForVisiblePins();
      } catch (err) {
        ToastManager.show('Failed to load saved places', 'error');
      }
    }
  };

  // ==========================================================================
  // 13. Application Lifecycle Initialization
  // ==========================================================================
  document.addEventListener('DOMContentLoaded', async () => {
    ThemeManager.init();
    MapController.init();
    OfflineManager.init();
    SwipeNavigationManager.init();
    ProfileManager.init();

    await handleIncomingSyncLink();
    await handleIncomingJoinLink();
    await App.loadData();
    await handleIncomingShareTarget();

    ImportModalController.setupDropzoneListeners();

    const initialParams = new URLSearchParams(window.location.search);
    if (initialParams.get('view') === 'list' || window.location.hash === '#list') {
      UIManager.showMobileView('list');
    }

    if (
      initialParams.get('about') !== null ||
      window.location.hash === '#about' ||
      window.location.pathname === '/about'
    ) {
      ModalManager.openAboutModal();
    }

    window.addEventListener('hashchange', () => {
      if (window.location.hash === '#about') {
        ModalManager.openAboutModal();
      }
    });

    setupGlobalClickHandlers();
    registerServiceWorker();
    if (window.lucide) window.lucide.createIcons();
  });

  // ==========================================================================
  // 14. Expose Global Functions for HTML Handlers & Backward Compatibility
  // ==========================================================================
  window.bList = {
    State,
    Utils,
    ToastManager,
    ThemeManager,
    ApiClient,
    MapController,
    FilterManager,
    UIManager,
    ModalManager,
    FeatureActions,
    ImportModalController,
    RouteOptimizer
  };

  // Theme
  window.setTheme = (theme) => ThemeManager.set(theme);
  window.toggleTheme = () => ThemeManager.toggle();
  window.toggleThemeMenu = () => ThemeManager.toggleMenu();
  window.toggleMobileMoreMenu = (e) => {
    if (e && e.stopPropagation) e.stopPropagation();
    const menu = document.getElementById('mobile-more-menu');
    if (menu) {
      menu.classList.toggle('hidden');
    }
  };
  window.closeMobileMoreMenu = () => {
    const menu = document.getElementById('mobile-more-menu');
    if (menu) menu.classList.add('hidden');
  };

  // Navigation & Map
  window.toggleSidebar = () => UIManager.toggleSidebar();
  window.showMobileView = (view) => UIManager.showMobileView(view);
  window.resetMapView = () => MapController.resetMapView();
  window.resetView = window.resetMapView;
  window.toggleLayerMenu = () => MapController.toggleLayerMenu();
  window.toggleLayerSwitcher = window.toggleLayerMenu;
  window.switchMapLayer = (layerKey) => MapController.switchLayer(layerKey);
  window.toggleRouteLine = () => MapController.toggleRouteLine();
  window.optimizeCurrentRoute = () => RouteOptimizer.optimizeCurrentRoute();

  // About & Info
  window.openAboutModal = () => ModalManager.openAboutModal();
  window.closeAboutModal = () => ModalManager.closeAboutModal();
  window.switchAboutTab = (tab) => ModalManager.switchAboutTab(tab);
  window.handleAboutLogoClick = (e) => ModalManager.handleAboutLogoClick(e);
  window.rollInspiration = () => ModalManager.rollInspiration();
  window.addCurrentInspirationToMap = () => ModalManager.addCurrentInspirationToMap();

  // Share bList
  window.openShareBListModal = () => ModalManager.openShareBListModal();
  window.closeShareBListModal = () => ModalManager.closeShareBListModal();
  window.copyBListLink = () => ModalManager.copyBListLink();
  window.shareBListViaNative = () => ModalManager.shareBListViaNative();
  window.shareBListVia = (platform) => ModalManager.shareBListVia(platform);
  window.toggleShareQrCode = () => ModalManager.toggleShareQrCode();

  // Import
  window.openImportModal = () => ImportModalController.openModal();
  window.closeImportModal = () => ImportModalController.closeModal();
  window.handleImportFileSelect = (e) => ImportModalController.handleFileSelect(e);
  window.handleImportDestListChange = () => ImportModalController.handleDestListChange();
  window.clearImportFile = (e) => ImportModalController.clearFile(e);
  window.executeImport = () => ImportModalController.executeImport();

  // Features
  window.surpriseMe = () => FeatureActions.surpriseMe();
  window.handleSurpriseMe = window.surpriseMe;
  window.locateUser = () => FeatureActions.locateUser();
  window.handleLocateMe = window.locateUser;
  window.sharePin = (id) => FeatureActions.sharePin(id);
  window.exportData = (format) => FeatureActions.exportData(format);
  window.showToast = (msg, type) => ToastManager.show(msg, type);
  window.escapeHtml = (str) => Utils.escapeHtml(str);
  window.copyPlusCode = async (code) => {
    if (!code) return;
    if (navigator.clipboard && navigator.clipboard.writeText) {
      try {
        await navigator.clipboard.writeText(code);
        ToastManager.show(`🧭 Plus Code ${code} copied to clipboard!`, 'success');
        return;
      } catch (_) {}
    }
    ToastManager.show(`🧭 Plus Code: ${code}`, 'info');
  };
  window.copyFormPlusCode = () => {
    const el = document.getElementById('form-plus-code-val');
    if (el && el.textContent) {
      window.copyPlusCode(el.textContent.trim());
    }
  };

  // Sync & Multi-Device
  window.openSyncModal = () => ModalManager.openSyncModal();
  window.closeSyncModal = () => ModalManager.closeSyncModal();
  window.copySyncLink = () => ModalManager.copySyncLink();
  window.handleRestoreKeySubmit = () => ModalManager.handleRestoreKeySubmit();

  // Lists & Trips
  window.handleListChange = (e) => FilterManager.selectList(e.target.value);
  window.openNewListModal = () => ModalManager.openNewListModal();
  window.openCreateListModal = window.openNewListModal;
  window.openShareListModal = () => ModalManager.openShareListModal();
  window.closeShareListModal = () => ModalManager.closeShareListModal();
  window.copyShareLink = () => ModalManager.copyShareLink();
  window.shareListVia = (platform) => ModalManager.shareListVia(platform);
  window.toggleShareListQrCode = () => ModalManager.toggleShareListQrCode();
  window.selectListIcon = (emoji) => ModalManager.selectListIcon(emoji);
  window.selectEmoji = window.selectListIcon;
  window.handleCreateListSubmit = (e) => ModalManager.handleCreateListSubmit(e);

  // Place Sharing
  window.openSharePlaceModal = (id) => ModalManager.openSharePlaceModal(id);
  window.closeSharePlaceModal = () => ModalManager.closeSharePlaceModal();
  window.copySharePlaceLink = () => ModalManager.copySharePlaceLink();
  window.sharePlaceVia = (platform) => ModalManager.sharePlaceVia(platform);
  window.toggleSharePlaceQrCode = () => ModalManager.toggleSharePlaceQrCode();

  // Filter & Search
  window.setStatusFilter = (status) => FilterManager.setStatusFilter(status);
  window.togglePriorityFilter = () => FilterManager.togglePriorityFilter();
  window.toggleOpenNowFilter = () => FilterManager.toggleOpenNowFilter();
  window.setTagFilter = (tag) => FilterManager.setTagFilter(tag);
  window.setDayFilter = (day) => FilterManager.setDayFilter(day);
  window.setCategoryFilter = (cat) => FilterManager.setCategoryFilter(cat);
  window.handleSearch = (e) => FilterManager.handleSearch(e.target.value);
  window.handleSortChange = (e) => FilterManager.handleSortChange(e.target.value);

  // Profile & Avatars
  window.openUserProfileModal = () => ProfileManager.openModal();
  window.closeUserProfileModal = () => ProfileManager.closeModal();
  window.selectProfileAvatar = (icon) => ProfileManager.selectAvatar(icon);
  window.selectProfileColor = (col) => ProfileManager.selectColor(col);
  window.handleProfileNameInput = () => ProfileManager.handleNameInput();
  window.saveUserProfile = () => ProfileManager.save();

  // Pins & Modals
  window.handlePinCardClick = (id) => {
    if (window.innerWidth <= 768) {
      UIManager.showMobileView('map');
    }
    MapController.flyToPin(id);
  };
  window.handleHeaderAddClick = () => UIManager.handleHeaderAddClick();
  window.toggleMobileQuickAdd = (forceOpen) => UIManager.toggleMobileQuickAdd(forceOpen);
  window.closeMobileQuickAdd = () => UIManager.closeMobileQuickAdd();
  window.openManualPinModal = (lat, lon) => ModalManager.openManualPinModal(lat, lon);
  window.openEditPinModal = (id) => ModalManager.openEditPinModal(id);
  window.closePinModal = () => ModalManager.closePinModal();
  window.handleDeleteFromPinModal = () => ModalManager.handleDeleteFromPinModal();
  window.handlePinFormSubmit = (e) => ModalManager.handlePinFormSubmit(e);
  window.handleModalBackdropClick = (e, modalId) => ModalManager.handleBackdropClick(e, modalId);
  window.handleSaveLinkSubmit = (e, inputId) => FeatureActions.handleSaveLinkSubmit(e, inputId);
  window.toggleVisited = (id) => FeatureActions.toggleVisited(id);
  window.deletePin = (id) => FeatureActions.deletePin(id);
  window.openGoogleMapsRoute = () => FeatureActions.openGoogleMapsRoute();
  window.handleNativeShare = (text) => handleIncomingShareTarget(text);
})();
