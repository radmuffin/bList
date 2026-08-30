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
      light: {
        name: 'Clean Light',
        url: 'https://cartodb-basemaps-{s}.global.ssl.fastly.net/light_all/{z}/{x}/{y}.png',
        options: {
          maxZoom: 19,
          subdomains: 'abcd',
          attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap</a> &copy; <a href="https://carto.com/attributions">CARTO</a>'
        }
      },
      dark: {
        name: 'Dark Mode',
        url: 'https://server.arcgisonline.com/ArcGIS/rest/services/Canvas/World_Dark_Gray_Base/MapServer/tile/{z}/{y}/{x}',
        options: {
          maxZoom: 16,
          attribution: 'Tiles &copy; Esri &mdash; Esri, DeLorme, NAVTEQ'
        }
      },
      satellite: {
        name: 'Satellite',
        url: 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{x}/{y}',
        options: {
          maxZoom: 18,
          attribution: 'Tiles &copy; Esri &mdash; Source: Esri, i-cubed, USDA, USGS, AEX, GeoEye, Getmapping, Aerogrid, IGN, IGP, UPR-EGP'
        }
      }
    },

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
      80: { icon: '🌦️', text: 'Rain Showers' },
      81: { icon: '🌧️', text: 'Showers' },
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
    allPins: [],
    lists: [],
    currentListFilter: 'all', // 'all', 'bucket', 'visited', or list_id as string
    selectedCategory: 'All',
    selectedStatus: 'all', // 'all', 'bucket', 'visited'
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
  const Utils = {
    escapeHtml(str) {
      if (!str) return '';
      return String(str).replace(/[&<>"']/g, (m) => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;'
      })[m]);
    },

    calculateDistance(lat1, lon1, lat2, lon2) {
      const R = 6371; // Earth radius in km
      const dLat = ((lat2 - lat1) * Math.PI) / 180;
      const dLon = ((lon2 - lon1) * Math.PI) / 180;
      const a =
        Math.sin(dLat / 2) * Math.sin(dLat / 2) +
        Math.cos((lat1 * Math.PI) / 180) *
          Math.cos((lat2 * Math.PI) / 180) *
          Math.sin(dLon / 2) *
          Math.sin(dLon / 2);
      const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
      return R * c;
    },

    formatDistance(dKm) {
      if (dKm < 1) {
        return `${Math.round(dKm * 1000)} m away`;
      }
      const mi = (dKm * 0.621371).toFixed(1);
      return `${mi} mi away (${dKm.toFixed(1)} km)`;
    },

    getListNameForPin(pin) {
      if (!pin || !pin.list_id) return null;
      const found = State.lists.find((l) => l.id === pin.list_id);
      return found && found.id !== 1 ? found : null;
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

      document.querySelectorAll('.theme-opt').forEach((btn) => {
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

      const toastMsg =
        theme === 'auto'
          ? '💻 Auto theme (matches system)'
          : theme === 'dark'
          ? '🌙 Dark Mode enabled'
          : '☀️ Light Mode enabled';
      ToastManager.show(toastMsg);
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
    async request(url, options = {}) {
      try {
        const res = await fetch(url, options);
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

    async fetchLists() {
      const json = await this.request('/api/lists');
      return json && json.success && json.data ? json.data : [];
    },

    async createList(payload) {
      return this.request('/api/lists', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    },

    async fetchPins() {
      const json = await this.request('/api/pins');
      return json && json.success && json.data ? json.data : [];
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

    async ingestPin(url, listId = 1) {
      return this.request('/api/pins/ingest', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url, list_id: listId })
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
      const res = await fetch(`/api/export/${format}`);
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
        tap: true
      }).setView([20.0, 0.0], 2);

      this.applyLayer(State.currentLayerName, false);

      // Click on map to add place manually with reverse-geocoding
      State.map.on('click', (e) => {
        const lat = e.latlng.lat.toFixed(6);
        const lon = e.latlng.lng.toFixed(6);
        ModalManager.openManualPinModal(lat, lon);
      });
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
      const isLayerLocked = localStorage.getItem('blist_layer_locked') === 'true';
      if (!isLayerLocked) {
        if (
          effectiveTheme === 'dark' &&
          (State.currentLayerName === 'osm' || State.currentLayerName === 'light')
        ) {
          this.applyLayer('dark', false);
        } else if (effectiveTheme === 'light' && State.currentLayerName === 'dark') {
          this.applyLayer('osm', false);
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

      Object.values(State.markers).forEach((m) => State.map.removeLayer(m));
      State.markers = {};

      const filtered = FilterManager.getFilteredPins();

      filtered.forEach((pin) => {
        const iconName = CONFIG.CATEGORY_ICONS[pin.category] || 'map-pin';
        const customIcon = L.divIcon({
          className: 'custom-pin-container',
          html: `
            <div class="custom-pin-marker ${pin.visited ? 'visited-pin' : ''}" id="marker-elem-${pin.id}">
              <i data-lucide="${iconName}"></i>
            </div>
          `,
          iconSize: [32, 32],
          iconAnchor: [16, 32],
          popupAnchor: [0, -32]
        });

        const marker = L.marker([pin.latitude, pin.longitude], { icon: customIcon }).addTo(
          State.map
        );

        marker.on('click', () => {
          this.loadAndRenderPopup(marker, pin);
        });

        State.markers[pin.id] = marker;
      });

      if (window.lucide) window.lucide.createIcons();
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

      marker.bindPopup(popupHtml).openPopup();
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
      let listPins = State.allPins;

      // Custom List / Trip Filter
      if (State.currentListFilter === 'bucket') {
        listPins = State.allPins.filter((p) => !p.visited);
      } else if (State.currentListFilter === 'visited') {
        listPins = State.allPins.filter((p) => p.visited);
      } else if (State.currentListFilter !== 'all') {
        const listIdNum = parseInt(State.currentListFilter, 10);
        listPins = State.allPins.filter((p) => p.list_id === listIdNum);
      }

      return listPins
        .filter((pin) => {
          // Status Tab Filter
          if (State.selectedStatus === 'bucket' && pin.visited) return false;
          if (State.selectedStatus === 'visited' && !pin.visited) return false;

          // Category Chip Filter
          if (State.selectedCategory !== 'All' && pin.category !== State.selectedCategory) {
            return false;
          }

          // Omni Search Query Filter
          if (State.searchQuery.trim()) {
            const q = State.searchQuery.toLowerCase();
            const matchTitle = pin.title && pin.title.toLowerCase().includes(q);
            const matchAddress = pin.address && pin.address.toLowerCase().includes(q);
            const matchNotes = pin.notes && pin.notes.toLowerCase().includes(q);
            const matchCategory = pin.category && pin.category.toLowerCase().includes(q);
            if (!matchTitle && !matchAddress && !matchNotes && !matchCategory) return false;
          }

          return true;
        })
        .sort((a, b) => {
          if (State.currentSort === 'nearest' && State.currentUserLocation) {
            const distA = Utils.calculateDistance(
              State.currentUserLocation.lat,
              State.currentUserLocation.lng,
              a.latitude,
              a.longitude
            );
            const distB = Utils.calculateDistance(
              State.currentUserLocation.lat,
              State.currentUserLocation.lng,
              b.latitude,
              b.longitude
            );
            return distA - distB;
          } else if (State.currentSort === 'az') {
            return (a.title || '').localeCompare(b.title || '');
          } else if (State.currentSort === 'category') {
            return (a.category || '').localeCompare(b.category || '');
          } else {
            return (b.id || 0) - (a.id || 0);
          }
        });
    },

    selectList(listId) {
      State.currentListFilter = String(listId);
      UIManager.renderAll();
      MapController.resetMapView();
    },

    setStatusFilter(status) {
      State.selectedStatus = status;
      document.querySelectorAll('.filter-tab').forEach((tab) => {
        tab.classList.toggle('active', tab.dataset.status === status);
      });
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
      this.renderPinList();
      MapController.renderMarkers();
      this.updateCounts();
      if (window.lucide) window.lucide.createIcons();
    },

    renderBadgesHtml(pin, { distanceStr, weather, assignedList }) {
      return `
        <span class="pin-badge ${pin.visited ? 'badge-visited' : ''}">
          ${pin.visited ? '✅ Visited' : Utils.escapeHtml(pin.category || 'Place')}
        </span>
        ${
          assignedList
            ? `<span class="pin-badge badge-list">${assignedList.icon || '📁'} ${Utils.escapeHtml(
                assignedList.name
              )}</span>`
            : ''
        }
        ${distanceStr ? `<span class="pin-badge badge-distance">📍 ${distanceStr}</span>` : ''}
        <span class="pin-badge badge-weather ${weather ? '' : 'hidden'}" id="weather-badge-${pin.id}">
          ${weather ? `${weather.icon} ${weather.tempF}°F` : ''}
        </span>
      `;
    },

    renderActionsHtml(pin, isPopup = false) {
      const directionsUrl = `https://www.google.com/maps/dir/?api=1&destination=${pin.latitude},${pin.longitude}`;
      const sourceUrl = pin.source_url ? Utils.escapeHtml(pin.source_url) : '';

      if (isPopup) {
        return `
          <div class="popup-actions-grid">
            <a href="${directionsUrl}" target="_blank" rel="noopener noreferrer" class="btn-popup-action primary-action">
              <i data-lucide="navigation"></i> Directions
            </a>
            <button class="btn-popup-action" onclick="sharePin(${pin.id})">
              <i data-lucide="share-2"></i> Share
            </button>
            ${
              sourceUrl
                ? `<a href="${sourceUrl}" target="_blank" rel="noopener noreferrer" class="btn-popup-action">
                    <i data-lucide="external-link"></i> Link
                   </a>`
                : ''
            }
          </div>
          <div class="popup-footer">
            <label class="status-toggle">
              <input type="checkbox" ${pin.visited ? 'checked' : ''} onchange="toggleVisited(${pin.id})">
              <span>${pin.visited ? 'Visited' : 'Mark Visited'}</span>
            </label>
            <div class="card-action-btns">
              <button class="btn-icon-sm" onclick="openEditPinModal(${pin.id})" title="Edit Place">
                <i data-lucide="edit-3" style="width: 14px; height: 14px;"></i>
              </button>
              <button class="btn-icon-sm delete-btn" onclick="deletePin(${pin.id})" title="Delete Place">
                <i data-lucide="trash-2" style="width: 14px; height: 14px;"></i>
              </button>
            </div>
          </div>
        `;
      }

      return `
        <div class="pin-card-footer" onclick="event.stopPropagation()">
          <label class="status-toggle">
            <input type="checkbox" ${pin.visited ? 'checked' : ''} onchange="toggleVisited(${pin.id})">
            <span>${pin.visited ? 'Visited' : 'Mark Visited'}</span>
          </label>

          <div class="card-action-btns">
            <a href="${directionsUrl}" target="_blank" rel="noopener noreferrer" class="btn-icon-sm" title="Get Directions">
              <i data-lucide="navigation" style="width: 14px; height: 14px;"></i>
            </a>
            <button class="btn-icon-sm" onclick="sharePin(${pin.id})" title="Share Place">
              <i data-lucide="share-2" style="width: 14px; height: 14px;"></i>
            </button>
            ${
              sourceUrl
                ? `<a href="${sourceUrl}" target="_blank" rel="noopener noreferrer" class="btn-icon-sm" title="Open Source Link">
                    <i data-lucide="external-link" style="width: 14px; height: 14px;"></i>
                   </a>`
                : ''
            }
            <button class="btn-icon-sm" onclick="openEditPinModal(${pin.id})" title="Edit Place">
              <i data-lucide="edit-3" style="width: 14px; height: 14px;"></i>
            </button>
            <button class="btn-icon-sm delete-btn" onclick="deletePin(${pin.id})" title="Delete Place">
              <i data-lucide="trash-2" style="width: 14px; height: 14px;"></i>
            </button>
          </div>
        </div>
      `;
    },

    renderPopupHtml(pin, { distanceStr, weather, assignedList }) {
      return `
        <div class="pin-popup">
          ${
            pin.image_url
              ? `<img src="${Utils.escapeHtml(pin.image_url)}" class="popup-img" alt="${Utils.escapeHtml(
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
              pin.address
                ? `<div class="popup-address"><i data-lucide="map-pin" style="width: 12px; height: 12px;"></i> ${Utils.escapeHtml(
                    pin.address
                  )}</div>`
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

      const categories = ['All', ...new Set(State.allPins.map((p) => p.category).filter(Boolean))];

      container.innerHTML = categories
        .map(
          (cat) => `
        <button class="cat-chip ${State.selectedCategory === cat ? 'active' : ''}" onclick="setCategoryFilter('${Utils.escapeHtml(
            cat
          )}')">
          ${Utils.escapeHtml(cat)}
        </button>
      `
        )
        .join('');
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

          return `
            <div class="pin-card ${pin.visited ? 'visited-card' : ''}" onclick="handlePinCardClick(${pin.id})" id="card-pin-${pin.id}">
              <div class="pin-card-header">
                <div class="pin-card-title">${Utils.escapeHtml(pin.title)}</div>
                <div class="badges-row">
                  ${this.renderBadgesHtml(pin, { distanceStr, weather: weatherCached, assignedList })}
                </div>
              </div>

              ${
                pin.image_url
                  ? `<img src="${Utils.escapeHtml(pin.image_url)}" class="pin-card-thumb" alt="${Utils.escapeHtml(
                      pin.title
                    )}" onerror="this.style.display='none'">`
                  : ''
              }

              ${
                pin.address
                  ? `<div class="pin-card-address">
                      <i data-lucide="map-pin" style="width: 13px; height: 13px; flex-shrink: 0;"></i>
                      <span>${Utils.escapeHtml(pin.address)}</span>
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
      const btnMap = document.getElementById('btn-show-map');
      const btnList = document.getElementById('btn-show-list');

      if (view === 'list') {
        if (sidebar) sidebar.classList.add('mobile-open');
        if (btnMap) btnMap.classList.remove('active');
        if (btnList) btnList.classList.add('active');
      } else {
        if (sidebar) sidebar.classList.remove('mobile-open');
        if (btnMap) btnMap.classList.add('active');
        if (btnList) btnList.classList.remove('active');
        setTimeout(() => State.map && State.map.invalidateSize(), 150);
      }
    }
  };

  // ==========================================================================
  // 10. Modal Dialog Manager
  // ==========================================================================
  const ModalManager = {
    async openManualPinModal(lat = '', lon = '') {
      document.getElementById('modal-title').innerText = 'Add Place';
      document.getElementById('form-pin-id').value = '';
      document.getElementById('form-title').value = '';
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

      document.getElementById('pin-modal').classList.remove('hidden');
    },

    openEditPinModal(id) {
      const pin = State.allPins.find((p) => p.id === id);
      if (!pin) return;

      document.getElementById('modal-title').innerText = 'Edit Place';
      document.getElementById('form-pin-id').value = pin.id;
      document.getElementById('form-title').value = pin.title;
      document.getElementById('form-lat').value = pin.latitude;
      document.getElementById('form-lon').value = pin.longitude;
      document.getElementById('form-category').value = pin.category || 'Place';
      document.getElementById('form-visited').value = pin.visited ? 'true' : 'false';
      document.getElementById('form-address').value = pin.address || '';
      document.getElementById('form-image').value = pin.image_url || '';
      document.getElementById('form-source').value = pin.source_url || '';
      document.getElementById('form-notes').value = pin.notes || '';

      const formList = document.getElementById('form-list-id');
      if (formList) {
        formList.value = pin.list_id || (State.lists[0] ? State.lists[0].id : 1);
      }

      document.getElementById('pin-modal').classList.remove('hidden');
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

      const payload = {
        list_id: selectedListId,
        title: document.getElementById('form-title').value.trim(),
        latitude: parseFloat(document.getElementById('form-lat').value),
        longitude: parseFloat(document.getElementById('form-lon').value),
        category: document.getElementById('form-category').value,
        visited: document.getElementById('form-visited').value === 'true',
        address: document.getElementById('form-address').value.trim() || null,
        image_url: document.getElementById('form-image').value.trim() || null,
        source_url: document.getElementById('form-source').value.trim() || null,
        notes: document.getElementById('form-notes').value.trim() || null
      };

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

    handleBackdropClick(e, modalId) {
      if (e.target.id === modalId) {
        const modal = document.getElementById(modalId);
        if (modal) modal.classList.add('hidden');
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
      const pin = State.allPins.find((p) => p.id === id);
      if (!pin) return;

      const shareUrl = pin.source_url || window.location.href;
      const shareData = {
        title: `${pin.title} | bList`,
        text: `Check out ${pin.title} ${pin.address ? `(${pin.address})` : ''} on my travel bucket list!`,
        url: shareUrl
      };

      if (navigator.share) {
        try {
          await navigator.share(shareData);
          ToastManager.show('Shared successfully!');
          return;
        } catch (err) {
          if (err.name === 'AbortError') return;
        }
      }

      // Fallback to clipboard
      const clipboardText = `${pin.title}\n${pin.address || ''}\n${shareUrl}`;
      try {
        await navigator.clipboard.writeText(clipboardText);
        ToastManager.show('📋 Place details copied to clipboard!');
      } catch (_) {
        ToastManager.show('Failed to copy to clipboard', 'error');
      }
    },

    async handleSaveLinkSubmit(e, inputId = 'save-url-input') {
      e.preventDefault();
      const input = document.getElementById(inputId) || document.getElementById('save-url-input');
      if (!input) return;
      const url = input.value.trim();
      if (!url) return;

      const overlay = document.getElementById('loading-overlay');
      if (overlay) overlay.classList.remove('hidden');

      let targetListId = 1;
      if (
        State.currentListFilter !== 'all' &&
        State.currentListFilter !== 'bucket' &&
        State.currentListFilter !== 'visited'
      ) {
        targetListId = parseInt(State.currentListFilter, 10) || 1;
      }

      try {
        const json = await ApiClient.ingestPin(url, targetListId);
        if (overlay) overlay.classList.add('hidden');

        if (json.success && json.data) {
          input.value = '';
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
      try {
        const json = await ApiClient.toggleVisited(id);
        if (json.success && json.data) {
          const idx = State.allPins.findIndex((p) => p.id === id);
          if (idx !== -1) {
            State.allPins[idx] = json.data;
            UIManager.renderAll();
            ToastManager.show(
              json.data.visited ? '🎉 Marked as visited!' : '🎯 Added back to bucket list'
            );
          }
        }
      } catch (err) {
        ToastManager.show(err.message || 'Failed to update status', 'error');
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

  // ==========================================================================
  // 13. Application Lifecycle Initialization
  // ==========================================================================
  document.addEventListener('DOMContentLoaded', async () => {
    ThemeManager.init();
    MapController.init();

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
    FeatureActions
  };

  // Theme
  window.setTheme = (theme) => ThemeManager.set(theme);
  window.toggleThemeMenu = () => ThemeManager.toggleMenu();

  // Navigation & Map
  window.toggleSidebar = () => UIManager.toggleSidebar();
  window.showMobileView = (view) => UIManager.showMobileView(view);
  window.resetMapView = () => MapController.resetMapView();
  window.resetView = window.resetMapView;
  window.toggleLayerMenu = () => MapController.toggleLayerMenu();
  window.toggleLayerSwitcher = window.toggleLayerMenu;
  window.switchMapLayer = (layerKey) => MapController.switchLayer(layerKey);

  // Features
  window.surpriseMe = () => FeatureActions.surpriseMe();
  window.handleSurpriseMe = window.surpriseMe;
  window.locateUser = () => FeatureActions.locateUser();
  window.handleLocateMe = window.locateUser;
  window.sharePin = (id) => FeatureActions.sharePin(id);
  window.exportData = (format) => FeatureActions.exportData(format);
  window.showToast = (msg, type) => ToastManager.show(msg, type);
  window.escapeHtml = (str) => Utils.escapeHtml(str);

  // Lists & Trips
  window.handleListChange = (e) => FilterManager.selectList(e.target.value);
  window.openNewListModal = () => ModalManager.openNewListModal();
  window.openCreateListModal = window.openNewListModal;
  window.closeNewListModal = () => ModalManager.closeNewListModal();
  window.closeCreateListModal = window.closeNewListModal;
  window.selectListIcon = (emoji) => ModalManager.selectListIcon(emoji);
  window.selectEmoji = window.selectListIcon;
  window.handleCreateListSubmit = (e) => ModalManager.handleCreateListSubmit(e);

  // Filter & Search
  window.setStatusFilter = (status) => FilterManager.setStatusFilter(status);
  window.setCategoryFilter = (cat) => FilterManager.setCategoryFilter(cat);
  window.handleSearch = (e) => FilterManager.handleSearch(e.target.value);
  window.handleSortChange = (e) => FilterManager.handleSortChange(e.target.value);

  // Pins & Modals
  window.handlePinCardClick = (id) => {
    if (window.innerWidth <= 768) {
      UIManager.showMobileView('map');
    }
    MapController.flyToPin(id);
  };
  window.openManualPinModal = (lat, lon) => ModalManager.openManualPinModal(lat, lon);
  window.openEditPinModal = (id) => ModalManager.openEditPinModal(id);
  window.closePinModal = () => ModalManager.closePinModal();
  window.handlePinFormSubmit = (e) => ModalManager.handlePinFormSubmit(e);
  window.handleModalBackdropClick = (e, modalId) => ModalManager.handleBackdropClick(e, modalId);
  window.handleSaveLinkSubmit = (e, inputId) => FeatureActions.handleSaveLinkSubmit(e, inputId);
  window.toggleVisited = (id) => FeatureActions.toggleVisited(id);
  window.deletePin = (id) => FeatureActions.deletePin(id);
})();
