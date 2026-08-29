// bList - Map Bucket List Frontend Application

let map;
let markers = {};
let allPins = [];
let selectedCategory = 'All';
let selectedStatus = 'all';
let searchQuery = '';
let currentMobileView = 'map'; // 'map' or 'list'

const CATEGORY_ICONS = {
  'Food & Drink': 'utensils',
  'Cafe': 'coffee',
  'Sightseeing': 'landmark',
  'Nature & Outdoors': 'trees',
  'Hotel & Stay': 'bed',
  'Shopping': 'shopping-bag',
  'General': 'map-pin',
  'Social': 'share-2',
  'Place': 'map-pin'
};

document.addEventListener('DOMContentLoaded', () => {
  initMap();
  fetchPins();
  lucide.createIcons();
});

// Initialize Leaflet Map
function initMap() {
  map = L.map('map', {
    zoomControl: true,
  }).setView([20.0, 0.0], 2);

  L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
    maxZoom: 19,
    attribution: '&copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap</a>'
  }).addTo(map);

  // Click on map to drop manual pin
  map.on('click', async (e) => {
    const lat = e.latlng.lat.toFixed(6);
    const lon = e.latlng.lng.toFixed(6);
    openManualPinModal(lat, lon);
  });
}

// Fetch all pins from API
async function fetchPins() {
  try {
    const res = await fetch('/api/pins');
    const json = await res.json();
    if (json.success && json.data) {
      allPins = json.data;
      renderAll();
    }
  } catch (err) {
    console.error('Failed to fetch pins:', err);
  }
}

// Main Render Dispatcher
function renderAll() {
  renderCategories();
  renderPinList();
  renderMarkers();
  updateCounts();
  lucide.createIcons();
}

// Filter pins based on active filters
function getFilteredPins() {
  return allPins.filter(pin => {
    // Status Filter
    if (selectedStatus === 'bucket' && pin.visited) return false;
    if (selectedStatus === 'visited' && !pin.visited) return false;

    // Category Filter
    if (selectedCategory !== 'All' && pin.category !== selectedCategory) return false;

    // Search Query
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      const matchTitle = pin.title && pin.title.toLowerCase().includes(q);
      const matchAddress = pin.address && pin.address.toLowerCase().includes(q);
      const matchNotes = pin.notes && pin.notes.toLowerCase().includes(q);
      const matchCategory = pin.category && pin.category.toLowerCase().includes(q);
      if (!matchTitle && !matchAddress && !matchNotes && !matchCategory) return false;
    }

    return true;
  });
}

// Render Category Chips
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

// Render Sidebar List of Pin Cards
function renderPinList() {
  const container = document.getElementById('pin-list');
  const filtered = getFilteredPins();

  if (filtered.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">📍</div>
        <h3>No pins match your filters</h3>
        <p>Try clearing your search query or selecting "All" categories.</p>
      </div>
    `;
    return;
  }

  container.innerHTML = filtered.map(pin => `
    <div class="pin-card ${pin.visited ? 'visited-card' : ''}" onclick="handlePinCardClick(${pin.id})" id="card-pin-${pin.id}">
      <div class="pin-card-header">
        <div class="pin-card-title">${escapeHtml(pin.title)}</div>
        <span class="pin-badge ${pin.visited ? 'badge-visited' : ''}">
          ${pin.visited ? '✅ Visited' : escapeHtml(pin.category)}
        </span>
      </div>

      ${pin.image_url ? `<img src="${escapeHtml(pin.image_url)}" class="pin-card-thumb" alt="${escapeHtml(pin.title)}" onerror="this.style.display='none'">` : ''}

      ${pin.address ? `
        <div class="pin-card-address">
          <i data-lucide="map-pin" style="width: 13px; height: 13px;"></i>
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
          ${pin.source_url ? `
            <a href="${escapeHtml(pin.source_url)}" target="_blank" rel="noopener noreferrer" class="btn-icon-sm" title="Open Source">
              <i data-lucide="external-link" style="width: 14px; height: 14px;"></i>
            </a>
          ` : ''}
          <button class="btn-icon-sm" onclick="openEditPinModal(${pin.id})" title="Edit Pin">
            <i data-lucide="edit-3" style="width: 14px; height: 14px;"></i>
          </button>
          <button class="btn-icon-sm" onclick="deletePin(${pin.id})" title="Delete Pin">
            <i data-lucide="trash-2" style="width: 14px; height: 14px;"></i>
          </button>
        </div>
      </div>
    </div>
  `).join('');
}

// Render Markers on Map
function renderMarkers() {
  Object.values(markers).forEach(m => map.removeLayer(m));
  markers = {};

  const filtered = getFilteredPins();

  filtered.forEach(pin => {
    const customIcon = L.divIcon({
      className: 'custom-pin-container',
      html: `
        <div class="custom-pin-marker ${pin.visited ? 'visited-pin' : ''}">
          <i data-lucide="${CATEGORY_ICONS[pin.category] || 'map-pin'}"></i>
        </div>
      `,
      iconSize: [32, 32],
      iconAnchor: [16, 32],
      popupAnchor: [0, -30]
    });

    const marker = L.marker([pin.latitude, pin.longitude], { icon: customIcon }).addTo(map);

    const popupHtml = `
      <div class="pin-popup">
        ${pin.image_url ? `<img src="${escapeHtml(pin.image_url)}" class="popup-img" alt="${escapeHtml(pin.title)}" onerror="this.style.display='none'">` : ''}
        <div class="popup-body">
          <div class="popup-title">${escapeHtml(pin.title)}</div>
          ${pin.address ? `<div class="popup-address">${escapeHtml(pin.address)}</div>` : ''}
          ${pin.notes ? `<div class="popup-notes">${escapeHtml(pin.notes)}</div>` : ''}
          <div class="popup-actions">
            <label class="status-toggle">
              <input type="checkbox" ${pin.visited ? 'checked' : ''} onchange="toggleVisited(${pin.id})">
              <span>${pin.visited ? 'Visited' : 'Mark Visited'}</span>
            </label>
            ${pin.source_url ? `<a href="${escapeHtml(pin.source_url)}" target="_blank" class="btn btn-ghost" style="padding: 4px 8px; font-size: 11px;">Source ↗</a>` : ''}
          </div>
        </div>
      </div>
    `;

    marker.bindPopup(popupHtml);
    markers[pin.id] = marker;
  });

  lucide.createIcons();
}

function handlePinCardClick(id) {
  if (window.innerWidth <= 768) {
    showMobileView('map');
  }
  flyToPin(id);
}

// Fly to specific pin
function flyToPin(id) {
  const pin = allPins.find(p => p.id === id);
  if (!pin) return;

  map.flyTo([pin.latitude, pin.longitude], 15, { duration: 1.2 });
  if (markers[id]) {
    setTimeout(() => {
      markers[id].openPopup();
    }, 600);
  }
}

// Update Top Badge Counts
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
  if (elMobile) elMobile.innerText = total;
}

// Sidebar Toggles
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
    map.invalidateSize();
  }
}

function resetView() {
  if (allPins.length > 0) {
    const group = new L.featureGroup(Object.values(markers));
    map.fitBounds(group.getBounds().pad(0.2));
  } else {
    map.setView([20.0, 0.0], 2);
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
}

function setCategoryFilter(cat) {
  selectedCategory = cat;
  renderCategories();
  renderPinList();
  renderMarkers();
}

function handleSearch(e) {
  searchQuery = e.target.value;
  renderPinList();
  renderMarkers();
}

// Omni Link Ingestion Handler
async function handleIngestSubmit(e) {
  e.preventDefault();
  const input = document.getElementById('ingest-url-input');
  const url = input.value.trim();
  if (!url) return;

  const overlay = document.getElementById('loading-overlay');
  overlay.classList.remove('hidden');

  try {
    const res = await fetch('/api/pins/ingest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url })
    });

    const json = await res.json();
    overlay.classList.add('hidden');

    if (json.success && json.data) {
      input.value = '';
      allPins.unshift(json.data);
      renderAll();
      if (window.innerWidth <= 768) {
        showMobileView('map');
      }
      flyToPin(json.data.id);
    } else {
      alert(json.error || 'Failed to ingest URL. You can still add it manually with "+ Add Pin"!');
    }
  } catch (err) {
    overlay.classList.add('hidden');
    alert('Error connecting to server. Please check your connection.');
  }
}

// Toggle Visited Status
async function toggleVisited(id) {
  try {
    const res = await fetch(`/api/pins/${id}/visited`, { method: 'PATCH' });
    const json = await res.json();
    if (json.success && json.data) {
      const idx = allPins.findIndex(p => p.id === id);
      if (idx !== -1) {
        allPins[idx] = json.data;
        renderAll();
      }
    }
  } catch (err) {
    console.error('Failed to toggle visited:', err);
  }
}

// Delete Pin
async function deletePin(id) {
  if (!confirm('Delete this pin from your bucket list?')) return;

  try {
    const res = await fetch(`/api/pins/${id}`, { method: 'DELETE' });
    const json = await res.json();
    if (json.success) {
      allPins = allPins.filter(p => p.id !== id);
      renderAll();
    }
  } catch (err) {
    console.error('Failed to delete pin:', err);
  }
}

// Manual Pin & Edit Modal Handlers
async function openManualPinModal(lat = '', lon = '') {
  document.getElementById('modal-title').innerText = 'Add New Pin';
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

  document.getElementById('modal-title').innerText = 'Edit Pin';
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

  document.getElementById('pin-modal').classList.remove('hidden');
}

function closePinModal() {
  document.getElementById('pin-modal').classList.add('hidden');
}

async function handlePinFormSubmit(e) {
  e.preventDefault();
  const id = document.getElementById('pin-id').value;
  const payload = {
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
      if (id) {
        const idx = allPins.findIndex(p => p.id === parseInt(id, 10));
        if (idx !== -1) allPins[idx] = json.data;
      } else {
        allPins.unshift(json.data);
      }
      renderAll();
      if (window.innerWidth <= 768) {
        showMobileView('map');
      }
      flyToPin(json.data.id);
    } else {
      alert(json.error || 'Failed to save pin');
    }
  } catch (err) {
    alert('Error connecting to server');
  }
}

// Export GeoJSON / JSON
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
  } catch (err) {
    alert('Failed to export data');
  }
}

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
