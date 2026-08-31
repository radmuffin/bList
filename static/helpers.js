/**
 * bList - Frontend Helper Functions & Utilities
 * Shared between the browser application and automated test suites.
 */

(function (root, factory) {
  if (typeof module === 'object' && module.exports) {
    // Node.js CommonJS
    module.exports = factory();
  } else {
    // Browser global
    root.bListHelpers = factory();
  }
})(typeof self !== 'undefined' ? self : this, function () {

  /**
   * Escapes HTML entities to protect against XSS injection.
   * Handles strings, numbers, null, undefined, and objects gracefully.
   */
  function escapeHtml(str) {
    if (str === null || str === undefined) return '';
    return String(str).replace(/[&<>"']/g, function (m) {
      switch (m) {
        case '&': return '&amp;';
        case '<': return '&lt;';
        case '>': return '&gt;';
        case '"': return '&quot;';
        case "'": return '&#39;';
        default: return m;
      }
    });
  }

  /**
   * Validates if a string is a safe HTTP or HTTPS URL.
   */
  function isValidHttpUrl(urlString) {
    if (!urlString || typeof urlString !== 'string') return false;
    const trimmed = urlString.trim();
    if (!trimmed) return false;
    try {
      const parsed = new URL(trimmed);
      return parsed.protocol === 'http:' || parsed.protocol === 'https:';
    } catch (_) {
      return false;
    }
  }

  /**
   * Sanitizes a URL, disallowing javascript:, data:, vbscript:, and invalid schemas.
   * Returns clean sanitized URL or empty string if invalid/unsafe.
   */
  function sanitizeUrl(urlString) {
    if (!urlString || typeof urlString !== 'string') return '';
    const trimmed = urlString.trim();
    if (!trimmed) return '';

    // Check for dangerous URI schemes or embedded control/newline characters
    const lower = trimmed.toLowerCase().replace(/[\s\r\n\t\-]/g, '');
    if (
      lower.startsWith('javascript:') ||
      lower.startsWith('data:') ||
      lower.startsWith('vbscript:') ||
      lower.startsWith('file:') ||
      lower.startsWith('blob:') ||
      lower.startsWith('about:') ||
      lower.startsWith('tel:') ||
      lower.startsWith('mailto:')
    ) {
      return '';
    }

    try {
      const parsed = new URL(trimmed);
      if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
        return parsed.toString();
      }
      return '';
    } catch (_) {
      // If relative URL starting with '/', allow unless protocol-relative '//'
      if (trimmed.startsWith('/') && !trimmed.startsWith('//')) {
        return trimmed;
      }
      return '';
    }
  }

  /**
   * Resolves an API URL given an endpoint, optional base URL, and native mode indicator.
   */
  function resolveApiUrl(endpoint, baseUrl = '', isNative = false) {
    if (!endpoint || typeof endpoint !== 'string') return '';
    const trimmed = endpoint.trim();
    if (!trimmed) return '';

    // If endpoint is already an absolute HTTP/HTTPS URL, return as-is
    if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
      return trimmed;
    }

    if (baseUrl && typeof baseUrl === 'string' && baseUrl.trim()) {
      const cleanBase = baseUrl.trim().replace(/\/+$/, '');
      const cleanPath = trimmed.startsWith('/') ? trimmed : `/${trimmed}`;
      return `${cleanBase}${cleanPath}`;
    }

    if (isNative) {
      // In native app environment (e.g. Capacitor) without explicit base URL,
      // fallback to production staging host
      const defaultHost = 'https://blist.fly.dev';
      const cleanPath = trimmed.startsWith('/') ? trimmed : `/${trimmed}`;
      return `${defaultHost}${cleanPath}`;
    }

    return trimmed;
  }

  /**
   * Parses incoming Web Share Target or query parameters into a normalized place object.
   * Handles Google Maps, Instagram, Apple Maps shares with titles, notes, and embedded URLs.
   */
  function parseShareTargetPayload(params) {
    if (!params) {
      return { url: '', title: '', text: '', rawText: '', isUrlCandidate: false };
    }

    let rawTitle = '';
    let rawText = '';
    let rawUrl = '';

    if (typeof params === 'string') {
      try {
        const parsed = new URLSearchParams(params.startsWith('?') ? params.slice(1) : params);
        rawTitle = parsed.get('title') || '';
        rawText = parsed.get('text') || '';
        rawUrl = parsed.get('url') || '';
      } catch (_) {
        rawText = params;
      }
    } else if (typeof params === 'object') {
      if (typeof params.get === 'function') {
        rawTitle = params.get('title') || '';
        rawText = params.get('text') || '';
        rawUrl = params.get('url') || '';
      } else {
        rawTitle = params.title || '';
        rawText = params.text || '';
        rawUrl = params.url || '';
      }
    }

    rawTitle = String(rawTitle).trim();
    rawText = String(rawText).trim();
    rawUrl = String(rawUrl).trim();

    let extractedUrl = '';
    if (isValidHttpUrl(rawUrl)) {
      extractedUrl = rawUrl;
    }

    // If no direct URL provided in url param, search in text param
    if (!extractedUrl && rawText) {
      const urlMatch = rawText.match(/https?:\/\/[^\s]+/i);
      if (urlMatch) {
        extractedUrl = urlMatch[0];
      }
    }

    // Also check title if it was passed purely as a URL
    if (!extractedUrl && rawTitle && isValidHttpUrl(rawTitle)) {
      extractedUrl = rawTitle;
      rawTitle = '';
    }

    // Extract title candidate if missing
    let extractedTitle = rawTitle;
    let remainingText = rawText;

    if (extractedUrl && remainingText.includes(extractedUrl)) {
      remainingText = remainingText.replace(extractedUrl, '').trim();
    }

    if (!extractedTitle && remainingText) {
      // Use first line of remaining text as title candidate
      const lines = remainingText.split(/[\r\n]+/);
      extractedTitle = lines[0].trim();
    }

    return {
      url: extractedUrl,
      title: extractedTitle,
      text: remainingText,
      rawText: rawText || rawTitle || rawUrl,
      isUrlCandidate: Boolean(extractedUrl)
    };
  }

  /**
   * Calculates Haversine distance in kilometers between two GPS coordinates.
   */
  function calculateDistance(lat1, lon1, lat2, lon2) {
    if (
      typeof lat1 !== 'number' || typeof lon1 !== 'number' ||
      typeof lat2 !== 'number' || typeof lon2 !== 'number' ||
      isNaN(lat1) || isNaN(lon1) || isNaN(lat2) || isNaN(lon2)
    ) {
      return 0;
    }

    const R = 6371; // Earth's radius in km
    const dLat = (lat2 - lat1) * Math.PI / 180;
    const dLon = (lon2 - lon1) * Math.PI / 180;
    const a =
      Math.sin(dLat / 2) * Math.sin(dLat / 2) +
      Math.cos(lat1 * Math.PI / 180) * Math.cos(lat2 * Math.PI / 180) *
      Math.sin(dLon / 2) * Math.sin(dLon / 2);
    const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
    return R * c;
  }

  /**
   * Formats distance in km to human-readable string (meters / miles & km).
   */
  function formatDistance(dKm) {
    if (typeof dKm !== 'number' || isNaN(dKm) || dKm < 0) return '0 m away';
    if (dKm < 1) {
      return `${Math.round(dKm * 1000)} m away`;
    } else {
      const mi = (dKm * 0.621371).toFixed(1);
      return `${mi} mi away (${dKm.toFixed(1)} km)`;
    }
  }

  /**
   * Validates if latitude and longitude are valid numbers within geographical limits.
   */
  function validateCoordinates(lat, lon) {
    if (
      lat === null || lon === null ||
      lat === undefined || lon === undefined ||
      typeof lat === 'boolean' || typeof lon === 'boolean' ||
      Array.isArray(lat) || Array.isArray(lon)
    ) {
      return false;
    }
    const latNum = Number(lat);
    const lonNum = Number(lon);
    if (!Number.isFinite(latNum) || !Number.isFinite(lonNum)) return false;
    if (latNum < -90 || latNum > 90) return false;
    if (lonNum < -180 || lonNum > 180) return false;
    return true;
  }

  /**
   * Generates a clean Google Maps multi-stop directions URL connecting pins in sequence.
   * Format: https://www.google.com/maps/dir/lat1,lon1/lat2,lon2/lat3,lon3
   * Automatically caps at maxStops (default 10) to respect Google Maps waypoint limits.
   */
  function generateGoogleMapsRouteUrl(pins, maxStops = 10) {
    if (!Array.isArray(pins) || pins.length < 2) return null;
    const validPins = pins.filter((p) => p && validateCoordinates(p.latitude, p.longitude)).slice(0, maxStops);
    if (validPins.length < 2) return null;

    const coordsPath = validPins.map((p) => `${p.latitude},${p.longitude}`).join('/');
    return `https://www.google.com/maps/dir/${coordsPath}`;
  }

  /**
   * Converts a list of Pin objects to a standard GeoJSON FeatureCollection.
   */
  function pinsToGeoJSON(pins) {
    if (!Array.isArray(pins)) {
      return { type: 'FeatureCollection', features: [] };
    }

    const features = pins
      .filter(pin => pin && validateCoordinates(pin.latitude, pin.longitude))
      .map(pin => ({
        type: 'Feature',
        geometry: {
          type: 'Point',
          // Standard GeoJSON coordinate order: [longitude, latitude]
          coordinates: [Number(pin.longitude), Number(pin.latitude)]
        },
        properties: {
          id: pin.id,
          list_id: pin.list_id || 1,
          title: pin.title || '',
          description: pin.description || null,
          category: pin.category || 'General',
          source_url: pin.source_url || null,
          image_url: pin.image_url || null,
          address: pin.address || null,
          notes: pin.notes || null,
          visited: Boolean(pin.visited),
          created_at: pin.created_at || ''
        }
      }));

    return {
      type: 'FeatureCollection',
      features: features
    };
  }

  /**
   * Parses a GeoJSON FeatureCollection or Feature object into a list of Pin objects.
   */
  function geoJSONToPins(geojson) {
    if (!geojson || typeof geojson !== 'object') return [];

    let features = [];
    if (geojson.type === 'FeatureCollection' && Array.isArray(geojson.features)) {
      features = geojson.features;
    } else if (geojson.type === 'Feature') {
      features = [geojson];
    }

    const pins = [];
    for (const f of features) {
      if (!f || !f.geometry || f.geometry.type !== 'Point' || !Array.isArray(f.geometry.coordinates)) {
        continue;
      }
      const coords = f.geometry.coordinates;
      const lon = coords[0];
      const lat = coords[1];
      if (!validateCoordinates(lat, lon)) continue;

      const props = f.properties || {};
      pins.push({
        id: props.id || undefined,
        list_id: props.list_id || 1,
        title: props.title || 'Untitled Place',
        description: props.description || null,
        latitude: Number(lat),
        longitude: Number(lon),
        category: props.category || 'General',
        source_url: props.source_url || null,
        image_url: props.image_url || null,
        address: props.address || null,
        notes: props.notes || null,
        visited: Boolean(props.visited),
        created_at: props.created_at || ''
      });
    }

    return pins;
  }

  /**
   * Filters a list of pins based on trip list, status, category, and search query.
   */
  function filterPins(pins, options = {}) {
    if (!Array.isArray(pins)) return [];

    const {
      listFilter = 'all',
      status = 'all',
      category = 'All',
      search = ''
    } = options;

    let result = pins;

    // 1. List filter
    if (listFilter === 'bucket') {
      result = result.filter(p => !p.visited);
    } else if (listFilter === 'visited') {
      result = result.filter(p => p.visited);
    } else if (listFilter !== 'all') {
      const listIdNum = parseInt(listFilter, 10);
      if (!isNaN(listIdNum)) {
        result = result.filter(p => p.list_id === listIdNum);
      }
    }

    // 2. Quick Status filter
    if (status === 'bucket') {
      result = result.filter(p => !p.visited);
    } else if (status === 'visited') {
      result = result.filter(p => p.visited);
    }

    // 3. Category filter
    if (category && category !== 'All') {
      result = result.filter(p => p.category === category);
    }

    // 4. Search query filter
    const query = (search || '').trim().toLowerCase();
    if (query) {
      result = result.filter(pin => {
        const matchTitle = pin.title && pin.title.toLowerCase().includes(query);
        const matchAddress = pin.address && pin.address.toLowerCase().includes(query);
        const matchNotes = pin.notes && pin.notes.toLowerCase().includes(query);
        const matchDesc = pin.description && pin.description.toLowerCase().includes(query);
        const matchCategory = pin.category && pin.category.toLowerCase().includes(query);
        return matchTitle || matchAddress || matchNotes || matchDesc || matchCategory;
      });
    }

    return result;
  }

  /**
   * Sorts a list of pins according to sort criteria.
   */
  function sortPins(pins, sortType = 'newest', userLocation = null) {
    if (!Array.isArray(pins)) return [];
    const copy = [...pins];

    return copy.sort((a, b) => {
      if (sortType === 'nearest' && userLocation && typeof userLocation.lat === 'number' && typeof userLocation.lng === 'number') {
        const distA = calculateDistance(userLocation.lat, userLocation.lng, a.latitude, a.longitude);
        const distB = calculateDistance(userLocation.lat, userLocation.lng, b.latitude, b.longitude);
        return distA - distB;
      } else if (sortType === 'az') {
        return (a.title || '').localeCompare(b.title || '');
      } else if (sortType === 'category') {
        return (a.category || '').localeCompare(b.category || '');
      } else {
        // default newest (by ID descending)
        return (b.id || 0) - (a.id || 0);
      }
    });
  }

  /**
   * Resolves effective theme given user preference setting and OS preference.
   */
  function getEffectiveTheme(themeSetting, prefersDark = false) {
    if (themeSetting === 'dark' || themeSetting === 'light') {
      return themeSetting;
    }
    return prefersDark ? 'dark' : 'light';
  }

  /**
   * Open Location Code (Plus Code) character alphabet.
   */
  const PLUS_CODE_ALPHABET = '23456789CFGHJMPQRVWX';

  /**
   * Encodes latitude and longitude into an Open Location Code (Plus Code).
   */
  function encodePlusCode(latitude, longitude) {
    if (typeof latitude !== 'number' || typeof longitude !== 'number' || isNaN(latitude) || isNaN(longitude)) {
      return '';
    }
    let lat = Math.min(Math.max(latitude, -90), 90);
    let lon = longitude;
    while (lon < -180) lon += 360;
    while (lon >= 180) lon -= 360;
    if (lat === 90) lat = 89.99999999;

    let latVal = lat + 90;
    let lonVal = lon + 180;
    const resolutions = [20.0, 1.0, 0.05, 0.0025, 0.000125];
    let code = '';

    for (let i = 0; i < 5; i++) {
      const res = resolutions[i];
      const latDigit = Math.min(Math.floor(latVal / res), 19);
      const lonDigit = Math.min(Math.floor(lonVal / res), 19);

      code += PLUS_CODE_ALPHABET[latDigit];
      code += PLUS_CODE_ALPHABET[lonDigit];

      latVal -= latDigit * res;
      lonVal -= lonDigit * res;

      if (code.length === 8) {
        code += '+';
      }
    }
    return code;
  }

  /**
   * Extracts a clean, concise locality (City, State/Region/Country) from an address string.
   */
  function extractLocality(address) {
    if (!address || typeof address !== 'string') return '';
    const clean = address
      .trim()
      .replace(/\b\d{5}(-\d{4})?\b/g, '')
      .replace(/\b[A-Z]\d[A-Z]\s?\d[A-Z]\d\b/g, '')
      .replace(/\b\d{4,6}\b/g, '')
      .trim();

    let parts = clean
      .split(',')
      .map(p => p.trim())
      .filter(Boolean);

    if (parts.length === 0) return '';
    if (parts.length === 1) {
      return parts[0].replace(/\s*\b(County|Parish|Borough|District)\b/gi, '').trim();
    }

    // Filter out standalone "USA", "United States", "US"
    if (parts.length > 2 && /^(USA|United States|United States of America|US)$/i.test(parts[parts.length - 1])) {
      parts = parts.slice(0, -1);
    }

    // Filter out intermediate "County", "Parish", "District" segments
    if (parts.length > 2) {
      parts = parts.filter((part, idx) => {
        if (/\b(County|Parish|Borough|District)\b/i.test(part) && idx < parts.length - 1) {
          return false;
        }
        return true;
      });
    }

    let locParts = parts.slice(-2);
    locParts = locParts.map(p => p.replace(/\s*\b(County|Parish)\b/gi, '').trim()).filter(Boolean);

    return locParts.join(', ').replace(/\s+,/g, ',').trim();
  }

  /**
   * Formats a Plus Code into Google Maps style (Short Local Code + Locality when address is available).
   * Example: "85GC68XX+RM", "Provo, UT" -> "68XX+RM Provo, UT"
   */
  function formatDisplayPlusCode(fullCode, address) {
    if (!fullCode || typeof fullCode !== 'string' || fullCode.length < 8 || !fullCode.includes('+')) {
      return fullCode || '';
    }
    const plusIdx = fullCode.indexOf('+');
    if (plusIdx < 8) {
      return fullCode; // already short code
    }
    const shortCode = fullCode.slice(plusIdx - 4);
    const locality = extractLocality(address);
    if (locality) {
      return `${shortCode} ${locality}`;
    }
    return fullCode;
  }

  /**
   * Application metadata, version, and repository/support links.
   */
  const APP_INFO = Object.freeze({
    name: 'bList',
    fullName: 'bList - Visual Map Bucket List & Trip Planner',
    version: '0.1.0',
    repositoryUrl: 'https://github.com/radmuffin/bList',
    issuesUrl: 'https://github.com/radmuffin/bList/issues',
    license: 'MIT'
  });

  /**
   * Returns a copy of the application metadata.
   */
  function getAppInfo() {
    return { ...APP_INFO };
  }

  return {
    APP_INFO,
    getAppInfo,
    escapeHtml,
    isValidHttpUrl,
    sanitizeUrl,
    resolveApiUrl,
    parseShareTargetPayload,
    calculateDistance,
    formatDistance,
    validateCoordinates,
    generateGoogleMapsRouteUrl,
    pinsToGeoJSON,
    geoJSONToPins,
    filterPins,
    sortPins,
    getEffectiveTheme,
    encodePlusCode,
    extractLocality,
    formatDisplayPlusCode
  };
});
