// bList - Visual Map Bucket List & Trip Planner

let map;
let currentTileLayer;
let currentLayerName = 'light';
let markers = {};
let allPins = [];
let lists = []; // [{ id, name, icon, created_at }]
let currentListFilter = 'all'; // 'all', 'bucket', 'visited', or numeric list_id as string (e.g. '1', '2')
let selectedCategory = 'All';
let selectedStatus = 'all'; // 'all', 'bucket', 'visited'
let searchQuery = '';
let currentSort = 'newest'; // 'newest', 'nearest', 'az', 'category'
let currentMobileView = 'map'; // 'map' or 'list'
let currentUserLocation = null;
let userLocationMarker = null;
let weatherCache = {};

// Tile Layer Configuration
const MAP_LAYERS = {
  light: {
    name: 'Clean Light',
    url: 'https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png',
    options: {
      maxZoom: 19,
      attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap</a> &copy; <a href="https://carto.com/attributions">CARTO</a>'
    }
  },
  osm: {
    name: 'Streets (OSM)',
    url: 'https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png',
    options: {
      maxZoom: 19,
      attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap</a>'
    }
  },
  dark: {
    name: 'Dark Mode',
    url: 'https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png',
    options: {
      maxZoom: 19,
      attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap</a> &copy; <a href="https://carto.com/attributions">CARTO</a>'
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
};

const CATEGORY_ICONS = {
  'Food & Drink': 'utensils',
  'Cafe': 'coffee',
  'Sightseeing': 'landmark',
  'Nature & Outdoors': 'trees',
  'Hotel & Stay': 'bed',
  'Shopping': 'shopping-bag',
  'General': 'map-pin',
  'Social': 'instagram',
  'Place': 'map-pin'
};

const WEATHER_CODE_MAP = {
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
};

// ============================================================================
// App Lifecycle
// ============================================================================
document.addEventListener('DOMContentLoaded', async () => {
  initMap();
  await fetchLists();
  await fetchPins();
  setupGlobalClickHandlers();
  lucide.createIcons();
});

function setupGlobalClickHandlers() {
  document.addEventListener('click', (e) => {
    const listWrapper = document.querySelector('.custom-select-container');
    const layerWrapper = document.getElementById('layer-control-wrapper');
    const listMenu = document.getElementById('list-dropdown-menu');
    const layerCard = document.getElementById('layer-switcher-card');

    if (listWrapper && !listWrapper.contains(e.target) && listMenu) {
      listMenu.classList.add('hidden');
      const chevron = document.querySelector('.dropdown-chevron');
      if (chevron) chevron.style.transform = 'rotate(0deg)';
    }

    if (layerWrapper && !layerWrapper.contains(e.target) && layerCard) {
      layerCard.classList.add('hidden');
    }
  });
}

// ============================================================================
// Leaflet Map & Layer Switcher
// ============================================================================
function initMap() {
  currentLayerName = localStorage.getItem('blist_map_layer') || 'light';

  map = L.map('map', {
    zoomControl: true,
    tap: true
  }).setView([20.0, 0.0], 2);

  applyMapLayer(currentLayerName);

  // Click on map to add manual pin
  map.on('click', async (e) => {
    const lat = e.latlng.lat.toFixed(6);
    const lon = e.latlng.lng.toFixed(6);
    openManualPinModal(lat, lon);
  });
}

function applyMapLayer(layerKey) {
  const layerConf = MAP_LAYERS[layerKey] || MAP_LAYERS.light;
  
  if (currentTileLayer) {
    map.removeLayer(currentTileLayer);
  }

  currentTileLayer = L.tileLayer(layerConf.url, layerConf.options).addTo(map);
  currentLayerName = layerKey;
  localStorage.setItem('blist_map_layer', layerKey);

  // Update UI active buttons in switcher
  document.querySelectorAll('.layer-opt-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.layer === layerKey);
  });
}

function toggleLayerSwitcher() {
  const card = document.getElementById('layer-switcher-card');
  if (card) {
    card.classList.toggle('hidden');
  }
}

function switchMapLayer(layerKey) {
  applyMapLayer(layerKey);
  const card = document.getElementById('layer-switcher-card');
  if (card) card.classList.add('hidden');
}

// ============================================================================
// Multi-List & Trips Management
// ============================================================================
async function fetchLists() {
  try {
    const res = await fetch('/api/lists');
    const json = await res.json();
    if (json.success && json.data) {
      lists = json.data;
    } else {
      lists = [{ id: 1, name: 'My Bucket List', icon: '📍', created_at: '' }];
    }
  } catch (err) {
    console.error('Failed to fetch lists:', err);
    lists = [{ id: 1, name: 'My Bucket List', icon: '📍', created_at: '' }];
  }
}

async function fetchPins() {
  try {
    const res = await fetch('/api/pins');
    const json = await res.json();
    if (json.success && json.data) {
      allPins = json.data;
      renderAll();
      preloadWeatherForVisiblePins();
    }
  } catch (err) {
    console.error('Failed to fetch pins:', err);
    showToast('Failed to load saved places', 'error');
  }
}

function renderAll() {
  renderListsUI();
  renderCategories();
  renderPinList();
  renderMarkers();
  updateCounts();
  lucide.createIcons();
}

function toggleListDropdown() {
  const menu = document.getElementById('list-dropdown-menu');
  const chevron = document.querySelector('.dropdown-chevron');
  if (!menu) return;

  const isHidden = menu.classList.toggle('hidden');
  if (chevron) {
    chevron.style.transform = isHidden ? 'rotate(0deg)' : 'rotate(180deg)';
  }
}

function selectList(listId) {
  currentListFilter = String(listId);
  const menu = document.getElementById('list-dropdown-menu');
  if (menu) menu.classList.add('hidden');
  const chevron = document.querySelector('.dropdown-chevron');
  if (chevron) chevron.style.transform = 'rotate(0deg)';

  // Sync quick status tabs if standard view
  if (listId === 'bucket') {
    selectedStatus = 'bucket';
  } else if (listId === 'visited') {
    selectedStatus = 'visited';
  } else {
    selectedStatus = 'all';
  }

  document.querySelectorAll('.filter-tab').forEach(tab => {
    tab.classList.toggle('active', tab.dataset.status === selectedStatus);
  });

  renderAll();
  resetView();
}

function renderListsUI() {
  const container = document.getElementById('custom-lists-container');
  const formSelect = document.getElementById('form-custom-list');

  const activeIcon = document.getElementById('active-list-icon');
  const activeName = document.getElementById('active-list-name');
  const activeCount = document.getElementById('active-list-count');

  let activeTitle = 'All Places';
  let activeEmoji = '🗺️';
  let activeTotal = allPins.length;

  if (currentListFilter === 'bucket') {
    activeTitle = 'Bucket List';
    activeEmoji = '🎯';
    activeTotal = allPins.filter(p => !p.visited).length;
  } else if (currentListFilter === 'visited') {
    activeTitle = 'Visited Places';
    activeEmoji = '✅';
    activeTotal = allPins.filter(p => p.visited).length;
  } else if (currentListFilter !== 'all') {
    const listIdNum = parseInt(currentListFilter, 10);
    const list = lists.find(l => l.id === listIdNum);
    if (list) {
      activeTitle = list.name;
      activeEmoji = list.icon || '📁';
      activeTotal = allPins.filter(p => p.list_id === list.id).length;
    }
  }

  if (activeIcon) activeIcon.innerText = activeEmoji;
  if (activeName) activeName.innerText = activeTitle;
  if (activeCount) activeCount.innerText = activeTotal;

  // Render items in dropdown menu
  if (container) {
    container.innerHTML = lists.map(list => {
      const pinCount = allPins.filter(p => p.list_id === list.id).length;
      const isActive = currentListFilter === String(list.id);
      const isDefault = list.id === 1;

      return `
        <div class="list-dropdown-item ${isActive ? 'active' : ''}" onclick="selectList('${list.id}')">
          <span class="item-icon">${list.icon || '📁'}</span>
          <span class="item-title">${escapeHtml(list.name)}</span>
          <span class="item-badge">${pinCount}</span>
          ${!isDefault ? `
            <button class="btn-icon-sm" style="padding: 3px 5px; margin-left: 4px;" onclick="event.stopPropagation(); deleteList(${list.id})" title="Delete Trip/List">
              <i data-lucide="trash-2" style="width: 12px; height: 12px;"></i>
            </button>
          ` : ''}
        </div>
      `;
    }).join('');
  }

  // Populate Add / Edit Modal custom list select
  if (formSelect) {
    const currentVal = formSelect.value;
    formSelect.innerHTML = lists.map(l => `
      <option value="${l.id}">${l.icon || '📁'} ${escapeHtml(l.name)}</option>
    `).join('');
    if (currentVal && lists.some(l => String(l.id) === currentVal)) {
      formSelect.value = currentVal;
    }
  }

  // Update Standard Badges
  const bAll = document.getElementById('list-badge-all');
  const bBucket = document.getElementById('list-badge-bucket');
  const bVisited = document.getElementById('list-badge-visited');
  if (bAll) bAll.innerText = allPins.length;
  if (bBucket) bBucket.innerText = allPins.filter(p => !p.visited).length;
  if (bVisited) bVisited.innerText = allPins.filter(p => p.visited).length;
}

function openCreateListModal() {
  const dropdown = document.getElementById('list-dropdown-menu');
  if (dropdown) dropdown.classList.add('hidden');
  document.getElementById('list-name-input').value = '';
  document.getElementById('list-desc-input').value = '';
  selectEmoji('✈️');
  document.getElementById('create-list-modal').classList.remove('hidden');
}

function closeCreateListModal() {
  document.getElementById('create-list-modal').classList.add('hidden');
}

function selectEmoji(emoji) {
  document.getElementById('selected-list-emoji').value = emoji;
  document.querySelectorAll('.emoji-opt').forEach(btn => {
    btn.classList.toggle('active', btn.innerText.trim() === emoji);
  });
}

async function handleCreateListSubmit(e) {
  e.preventDefault();
  const name = document.getElementById('list-name-input').value.trim();
  const icon = document.getElementById('selected-list-emoji').value || '✈️';

  if (!name) return;

  try {
    const res = await fetch('/api/lists', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, icon })
    });

    const json = await res.json();
    if (json.success && json.data) {
      lists.push(json.data);
      closeCreateListModal();
      selectList(json.data.id);
      showToast(`Created trip "${icon} ${name}"!`, 'success');
    } else {
      showToast(json.error || 'Failed to create list', 'error');
    }
  } catch (err) {
    showToast('Failed to connect to server', 'error');
  }
}

async function deleteList(id) {
  const list = lists.find(l => l.id === id);
  if (!list) return;
  if (!confirm(`Delete list "${list.name}"? Pins in this list will be reassigned or deleted.`)) return;

  try {
    const res = await fetch(`/api/lists/${id}`, { method: 'DELETE' });
    const json = await res.json();
    if (json.success) {
      lists = lists.filter(l => l.id !== id);
      if (currentListFilter === String(id)) {
        selectList('all');
      } else {
        await fetchPins();
      }
      showToast('List deleted');
    }
  } catch (err) {
    showToast('Failed to delete list', 'error');
  }
}

function getListNameForPin(pin) {
  if (!pin.list_id) return null;
  const found = lists.find(l => l.id === pin.list_id);
  if (found && found.id !== 1) {
    return found;
  }
  return null;
}

// ============================================================================
// GPS Location & Distance Calculation
// ============================================================================
function handleLocateMe() {
  if (!navigator.geolocation) {
    showToast('⚠️ Geolocation is not supported by your browser', 'error');
    return;
  }

  const btn = document.getElementById('btn-locate-me');
  if (btn) btn.classList.add('active-locating');

  navigator.geolocation.getCurrentPosition(
    (position) => {
      if (btn) btn.classList.remove('active-locating');
      currentUserLocation = {
        lat: position.coords.latitude,
        lng: position.coords.longitude
      };

      updateUserLocationMarker();
      map.flyTo([currentUserLocation.lat, currentUserLocation.lng], 14, { duration: 1.2 });
      showToast('📍 Current location found!');
      renderPinList();
      lucide.createIcons();
    },
    (error) => {
      if (btn) btn.classList.remove('active-locating');
      let msg = 'Could not access GPS coordinates';
      if (error.code === 1) msg = 'Location permission denied';
      else if (error.code === 2) msg = 'Position unavailable';
      else if (error.code === 3) msg = 'Location request timed out';
      showToast('⚠️ ' + msg, 'error');
    },
    { enableHighAccuracy: true, timeout: 10000 }
  );
}

function updateUserLocationMarker() {
  if (!currentUserLocation) return;

  if (userLocationMarker) {
    map.removeLayer(userLocationMarker);
  }

  const userIcon = L.divIcon({
    className: 'user-location-marker-container',
    html: `
      <div class="user-location-pulse"></div>
      <div class="user-location-dot"></div>
    `,
    iconSize: [38, 38],
    iconAnchor: [19, 19]
  });

  userLocationMarker = L.marker([currentUserLocation.lat, currentUserLocation.lng], {
    icon: userIcon,
    zIndexOffset: 1000
  }).addTo(map);

  userLocationMarker.bindPopup('<b>📍 You are here</b>');
}

function calculateDistance(lat1, lon1, lat2, lon2) {
  const R = 6371; // Earth's radius in km
  const dLat = (lat2 - lat1) * Math.PI / 180;
  const dLon = (lon2 - lon1) * Math.PI / 180;
  const a = Math.sin(dLat / 2) * Math.sin(dLat / 2) +
            Math.cos(lat1 * Math.PI / 180) * Math.cos(lat2 * Math.PI / 180) *
            Math.sin(dLon / 2) * Math.sin(dLon / 2);
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
  return R * c;
}

function formatDistance(dKm) {
  if (dKm < 1) {
    return `${Math.round(dKm * 1000)} m away`;
  } else {
    const mi = (dKm * 0.621371).toFixed(1);
    return `${mi} mi away (${dKm.toFixed(1)} km)`;
  }
}

// ============================================================================
// Live Weather Integration (Open-Meteo)
// ============================================================================
async function fetchWeather(lat, lon) {
  const key = `${lat.toFixed(2)},${lon.toFixed(2)}`;
  if (weatherCache[key]) {
    return weatherCache[key];
  }

  try {
    const res = await fetch(`https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}&current_weather=true`);
    const data = await res.json();
    if (data && data.current_weather) {
      const cw = data.current_weather;
      const codeInfo = WEATHER_CODE_MAP[cw.weathercode] || { icon: '🌤️', text: 'Weather' };
      const tempC = Math.round(cw.temperature);
      const tempF = Math.round(cw.temperature * 9 / 5 + 32);
      
      const weatherInfo = {
        icon: codeInfo.icon,
        text: codeInfo.text,
        tempC: tempC,
        tempF: tempF,
        display: `${codeInfo.icon} ${tempF}°F / ${tempC}°C`
      };
      
      weatherCache[key] = weatherInfo;
      return weatherInfo;
    }
  } catch (err) {
    // Non-critical, fail gracefully
  }
  return null;
}

async function preloadWeatherForVisiblePins() {
  const pins = getFilteredPins().slice(0, 20);
  for (const pin of pins) {
    fetchWeather(pin.latitude, pin.longitude).then(w => {
      if (w) {
        const badge = document.getElementById(`weather-badge-${pin.id}`);
        if (badge) {
          badge.innerText = `${w.icon} ${w.tempF}°F`;
          badge.classList.remove('hidden');
        }
      }
    });
  }
}

// ============================================================================
// Filtering & Sorting Logic
// ============================================================================
function getFilteredPins() {
  let listPins = allPins;

  // Custom List / Trip Filter
  if (currentListFilter === 'bucket') {
    listPins = allPins.filter(p => !p.visited);
  } else if (currentListFilter === 'visited') {
    listPins = allPins.filter(p => p.visited);
  } else if (currentListFilter !== 'all') {
    const listIdNum = parseInt(currentListFilter, 10);
    listPins = allPins.filter(p => p.list_id === listIdNum);
  }

  return listPins.filter(pin => {
    // Quick Status Filter
    if (selectedStatus === 'bucket' && pin.visited) return false;
    if (selectedStatus === 'visited' && !pin.visited) return false;

    // Category Filter
    if (selectedCategory !== 'All' && pin.category !== selectedCategory) return false;

    // Search Query Filter
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      const matchTitle = pin.title && pin.title.toLowerCase().includes(q);
      const matchAddress = pin.address && pin.address.toLowerCase().includes(q);
      const matchNotes = pin.notes && pin.notes.toLowerCase().includes(q);
      const matchCategory = pin.category && pin.category.toLowerCase().includes(q);
      if (!matchTitle && !matchAddress && !matchNotes && !matchCategory) return false;
    }

    return true;
  }).sort((a, b) => {
    // Sort
    if (currentSort === 'nearest' && currentUserLocation) {
      const distA = calculateDistance(currentUserLocation.lat, currentUserLocation.lng, a.latitude, a.longitude);
      const distB = calculateDistance(currentUserLocation.lat, currentUserLocation.lng, b.latitude, b.longitude);
      return distA - distB;
    } else if (currentSort === 'az') {
      return (a.title || '').localeCompare(b.title || '');
    } else if (currentSort === 'category') {
      return (a.category || '').localeCompare(b.category || '');
    } else {
      // 'newest' default
      return (b.id || 0) - (a.id || 0);
    }
  });
}

function renderCategories() {
  const container = document.getElementById('categories-bar');
  if (!container) return;

  const categories = ['All', ...new Set(allPins.map(p => p.category).filter(Boolean))];
  
  container.innerHTML = categories.map(cat => `
    <button class="cat-chip ${selectedCategory === cat ? 'active' : ''}" onclick="setCategoryFilter('${escapeHtml(cat)}')">
      ${escapeHtml(cat)}
    </button>
  `).join('');
}

// ============================================================================
// Pin List & Marker Rendering
// ============================================================================
function renderPinList() {
  const container = document.getElementById('pin-list');
  const filtered = getFilteredPins();

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

  container.innerHTML = filtered.map(pin => {
    let distanceStr = '';
    if (currentUserLocation) {
      const dKm = calculateDistance(currentUserLocation.lat, currentUserLocation.lng, pin.latitude, pin.longitude);
      distanceStr = formatDistance(dKm);
    }

    const assignedList = getListNameForPin(pin);
    const weatherCached = weatherCache[`${pin.latitude.toFixed(2)},${pin.longitude.toFixed(2)}`];

    return `
      <div class="pin-card ${pin.visited ? 'visited-card' : ''}" onclick="handlePinCardClick(${pin.id})" id="card-pin-${pin.id}">
        <div class="pin-card-header">
          <div class="pin-card-title">${escapeHtml(pin.title)}</div>
          <div class="badges-row">
            <span class="pin-badge ${pin.visited ? 'badge-visited' : ''}">
              ${pin.visited ? '✅ Visited' : escapeHtml(pin.category || 'Place')}
            </span>
            ${assignedList ? `
              <span class="pin-badge badge-list">${assignedList.icon || '📁'} ${escapeHtml(assignedList.name)}</span>
            ` : ''}
            ${distanceStr ? `
              <span class="pin-badge badge-distance">📍 ${distanceStr}</span>
            ` : ''}
            <span class="pin-badge badge-weather ${weatherCached ? '' : 'hidden'}" id="weather-badge-${pin.id}">
              ${weatherCached ? `${weatherCached.icon} ${weatherCached.tempF}°F` : ''}
            </span>
          </div>
        </div>

        ${pin.image_url ? `
          <img src="${escapeHtml(pin.image_url)}" class="pin-card-thumb" alt="${escapeHtml(pin.title)}" onerror="this.style.display='none'">
        ` : ''}

        ${pin.address ? `
          <div class="pin-card-address">
            <i data-lucide="map-pin" style="width: 13px; height: 13px; flex-shrink: 0;"></i>
            <span>${escapeHtml(pin.address)}</span>
          </div>
        ` : ''}

        ${pin.notes ? `
          <div class="pin-card-notes">
            "${escapeHtml(pin.notes)}"
          </div>
        ` : ''}

        <div class="pin-card-footer" onclick="event.stopPropagation()">
          <label class="status-toggle">
            <input type="checkbox" ${pin.visited ? 'checked' : ''} onchange="toggleVisited(${pin.id})">
            <span>${pin.visited ? 'Visited' : 'Mark Visited'}</span>
          </label>

          <div class="card-action-btns">
            <a href="https://www.google.com/maps/dir/?api=1&destination=${pin.latitude},${pin.longitude}" target="_blank" rel="noopener noreferrer" class="btn-icon-sm" title="Get Directions">
              <i data-lucide="navigation" style="width: 14px; height: 14px;"></i>
            </a>
            <button class="btn-icon-sm" onclick="sharePin(${pin.id})" title="Share Place">
              <i data-lucide="share-2" style="width: 14px; height: 14px;"></i>
            </button>
            ${pin.source_url ? `
              <a href="${escapeHtml(pin.source_url)}" target="_blank" rel="noopener noreferrer" class="btn-icon-sm" title="Open Source Link">
                <i data-lucide="external-link" style="width: 14px; height: 14px;"></i>
              </a>
            ` : ''}
            <button class="btn-icon-sm" onclick="openEditPinModal(${pin.id})" title="Edit Place">
              <i data-lucide="edit-3" style="width: 14px; height: 14px;"></i>
            </button>
            <button class="btn-icon-sm delete-btn" onclick="deletePin(${pin.id})" title="Delete Place">
              <i data-lucide="trash-2" style="width: 14px; height: 14px;"></i>
            </button>
          </div>
        </div>
      </div>
    `;
  }).join('');
}

function renderMarkers() {
  Object.values(markers).forEach(m => map.removeLayer(m));
  markers = {};

  const filtered = getFilteredPins();

  filtered.forEach(pin => {
    const customIcon = L.divIcon({
      className: 'custom-pin-container',
      html: `
        <div class="custom-pin-marker ${pin.visited ? 'visited-pin' : ''}" id="marker-elem-${pin.id}">
          <i data-lucide="${CATEGORY_ICONS[pin.category] || 'map-pin'}"></i>
        </div>
      `,
      iconSize: [32, 32],
      iconAnchor: [16, 32],
      popupAnchor: [0, -32]
    });

    const marker = L.marker([pin.latitude, pin.longitude], { icon: customIcon }).addTo(map);

    marker.on('click', () => {
      loadAndRenderPopup(marker, pin);
    });

    markers[pin.id] = marker;
  });

  lucide.createIcons();
}

async function loadAndRenderPopup(marker, pin) {
  let distanceStr = '';
  if (currentUserLocation) {
    const dKm = calculateDistance(currentUserLocation.lat, currentUserLocation.lng, pin.latitude, pin.longitude);
    distanceStr = formatDistance(dKm);
  }

  const assignedList = getListNameForPin(pin);
  const weather = await fetchWeather(pin.latitude, pin.longitude);

  const popupHtml = `
    <div class="pin-popup">
      ${pin.image_url ? `<img src="${escapeHtml(pin.image_url)}" class="popup-img" alt="${escapeHtml(pin.title)}" onerror="this.style.display='none'">` : ''}
      <div class="popup-body">
        <div class="popup-badges-row">
          <span class="pin-badge ${pin.visited ? 'badge-visited' : ''}">
            ${pin.visited ? '✅ Visited' : escapeHtml(pin.category || 'Place')}
          </span>
          ${assignedList ? `
            <span class="pin-badge badge-list">${assignedList.icon || '📁'} ${escapeHtml(assignedList.name)}</span>
          ` : ''}
          ${weather ? `
            <span class="pin-badge badge-weather">${weather.display}</span>
          ` : ''}
          ${distanceStr ? `
            <span class="pin-badge badge-distance">📍 ${distanceStr}</span>
          ` : ''}
        </div>
        
        <div class="popup-title">${escapeHtml(pin.title)}</div>
        ${pin.address ? `<div class="popup-address"><i data-lucide="map-pin" style="width: 12px; height: 12px;"></i> ${escapeHtml(pin.address)}</div>` : ''}
        ${pin.notes ? `<div class="popup-notes">"${escapeHtml(pin.notes)}"</div>` : ''}

        <div class="popup-actions-grid">
          <a href="https://www.google.com/maps/dir/?api=1&destination=${pin.latitude},${pin.longitude}" target="_blank" rel="noopener noreferrer" class="btn-popup-action primary-action">
            <i data-lucide="navigation"></i> Directions
          </a>
          <button class="btn-popup-action" onclick="sharePin(${pin.id})">
            <i data-lucide="share-2"></i> Share
          </button>
          ${pin.source_url ? `
            <a href="${escapeHtml(pin.source_url)}" target="_blank" rel="noopener noreferrer" class="btn-popup-action">
              <i data-lucide="external-link"></i> Link
            </a>
          ` : ''}
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
      </div>
    </div>
  `;

  marker.bindPopup(popupHtml).openPopup();
  lucide.createIcons();
}

// ============================================================================
// Interactions & Navigation
// ============================================================================
function handlePinCardClick(id) {
  if (window.innerWidth <= 768) {
    showMobileView('map');
  }
  flyToPin(id);
}

function flyToPin(id) {
  const pin = allPins.find(p => p.id === id);
  if (!pin) return;

  map.flyTo([pin.latitude, pin.longitude], 15, { duration: 1.2 });
  if (markers[id]) {
    setTimeout(() => {
      loadAndRenderPopup(markers[id], pin);
    }, 600);
  }
}

function resetView() {
  const filtered = getFilteredPins();
  if (filtered.length > 0) {
    const latLngs = filtered.map(p => [p.latitude, p.longitude]);
    map.fitBounds(L.latLngBounds(latLngs).pad(0.18), { maxZoom: 15 });
  } else {
    map.setView([20.0, 0.0], 2);
  }
}

function updateCounts() {
  const total = allPins.length;
  const visited = allPins.filter(p => p.visited).length;
  const bucket = total - visited;

  const elAll = document.getElementById('count-all');
  const elBucket = document.getElementById('count-bucket');
  const elVisited = document.getElementById('count-visited');
  const elMobile = document.getElementById('mobile-count');

  if (elAll) elAll.innerText = total;
  if (elBucket) elBucket.innerText = bucket;
  if (elVisited) elVisited.innerText = visited;
  if (elMobile) elMobile.innerText = getFilteredPins().length;
}

// Sidebar Drawer & Mobile View
function toggleSidebar() {
  const sidebar = document.getElementById('sidebar');
  if (window.innerWidth <= 768) {
    if (sidebar.classList.contains('mobile-open')) {
      showMobileView('map');
    } else {
      showMobileView('list');
    }
  } else {
    sidebar.classList.toggle('collapsed');
    setTimeout(() => map.invalidateSize(), 300);
  }
}

function showMobileView(view) {
  currentMobileView = view;
  const sidebar = document.getElementById('sidebar');
  const btnMap = document.getElementById('btn-show-map');
  const btnList = document.getElementById('btn-show-list');

  if (view === 'list') {
    sidebar.classList.add('mobile-open');
    if (btnMap) btnMap.classList.remove('active');
    if (btnList) btnList.classList.add('active');
  } else {
    sidebar.classList.remove('mobile-open');
    if (btnMap) btnMap.classList.add('active');
    if (btnList) btnList.classList.remove('active');
    setTimeout(() => map.invalidateSize(), 150);
  }
}

// Filter Actions
function setStatusFilter(status) {
  selectedStatus = status;
  document.querySelectorAll('.filter-tab').forEach(tab => {
    tab.classList.toggle('active', tab.dataset.status === status);
  });
  renderPinList();
  renderMarkers();
  updateCounts();
  lucide.createIcons();
}

function setCategoryFilter(cat) {
  selectedCategory = cat;
  renderCategories();
  renderPinList();
  renderMarkers();
  updateCounts();
  lucide.createIcons();
}

function handleSearch(e) {
  searchQuery = e.target.value;
  const clearBtn = document.getElementById('clear-search-btn');
  if (clearBtn) {
    clearBtn.classList.toggle('hidden', !searchQuery);
  }
  renderPinList();
  renderMarkers();
  updateCounts();
  lucide.createIcons();
}

function clearSearch() {
  searchQuery = '';
  const searchInput = document.getElementById('search-input');
  if (searchInput) searchInput.value = '';
  const clearBtn = document.getElementById('clear-search-btn');
  if (clearBtn) clearBtn.classList.add('hidden');
  renderPinList();
  renderMarkers();
  updateCounts();
  lucide.createIcons();
}

function handleSortChange(e) {
  currentSort = e.target.value;
  if (currentSort === 'nearest' && !currentUserLocation) {
    handleLocateMe();
  }
  renderPinList();
  lucide.createIcons();
}

// ============================================================================
// "Surprise Me" Random Pick Feature
// ============================================================================
function handleSurpriseMe() {
  const pool = allPins.filter(p => !p.visited);
  const candidates = pool.length > 0 ? pool : allPins;

  if (candidates.length === 0) {
    showToast('No places to choose from! Save some links first 🗺️', 'error');
    return;
  }

  // Visual button bounce
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
    showMobileView('map');
  }

  map.flyTo([picked.latitude, picked.longitude], 16, { duration: 1.5 });

  setTimeout(() => {
    if (markers[picked.id]) {
      loadAndRenderPopup(markers[picked.id], picked);
      
      const elem = document.getElementById(`marker-elem-${picked.id}`);
      if (elem) {
        elem.classList.add('surprise-pin');
        setTimeout(() => elem.classList.remove('surprise-pin'), 4000);
      }
    }

    const card = document.getElementById(`card-pin-${picked.id}`);
    if (card) {
      card.scrollIntoView({ behavior: 'smooth', block: 'center' });
      card.classList.add('highlight-card');
      setTimeout(() => card.classList.remove('highlight-card'), 3000);
    }

    showToast(`🎲 Surprise Pick: "${picked.title}"!`);
  }, 800);
}

// ============================================================================
// Native Sharing & Clipboard
// ============================================================================
async function sharePin(id) {
  const pin = allPins.find(p => p.id === id);
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
      showToast('Shared successfully!');
      return;
    } catch (err) {
      if (err.name === 'AbortError') return;
    }
  }

  // Fallback to clipboard
  const clipboardText = `${pin.title}\n${pin.address || ''}\n${shareUrl}`;
  try {
    await navigator.clipboard.writeText(clipboardText);
    showToast('📋 Place details copied to clipboard!');
  } catch (_) {
    showToast('Failed to copy to clipboard', 'error');
  }
}

// ============================================================================
// Link Saving (Omni Bar) & CRUD
// ============================================================================
async function handleSaveLinkSubmit(e) {
  e.preventDefault();
  const input = document.getElementById('save-url-input');
  const url = input.value.trim();
  if (!url) return;

  const overlay = document.getElementById('loading-overlay');
  overlay.classList.remove('hidden');

  let targetListId = 1;
  if (currentListFilter !== 'all' && currentListFilter !== 'bucket' && currentListFilter !== 'visited') {
    targetListId = parseInt(currentListFilter, 10) || 1;
  }

  try {
    const res = await fetch('/api/pins/ingest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url, list_id: targetListId })
    });

    const json = await res.json();
    overlay.classList.add('hidden');

    if (json.success && json.data) {
      input.value = '';
      allPins.unshift(json.data);

      renderAll();
      showToast(`✨ Saved "${json.data.title}"!`, 'success');

      if (window.innerWidth <= 768) {
        showMobileView('map');
      }
      flyToPin(json.data.id);
    } else {
      showToast(json.error || 'Failed to extract place details. You can add it manually with "+ Add Place"!', 'error');
    }
  } catch (err) {
    overlay.classList.add('hidden');
    showToast('Error connecting to server. Please check your network.', 'error');
  }
}

async function toggleVisited(id) {
  try {
    const res = await fetch(`/api/pins/${id}/visited`, { method: 'PATCH' });
    const json = await res.json();
    if (json.success && json.data) {
      const idx = allPins.findIndex(p => p.id === id);
      if (idx !== -1) {
        allPins[idx] = json.data;
        renderAll();
        showToast(json.data.visited ? '🎉 Marked as visited!' : '🎯 Added back to bucket list');
      }
    }
  } catch (err) {
    console.error('Failed to toggle visited:', err);
    showToast('Failed to update status', 'error');
  }
}

async function deletePin(id) {
  const pin = allPins.find(p => p.id === id);
  const title = pin ? pin.title : 'this place';
  if (!confirm(`Remove "${title}" from your bucket list?`)) return;

  try {
    const res = await fetch(`/api/pins/${id}`, { method: 'DELETE' });
    const json = await res.json();
    if (json.success) {
      allPins = allPins.filter(p => p.id !== id);
      renderAll();
      showToast('Place deleted');
    }
  } catch (err) {
    console.error('Failed to delete pin:', err);
    showToast('Failed to delete place', 'error');
  }
}

// ============================================================================
// Manual Add / Edit Pin Modal
// ============================================================================
async function openManualPinModal(lat = '', lon = '') {
  document.getElementById('modal-title').innerText = 'Add Place';
  document.getElementById('pin-id').value = '';
  document.getElementById('form-title').value = '';
  document.getElementById('form-lat').value = lat;
  document.getElementById('form-lon').value = lon;
  document.getElementById('form-category').value = 'General';
  document.getElementById('form-visited').value = 'false';
  document.getElementById('form-address').value = '';
  document.getElementById('form-image').value = '';
  document.getElementById('form-source').value = '';
  document.getElementById('form-notes').value = '';

  const formList = document.getElementById('form-custom-list');
  if (formList) {
    if (currentListFilter !== 'all' && currentListFilter !== 'bucket' && currentListFilter !== 'visited') {
      formList.value = currentListFilter;
    } else if (lists.length > 0) {
      formList.value = lists[0].id;
    }
  }

  if (lat && lon) {
    try {
      const res = await fetch(`https://nominatim.openstreetmap.org/reverse?format=json&lat=${lat}&lon=${lon}`);
      const data = await res.json();
      if (data && data.display_name) {
        document.getElementById('form-address').value = data.display_name;
        if (!document.getElementById('form-title').value) {
          const parts = data.display_name.split(',');
          document.getElementById('form-title').value = parts[0].trim();
        }
      }
    } catch (_) {}
  }

  document.getElementById('pin-modal').classList.remove('hidden');
}

function openEditPinModal(id) {
  const pin = allPins.find(p => p.id === id);
  if (!pin) return;

  document.getElementById('modal-title').innerText = 'Edit Place';
  document.getElementById('pin-id').value = pin.id;
  document.getElementById('form-title').value = pin.title;
  document.getElementById('form-lat').value = pin.latitude;
  document.getElementById('form-lon').value = pin.longitude;
  document.getElementById('form-category').value = pin.category || 'General';
  document.getElementById('form-visited').value = pin.visited ? 'true' : 'false';
  document.getElementById('form-address').value = pin.address || '';
  document.getElementById('form-image').value = pin.image_url || '';
  document.getElementById('form-source').value = pin.source_url || '';
  document.getElementById('form-notes').value = pin.notes || '';

  const formList = document.getElementById('form-custom-list');
  if (formList) {
    formList.value = pin.list_id || (lists[0] ? lists[0].id : 1);
  }

  document.getElementById('pin-modal').classList.remove('hidden');
}

function closePinModal() {
  document.getElementById('pin-modal').classList.add('hidden');
}

async function handlePinFormSubmit(e) {
  e.preventDefault();
  const id = document.getElementById('pin-id').value;
  const selectedListId = parseInt(document.getElementById('form-custom-list').value, 10) || 1;

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
    notes: document.getElementById('form-notes').value.trim() || null,
  };

  try {
    let res;
    if (id) {
      res = await fetch(`/api/pins/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    } else {
      res = await fetch('/api/pins', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    }

    const json = await res.json();
    if (json.success && json.data) {
      closePinModal();
      const savedPin = json.data;

      if (id) {
        const idx = allPins.findIndex(p => p.id === parseInt(id, 10));
        if (idx !== -1) allPins[idx] = savedPin;
      } else {
        allPins.unshift(savedPin);
      }

      renderAll();
      showToast(id ? 'Place updated!' : 'Place added!', 'success');

      if (window.innerWidth <= 768) {
        showMobileView('map');
      }
      flyToPin(savedPin.id);
    } else {
      showToast(json.error || 'Failed to save place', 'error');
    }
  } catch (err) {
    showToast('Error connecting to server', 'error');
  }
}

function handleModalBackdropClick(e, modalId) {
  if (e.target.id === modalId) {
    document.getElementById(modalId).classList.add('hidden');
  }
}

// ============================================================================
// Exporting Data
// ============================================================================
async function exportData(format = 'geojson') {
  try {
    const res = await fetch(`/api/export/${format}`);
    const data = await res.json();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `blist_${new Date().toISOString().split('T')[0]}.${format === 'geojson' ? 'geojson' : 'json'}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    showToast('Export file downloaded!');
  } catch (err) {
    showToast('Failed to export data', 'error');
  }
}

// ============================================================================
// Toast Notification Utility
// ============================================================================
function showToast(message, type = 'info') {
  const container = document.getElementById('toast-container');
  if (!container) return;

  const toast = document.createElement('div');
  toast.className = `toast ${type === 'error' ? 'toast-error' : type === 'success' ? 'toast-success' : ''}`;
  toast.innerText = message;

  container.appendChild(toast);
  setTimeout(() => {
    if (toast.parentNode) toast.parentNode.removeChild(toast);
  }, 3000);
}

// Utility: HTML Escape
function escapeHtml(str) {
  if (!str) return '';
  return str.replace(/[&<>"']/g, m => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;'
  })[m]);
}
