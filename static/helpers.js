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
      return {
        url: '',
        title: '',
        text: '',
        rawText: '',
        isJoinLink: false,
        joinToken: null,
        isSyncLink: false,
        syncToken: null,
        isUrlCandidate: false
      };
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

    const foundUrl = extractedUrl;
    // Check if the URL is a bare app root (e.g. https://blist.fly.dev/ or http://localhost:3000/ without search params)
    let isBareAppUrl = false;
    if (extractedUrl) {
      try {
        const u = new URL(extractedUrl);
        if (
          (u.hostname.includes('blist') || u.hostname === 'localhost' || u.hostname === '127.0.0.1') &&
          (u.pathname === '/' || u.pathname === '') &&
          !u.search &&
          !u.hash
        ) {
          isBareAppUrl = true;
        }
      } catch (_) {}
    }

    // If it's a bare app URL without place parameters, ignore it as the place URL candidate so remaining text is used
    if (isBareAppUrl) {
      extractedUrl = '';
    }

    // Extract title candidate if missing
    let extractedTitle = rawTitle;
    let remainingText = rawText;

    if (foundUrl && remainingText.includes(foundUrl)) {
      remainingText = remainingText.replace(foundUrl, '').trim();
    }

    let isJoinLink = false;
    let joinToken = null;
    let isSyncLink = false;
    let syncToken = null;

    const fullStringToCheck = `${rawUrl} ${rawText} ${rawTitle} ${foundUrl}`;
    const joinMatch = fullStringToCheck.match(/[?&]join=([a-zA-Z0-9_-]+)/);
    if (joinMatch) {
      isJoinLink = true;
      joinToken = joinMatch[1];
    }

    const syncMatch = fullStringToCheck.match(/[?&]sync_token=([a-zA-Z0-9_-]+)/);
    if (syncMatch) {
      isSyncLink = true;
      syncToken = syncMatch[1];
    }

    // Clean up "Check out ... on my travel bucket list!" or "Check out my ... trip" wrapper if shared from bList
    if (remainingText) {
      const blistShareMatch = remainingText.match(/^Check out(?:\s+my)?\s+(.+?)(?:\s+trip)?(?:\s+on my travel bucket list!?)?$/i);
      if (blistShareMatch) {
        remainingText = blistShareMatch[1].trim();
      }
    }

    if (!extractedTitle && remainingText) {
      // Use first line of remaining text as title candidate
      const lines = remainingText.split(/[\r\n]+/);
      extractedTitle = lines[0].trim();
    }

    // If extractedUrl has search params with title or coordinates (e.g. bList shared link), extract title if still empty
    if (!extractedTitle && extractedUrl) {
      try {
        const u = new URL(extractedUrl);
        const qTitle = u.searchParams.get('title') || u.searchParams.get('name') || u.searchParams.get('q');
        if (qTitle) extractedTitle = qTitle.trim();
      } catch (_) {}
    }

    // Clean up " | bList" or " - bList" suffix in title
    if (extractedTitle) {
      extractedTitle = extractedTitle.replace(/\s*\|\s*bList\s*$/i, '').replace(/\s*-\s*bList\s*$/i, '').trim();
    }

    return {
      url: extractedUrl,
      title: extractedTitle,
      text: remainingText,
      rawText: rawText || rawTitle || rawUrl,
      isJoinLink,
      joinToken,
      isSyncLink,
      syncToken,
      isUrlCandidate: Boolean(extractedUrl) && !isJoinLink && !isSyncLink
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
   * Filters a list of pins based on trip list, status, category, tag, priority, day group, and search query.
   */
  function filterPins(pins, options = {}) {
    if (!Array.isArray(pins)) return [];

    const {
      listFilter = 'all',
      status = 'all',
      category = 'All',
      tag = null,
      priorityOnly = false,
      dayGroup = null,
      openNowOnly = false,
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

    // 3. Priority filter
    if (priorityOnly) {
      result = result.filter(p => Boolean(p.priority));
    }

    // 4. Tag filter
    if (tag) {
      const targetTag = tag.trim().toLowerCase().replace(/^#/, '');
      result = result.filter(p => {
        if (!p.tags) return false;
        const pinTags = p.tags.toLowerCase().split(/[\s,]+/).map(t => t.replace(/^#/, ''));
        return pinTags.includes(targetTag);
      });
    }

    // 5. Day Planner Group
    if (dayGroup !== null && dayGroup !== undefined) {
      result = result.filter(p => p.day_group === dayGroup);
    }

    // 6. Open Now Filter
    if (openNowOnly) {
      result = result.filter(p => {
        if (!p.opening_hours) return false;
        const op = getOpeningStatus(p.opening_hours);
        return Boolean(op.isOpen);
      });
    }

    // 7. Category filter
    if (category && category !== 'All') {
      result = result.filter(p => p.category === category);
    }

    // 8. Search query filter
    const query = (search || '').trim().toLowerCase();
    if (query) {
      result = result.filter(pin => {
        const matchTitle = pin.title && pin.title.toLowerCase().includes(query);
        const matchAddress = pin.address && pin.address.toLowerCase().includes(query);
        const matchNotes = pin.notes && pin.notes.toLowerCase().includes(query);
        const matchDesc = pin.description && pin.description.toLowerCase().includes(query);
        const matchCategory = pin.category && pin.category.toLowerCase().includes(query);
        const matchTags = pin.tags && pin.tags.toLowerCase().includes(query);
        return matchTitle || matchAddress || matchNotes || matchDesc || matchCategory || matchTags;
      });
    }

    return result;
  }

  /**
   * Solves TSP sequence optimization using Nearest Neighbor + 2-Opt heuristic.
   */
  function optimizeTour2Opt(pins) {
    if (!Array.isArray(pins) || pins.length <= 2) return Array.isArray(pins) ? [...pins] : [];

    const validPins = pins.filter((p) => p && validateCoordinates(p.latitude, p.longitude));
    if (validPins.length <= 2) return [...pins];

    const unvisited = [...validPins];
    const tour = [unvisited.shift()];

    while (unvisited.length > 0) {
      const current = tour[tour.length - 1];
      let nearestIdx = 0;
      let minDistance = Infinity;

      for (let i = 0; i < unvisited.length; i++) {
        const d = calculateDistance(
          current.latitude,
          current.longitude,
          unvisited[i].latitude,
          unvisited[i].longitude
        );
        if (d < minDistance) {
          minDistance = d;
          nearestIdx = i;
        }
      }
      tour.push(unvisited.splice(nearestIdx, 1)[0]);
    }

    // 2-Opt heuristic passes
    let improved = true;
    let iterations = 0;
    while (improved && iterations < 50) {
      improved = false;
      iterations++;
      for (let i = 0; i < tour.length - 1; i++) {
        for (let k = i + 1; k < tour.length; k++) {
          const d1 = calculateDistance(
            tour[i].latitude,
            tour[i].longitude,
            tour[i + 1] ? tour[i + 1].latitude : tour[i].latitude,
            tour[i + 1] ? tour[i + 1].longitude : tour[i].longitude
          );
          const d2 = k + 1 < tour.length ? calculateDistance(
            tour[k].latitude,
            tour[k].longitude,
            tour[k + 1].latitude,
            tour[k + 1].longitude
          ) : 0;

          const d3 = calculateDistance(
            tour[i].latitude,
            tour[i].longitude,
            tour[k].latitude,
            tour[k].longitude
          );
          const d4 = k + 1 < tour.length ? calculateDistance(
            tour[i + 1].latitude,
            tour[i + 1].longitude,
            tour[k + 1].latitude,
            tour[k + 1].longitude
          ) : 0;

          if (d3 + d4 < d1 + d2 - 0.0001) {
            const reversed = tour.slice(i + 1, k + 1).reverse();
            tour.splice(i + 1, reversed.length, ...reversed);
            improved = true;
          }
        }
      }
    }

    return tour;
  }

  function formatFileSize(bytes) {
    if (!bytes || bytes < 1024) return (bytes || 0) + ' B';
    if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / 1048576).toFixed(1) + ' MB';
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
   * Formats a full address into a concise street-level section.
   * Strips redundant place title from the start and city/state/zip/country from the end
   * to provide a compact, clean address row that pairs with Plus Code & locality.
   */
  function formatStreetAddress(address, title = '') {
    if (!address || typeof address !== 'string') return '';
    let raw = address.trim();
    if (!raw) return '';

    // Split into comma-separated segments
    let segments = raw.split(',').map(s => s.trim()).filter(Boolean);
    if (segments.length === 0) return '';

    // 1. If first segment is the place title (or matches title prefix/suffix), strip it
    if (title && typeof title === 'string' && segments.length > 1) {
      const cleanTitle = title.trim().toLowerCase();
      const firstSeg = segments[0].toLowerCase();
      if (firstSeg === cleanTitle || cleanTitle.startsWith(firstSeg) || firstSeg.startsWith(cleanTitle)) {
        segments.shift();
      }
    }

    if (segments.length === 0) return '';
    if (segments.length === 1) {
      // If single segment is identical to title, return empty to avoid duplication
      if (title && segments[0].toLowerCase() === title.trim().toLowerCase()) return '';
      return segments[0];
    }

    // 2. Filter trailing country names
    if (
      segments.length > 1 &&
      /^(USA|United States|United States of America|US|United Kingdom|UK|Canada|Australia|Germany|France|Japan|Italy|Spain|Mexico)$/i.test(segments[segments.length - 1])
    ) {
      segments.pop();
    }

    // 3. Filter trailing postal codes (e.g. 84604, 84604-1234, SW1A 1AA)
    if (segments.length > 1) {
      const last = segments[segments.length - 1];
      if (/^\b(\d{4,6}(-\d{4})?|[A-Z]\d[A-Z]\s?\d[A-Z]\d)\b$/i.test(last.trim())) {
        segments.pop();
      }
    }

    // 4. Strip trailing state + zip (e.g. "UT 84604" -> "UT")
    if (segments.length > 1) {
      segments[segments.length - 1] = segments[segments.length - 1]
        .replace(/\b\d{5}(-\d{4})?\b/g, '')
        .replace(/\b[A-Z]\d[A-Z]\s?\d[A-Z]\d\b/g, '')
        .replace(/\b\d{4,6}\b/g, '')
        .trim();

      if (!segments[segments.length - 1]) {
        segments.pop();
      }
    }

    // 5. Remove county/district segments
    segments = segments.filter(seg => !/\b(County|Parish|Borough|District)\b/i.test(seg));

    if (segments.length === 0) return '';

    // 6. Handle street segments
    // If the first segment is just a house/building number (e.g., "2060"), combine with road (segment 1)
    if (/^\d+[a-zA-Z]?$/.test(segments[0]) && segments.length >= 2) {
      return `${segments[0]} ${segments[1]}`.trim();
    }

    // If we have multiple segments left, the first is the street
    if (segments.length >= 2) {
      return segments[0];
    }

    return segments.join(', ').trim();
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
   * Parses a time string (e.g. "08:00", "8:00 AM", "8pm", "20:30", "24:00") into minutes from midnight (0..1440).
   */
  function parseTimeToMinutes(str) {
    if (!str || typeof str !== 'string') return null;
    const s = str.trim().toLowerCase();
    if (s === '24:00' || s === '24.00' || s === '24h' || s === '24:00:00') {
      return 1440;
    }
    const match = s.match(/^(\d{1,2})(?:[:.](\d{2}))?\s*(am|pm)?$/);
    if (!match) return null;

    let hours = parseInt(match[1], 10);
    const minutes = match[2] ? parseInt(match[2], 10) : 0;
    const meridiem = match[3];

    if (hours === 24 && minutes === 0) return 1440;

    if (meridiem === 'pm' && hours < 12) hours += 12;
    if (meridiem === 'am' && hours === 12) hours = 0;

    if (hours < 0 || hours > 24 || minutes < 0 || minutes > 59) return null;
    if (hours === 24 && minutes > 0) return null;
    return hours * 60 + minutes;
  }

  /**
   * Formats minutes from midnight into 12-hour AM/PM time (e.g. 600 -> "10:00 AM", 1260 -> "9:00 PM", 1440 -> "12:00 AM").
   */
  function formatMinutesToTime(mins) {
    let m = mins % (24 * 60);
    if (m < 0) m += 24 * 60;
    const h24 = Math.floor(m / 60);
    const min = m % 60;
    const ampm = h24 >= 12 ? 'PM' : 'AM';
    const h12 = h24 % 12 === 0 ? 12 : h24 % 12;
    const minStr = min < 10 ? `0${min}` : `${min}`;
    return min === 0 ? `${h12}:00 ${ampm}` : `${h12}:${minStr} ${ampm}`;
  }

  const DAY_ALIASES = {
    su: 0, sun: 0, sunday: 0,
    mo: 1, mon: 1, monday: 1,
    tu: 2, tue: 2, tues: 2, tuesday: 2,
    we: 3, wed: 3, weds: 3, wednesday: 3,
    th: 4, thu: 4, thur: 4, thurs: 4, thursday: 4,
    fr: 5, fri: 5, friday: 5,
    sa: 6, sat: 6, saturday: 6
  };

  /**
   * Deterministically evaluates whether a location is Open Now, Closing Soon, or Closed based on opening_hours string.
   */
  function getOpeningStatus(openingHoursStr, currentDateTime = new Date()) {
    if (!openingHoursStr || typeof openingHoursStr !== 'string' || !openingHoursStr.trim()) {
      return {
        status: 'unknown',
        isOpen: null,
        label: '',
        details: '',
        badgeClass: ''
      };
    }

    const clean = openingHoursStr.trim();
    const lower = clean.toLowerCase();

    if (lower === '24/7' || lower === 'open 24 hours' || lower === '24 hours' || lower === 'open 24/7') {
      return {
        status: 'open',
        isOpen: true,
        label: 'Open 24/7',
        details: 'Open 24 hours daily',
        badgeClass: 'badge-open'
      };
    }

    if (lower === 'closed' || lower === 'temporarily closed' || lower === 'permanently closed') {
      return {
        status: 'closed',
        isOpen: false,
        label: 'Closed',
        details: clean,
        badgeClass: 'badge-closed'
      };
    }

    const currentDay = currentDateTime.getDay(); // 0: Sun, 1: Mon, ...
    const prevDay = (currentDay + 6) % 7;
    const currentMins = currentDateTime.getHours() * 60 + currentDateTime.getMinutes();

    function matchesDay(daySpec, targetDay) {
      if (daySpec === 'daily' || daySpec === 'everyday') return true;
      const parts = daySpec.split(',').map(d => d.trim());
      for (const part of parts) {
        const rangeMatch = part.match(/^([a-z]{2,9})\s*-\s*([a-z]{2,9})$/);
        if (rangeMatch) {
          const startDay = DAY_ALIASES[rangeMatch[1]];
          const endDay = DAY_ALIASES[rangeMatch[2]];
          if (startDay !== undefined && endDay !== undefined) {
            if (startDay <= endDay) {
              if (targetDay >= startDay && targetDay <= endDay) return true;
            } else {
              if (targetDay >= startDay || targetDay <= endDay) return true;
            }
          }
        } else {
          const singleDay = DAY_ALIASES[part];
          if (singleDay !== undefined && singleDay === targetDay) return true;
        }
      }
      return false;
    }

    const segments = clean.split(/;|\n|\|/).map(s => s.trim()).filter(Boolean);
    let todayIntervals = [];
    let prevDayOvernightIntervals = [];
    let fallbackIntervals = [];
    let isExplicitlyClosedToday = false;

    for (const seg of segments) {
      const segLower = seg.toLowerCase();
      const dayMatch = segLower.match(/^([a-z]{2,9}(?:\s*-\s*[a-z]{2,9})?(?:,\s*[a-z]{2,9})*|daily|everyday)\s*[:]?\s*(.*)$/);

      let timePart = seg;
      let hasDaySpec = false;
      let appliesToday = true;
      let appliesPrevDay = false;

      if (dayMatch) {
        hasDaySpec = true;
        const daySpec = dayMatch[1].trim();
        timePart = dayMatch[2].trim();
        appliesToday = matchesDay(daySpec, currentDay);
        appliesPrevDay = matchesDay(daySpec, prevDay);
      }

      if (timePart.toLowerCase().includes('off') || timePart.toLowerCase().includes('closed')) {
        if (appliesToday && hasDaySpec) {
          isExplicitlyClosedToday = true;
        }
        continue;
      }

      const timeRanges = timePart.split(/,|&/).map(t => t.trim()).filter(Boolean);
      for (const tr of timeRanges) {
        const rangeParts = tr.split(/\s*(?:-|–|—|to)\s*/i);
        if (rangeParts.length === 2) {
          const start = parseTimeToMinutes(rangeParts[0]);
          const end = parseTimeToMinutes(rangeParts[1]);
          if (start !== null && end !== null) {
            const isOvernight = end <= start;
            const interval = { start, end, isOvernight };
            if (hasDaySpec) {
              if (appliesToday) {
                todayIntervals.push(interval);
              }
              if (appliesPrevDay && isOvernight) {
                prevDayOvernightIntervals.push(interval);
              }
            } else {
              fallbackIntervals.push(interval);
            }
          }
        }
      }
    }

    // 1. Check if we are currently inside an overnight shift from yesterday (e.g. Friday 20:00 - 02:00 at Saturday 01:00)
    for (const item of prevDayOvernightIntervals) {
      if (currentMins < item.end) {
        const minsUntilClose = item.end - currentMins;
        if (minsUntilClose <= 45 && minsUntilClose > 0) {
          return {
            status: 'closing_soon',
            isOpen: true,
            label: `Closing Soon (${minsUntilClose}m)`,
            details: `Closes at ${formatMinutesToTime(item.end)}`,
            badgeClass: 'badge-closing-soon'
          };
        }
        return {
          status: 'open',
          isOpen: true,
          label: `Open until ${formatMinutesToTime(item.end)}`,
          details: `Open now • Closes at ${formatMinutesToTime(item.end)}`,
          badgeClass: 'badge-open'
        };
      }
    }

    // 2. Check today's intervals or fallback intervals
    const intervalsToEval = todayIntervals.length > 0 ? todayIntervals : fallbackIntervals;

    if (isExplicitlyClosedToday && todayIntervals.length === 0) {
      return {
        status: 'closed',
        isOpen: false,
        label: 'Closed Today',
        details: clean,
        badgeClass: 'badge-closed'
      };
    }

    if (intervalsToEval.length === 0) {
      return {
        status: 'unknown',
        isOpen: null,
        label: clean,
        details: clean,
        badgeClass: ''
      };
    }

    for (const item of intervalsToEval) {
      let isOpenNow = false;
      let minsUntilClose = 0;

      if (!item.isOvernight) {
        if (currentMins >= item.start && currentMins < item.end) {
          isOpenNow = true;
          minsUntilClose = item.end - currentMins;
        }
      } else {
        if (currentMins >= item.start) {
          isOpenNow = true;
          minsUntilClose = (24 * 60 - currentMins) + item.end;
        } else if (todayIntervals.length === 0 && currentMins < item.end) {
          // General daily fallback overnight early morning
          isOpenNow = true;
          minsUntilClose = item.end - currentMins;
        }
      }

      if (isOpenNow) {
        if (minsUntilClose <= 45 && minsUntilClose > 0) {
          return {
            status: 'closing_soon',
            isOpen: true,
            label: `Closing Soon (${minsUntilClose}m)`,
            details: `Closes at ${formatMinutesToTime(item.end)}`,
            badgeClass: 'badge-closing-soon'
          };
        }
        return {
          status: 'open',
          isOpen: true,
          label: `Open until ${formatMinutesToTime(item.end)}`,
          details: `Open now • Closes at ${formatMinutesToTime(item.end)}`,
          badgeClass: 'badge-open'
        };
      }
    }

    // Next upcoming interval today
    const upcoming = intervalsToEval
      .filter(i => i.start > currentMins)
      .sort((a, b) => a.start - b.start)[0];

    if (upcoming) {
      return {
        status: 'closed',
        isOpen: false,
        label: `Closed • Opens ${formatMinutesToTime(upcoming.start)}`,
        details: `Closed now • Opens at ${formatMinutesToTime(upcoming.start)}`,
        badgeClass: 'badge-closed'
      };
    }

    return {
      status: 'closed',
      isOpen: false,
      label: 'Closed',
      details: clean,
      badgeClass: 'badge-closed'
    };
  }

  /**
   * Curated collection of whimsical, iconic bucket list destinations for the inspiration easter egg.
   */
  const INSPIRATIONS = [
    {
      title: "Jiufen Old Street",
      emoji: "🏮",
      category: "Food & Drink",
      latitude: 25.1099,
      longitude: 121.8452,
      address: "Ruifang District, New Taipei City, Taiwan",
      description: "Atmospheric lantern-lit mountain village with legendary taro balls and tea houses.",
      notes: "Inspired Spirited Away vibes! Try the herbal rice cakes and sip Oolong at Amei Tea House."
    },
    {
      title: "Chefchaouen Blue City",
      emoji: "💙",
      category: "Sightseeing",
      latitude: 35.1688,
      longitude: -5.2636,
      address: "Chefchaouen, Morocco",
      description: "Enchanting medina washed in vibrant shades of cobalt and powder blue nestled in the Rif Mountains.",
      notes: "Get lost in the blue alleys early in the morning before crowds arrive."
    },
    {
      title: "Fushimi Inari Taisha",
      emoji: "⛩️",
      category: "Culture",
      latitude: 34.9671,
      longitude: 135.7727,
      address: "Fushimi Ward, Kyoto, Japan",
      description: "Thousands of vermilion torii gates winding through sacred forest paths up Mount Inari.",
      notes: "Hike past the summit during twilight for breathtaking Kyoto panorama."
    },
    {
      title: "Blue Lagoon Geothermal Spa",
      emoji: "♨️",
      category: "Nature & Outdoors",
      latitude: 63.8804,
      longitude: -22.4495,
      address: "Grindavík, Iceland",
      description: "Mineral-rich milky blue geothermal seawater surrounded by black lava fields.",
      notes: "Pre-book the silica mud mask experience. Unforgettable during snow season."
    },
    {
      title: "Cinque Terre Coastal Trail",
      emoji: "🌊",
      category: "Nature & Outdoors",
      latitude: 44.1461,
      longitude: 9.6439,
      address: "Liguria, Italy",
      description: "Five cliffside pastel fishing villages connected by coastal hiking paths overlooking the Mediterranean.",
      notes: "Grab freshly fried calamari in a paper cone in Riomaggiore at sunset."
    },
    {
      title: "Banff Moraine Lake",
      emoji: "🏔️",
      category: "Nature & Outdoors",
      latitude: 51.3217,
      longitude: -116.1860,
      address: "Banff National Park, Alberta, Canada",
      description: "Glacially fed azure lake reflecting the dramatic Valley of the Ten Peaks.",
      notes: "Canoe on the turquoise water at sunrise for mirrored mountain reflections."
    },
    {
      title: "Sidi Bou Said",
      emoji: "☕",
      category: "Cafe",
      latitude: 36.8703,
      longitude: 10.3417,
      address: "Carthage, Tunis, Tunisia",
      description: "Cliffside village overlooking the Gulf of Tunis with whitewashed walls and electric blue doors.",
      notes: "Sip traditional pine-nut mint tea at Café des Délices while watching Mediterranean boats."
    },
    {
      title: "Horseshoe Bend",
      emoji: "🏜️",
      category: "Sightseeing",
      latitude: 36.8790,
      longitude: -111.5105,
      address: "Page, Arizona, USA",
      description: "Dramatic 1,000-foot sheer canyon drop carved into a horseshoe curve by the Colorado River.",
      notes: "Best photographed with a wide-angle lens 1 hour before sunset."
    },
    {
      title: "Bagan Ancient Pagodas",
      emoji: "🎈",
      category: "Culture",
      latitude: 21.1717,
      longitude: 94.8585,
      address: "Mandalay Region, Myanmar",
      description: "Archaeological landscape dotted with over 2,000 ancient Buddhist temples and stupas.",
      notes: "Hot air balloon flight over the misty temple plains at sunrise."
    },
    {
      title: "Giant's Causeway",
      emoji: "🗿",
      category: "Nature & Outdoors",
      latitude: 55.2408,
      longitude: -6.5116,
      address: "Bushmills, County Antrim, Northern Ireland",
      description: "40,000 interlocking basalt hexagonal columns created by ancient volcanic fissures.",
      notes: "Walk the Shepherd's Steps cliff path for panoramic Atlantic views."
    }
  ];

  // ---------------------------------------------------------------------------
  // Self-Contained Offline SVG QR Code Generator (ISO/IEC 18004 Standard)
  // ---------------------------------------------------------------------------

  const GF256_EXP = new Uint8Array(512);
  const GF256_LOG = new Uint8Array(256);
  (function initGf256() {
    let x = 1;
    for (let i = 0; i < 255; i++) {
      GF256_EXP[i] = x;
      GF256_EXP[i + 255] = x;
      GF256_LOG[x] = i;
      x <<= 1;
      if (x >= 256) x ^= 0x11d;
    }
  })();

  function gfMul(x, y) {
    if (x === 0 || y === 0) return 0;
    return GF256_EXP[GF256_LOG[x] + GF256_LOG[y]];
  }

  function rsPolyMul(p, q) {
    const r = new Array(p.length + q.length - 1).fill(0);
    for (let i = 0; i < p.length; i++) {
      for (let j = 0; j < q.length; j++) {
        r[i + j] ^= gfMul(p[i], q[j]);
      }
    }
    return r;
  }

  function rsGenPoly(ecLen) {
    let g = [1];
    for (let i = 0; i < ecLen; i++) {
      g = rsPolyMul(g, [1, GF256_EXP[i]]);
    }
    return g;
  }

  function rsCalcEcc(data, ecLen) {
    const gen = rsGenPoly(ecLen);
    const result = new Array(ecLen).fill(0);
    for (let i = 0; i < data.length; i++) {
      const factor = data[i] ^ result[0];
      result.shift();
      result.push(0);
      if (factor !== 0) {
        for (let j = 0; j < ecLen; j++) {
          result[j] ^= gfMul(gen[j + 1], factor);
        }
      }
    }
    return result;
  }

  // QR Version Specs for Level 'M' (15% Error Correction) and 'L' (7% Error Correction)
  // Format: [version, totalCodewords, dataCodewordsM, ecPerBlockM, numBlocksM, alignPatternCoords]
  const QR_SPECS_M = [
    null,
    [1, 26, 16, 10, 1, []],
    [2, 44, 28, 16, 1, [6, 18]],
    [3, 70, 44, 26, 1, [6, 22]],
    [4, 100, 64, 18, 2, [6, 26]],
    [5, 134, 86, 24, 2, [6, 30]],
    [6, 172, 108, 16, 4, [6, 34]],
    [7, 196, 124, 18, 4, [6, 22, 38]],
    [8, 242, 154, 22, 4, [6, 24, 42]],
    [9, 292, 182, 22, 5, [6, 26, 46]],
    [10, 346, 216, 26, 5, [6, 28, 50]]
  ];

  // BCH Format Info (15 bits) masked with 0x5412 for Level M and Mask Patterns 0-7
  const FORMAT_INFO_M = [
    0x5412 ^ 0x5412, // mask 0: 0x0000
    0x5125 ^ 0x5412, // mask 1
    0x5e7c ^ 0x5412, // mask 2
    0x5b4b ^ 0x5412, // mask 3
    0x45f9 ^ 0x5412, // mask 4
    0x40ce ^ 0x5412, // mask 5
    0x4f97 ^ 0x5412, // mask 6
    0x4aa0 ^ 0x5412  // mask 7
  ];

  /**
   * Generates a fully self-contained SVG QR Code and data URI for any given text.
   * Works 100% offline with zero dependencies or external network calls.
   */
  function generateQrSvg(text, options = {}) {
    const rawText = String(text || '').trim();
    if (!rawText) {
      return { svg: '', dataUrl: '', size: 0, moduleCount: 0 };
    }

    const margin = typeof options.margin === 'number' ? options.margin : 2;
    const size = typeof options.size === 'number' ? options.size : 240;
    const fg = options.foreground || '#000000';
    const bg = options.background || '#ffffff';

    // Encode text into UTF-8 bytes
    let utf8Bytes = [];
    if (typeof TextEncoder !== 'undefined') {
      utf8Bytes = Array.from(new TextEncoder().encode(rawText));
    } else {
      for (let i = 0; i < rawText.length; i++) {
        let code = rawText.charCodeAt(i);
        if (code < 128) {
          utf8Bytes.push(code);
        } else if (code < 2048) {
          utf8Bytes.push(192 | (code >> 6), 128 | (code & 63));
        } else {
          utf8Bytes.push(224 | (code >> 12), 128 | ((code >> 6) & 63), 128 | (code & 63));
        }
      }
    }

    // Determine smallest suitable QR version
    let chosenVersion = 1;
    for (let v = 1; v <= 10; v++) {
      const spec = QR_SPECS_M[v];
      const maxDataBytes = spec[2] - (v <= 9 ? 2 : 3); // Byte mode header overhead (4 bits mode + 8/16 bits len)
      if (utf8Bytes.length <= maxDataBytes) {
        chosenVersion = v;
        break;
      }
      if (v === 10) {
        chosenVersion = 10; // Cap at 10
      }
    }

    const spec = QR_SPECS_M[chosenVersion];
    const totalCodewords = spec[1];
    const dataCodewords = spec[2];
    const ecPerBlock = spec[3];
    const numBlocks = spec[4];
    const alignCoords = spec[5];
    const moduleCount = 17 + 4 * chosenVersion;

    // Bit buffer encoding (Byte Mode: 0100)
    let bitBuffer = [];
    function appendBits(val, len) {
      for (let i = len - 1; i >= 0; i--) {
        bitBuffer.push((val >> i) & 1);
      }
    }

    appendBits(0x4, 4); // Byte mode indicator
    appendBits(utf8Bytes.length, chosenVersion <= 9 ? 8 : 16); // Character count indicator
    for (let b of utf8Bytes) {
      appendBits(b, 8);
    }

    // Add terminator (up to 4 zeroes)
    const totalDataBits = dataCodewords * 8;
    const termBits = Math.min(4, totalDataBits - bitBuffer.length);
    appendBits(0, termBits);

    // Pad to byte boundary
    while (bitBuffer.length % 8 !== 0) {
      bitBuffer.push(0);
    }

    // Pad bytes to fill data capacity
    const padBytes = [0xEC, 0x11];
    let padIdx = 0;
    while (bitBuffer.length < totalDataBits) {
      appendBits(padBytes[padIdx % 2], 8);
      padIdx++;
    }

    // Convert bit buffer to data codewords
    const rawDataBytes = [];
    for (let i = 0; i < bitBuffer.length; i += 8) {
      let byteVal = 0;
      for (let j = 0; j < 8; j++) {
        byteVal = (byteVal << 1) | bitBuffer[i + j];
      }
      rawDataBytes.push(byteVal);
    }

    // Split data into blocks and calculate Reed-Solomon ECC
    const blocksData = [];
    const blocksEcc = [];
    const baseBlockLen = Math.floor(dataCodewords / numBlocks);
    const extraBlocks = dataCodewords % numBlocks;

    let dataOffset = 0;
    for (let b = 0; b < numBlocks; b++) {
      const curBlockLen = baseBlockLen + (b >= numBlocks - extraBlocks ? 1 : 0);
      const curData = rawDataBytes.slice(dataOffset, dataOffset + curBlockLen);
      dataOffset += curBlockLen;
      blocksData.push(curData);
      blocksEcc.push(rsCalcEcc(curData, ecPerBlock));
    }

    // Interleave data and ECC codewords
    const finalCodewords = [];
    const maxDataBlockLen = baseBlockLen + (extraBlocks > 0 ? 1 : 0);
    for (let i = 0; i < maxDataBlockLen; i++) {
      for (let b = 0; b < numBlocks; b++) {
        if (i < blocksData[b].length) {
          finalCodewords.push(blocksData[b][i]);
        }
      }
    }
    for (let i = 0; i < ecPerBlock; i++) {
      for (let b = 0; b < numBlocks; b++) {
        finalCodewords.push(blocksEcc[b][i]);
      }
    }

    // Initialize module matrix and isFunction (reserved) mask
    const matrix = Array.from({ length: moduleCount }, () => new Array(moduleCount).fill(0));
    const isFunc = Array.from({ length: moduleCount }, () => new Array(moduleCount).fill(false));

    function setModule(r, c, val, func = true) {
      if (r >= 0 && r < moduleCount && c >= 0 && c < moduleCount) {
        matrix[r][c] = val ? 1 : 0;
        if (func) isFunc[r][c] = true;
      }
    }

    // Place Finder Patterns (7x7) + Separators
    function placeFinder(r0, c0) {
      for (let r = -1; r <= 7; r++) {
        for (let c = -1; c <= 7; c++) {
          const rCur = r0 + r;
          const cCur = c0 + c;
          if (rCur < 0 || rCur >= moduleCount || cCur < 0 || cCur >= moduleCount) continue;
          if (r >= 0 && r <= 6 && c >= 0 && c <= 6) {
            const isDark = r === 0 || r === 6 || c === 0 || c === 6 || (r >= 2 && r <= 4 && c >= 2 && c <= 4);
            setModule(rCur, cCur, isDark, true);
          } else {
            setModule(rCur, cCur, false, true);
          }
        }
      }
    }

    placeFinder(0, 0);
    placeFinder(0, moduleCount - 7);
    placeFinder(moduleCount - 7, 0);

    // Place Alignment Patterns for Version >= 2
    if (alignCoords && alignCoords.length > 0) {
      for (let r of alignCoords) {
        for (let c of alignCoords) {
          if (isFunc[r][c]) continue; // Skip if overlapping finder patterns
          for (let dr = -2; dr <= 2; dr++) {
            for (let dc = -2; dc <= 2; dc++) {
              const isDark = Math.abs(dr) === 2 || Math.abs(dc) === 2 || (dr === 0 && dc === 0);
              setModule(r + dr, c + dc, isDark, true);
            }
          }
        }
      }
    }

    // Place Timing Patterns (row 6 and col 6)
    for (let i = 8; i < moduleCount - 8; i++) {
      if (!isFunc[6][i]) setModule(6, i, i % 2 === 0, true);
      if (!isFunc[i][6]) setModule(i, 6, i % 2 === 0, true);
    }

    // Dark Module
    setModule(moduleCount - 8, 8, true, true);

    // Reserve Format Information Areas
    for (let i = 0; i < 9; i++) {
      if (!isFunc[8][i]) isFunc[8][i] = true;
      if (!isFunc[i][8]) isFunc[i][8] = true;
    }
    for (let i = 0; i < 8; i++) {
      if (!isFunc[8][moduleCount - 1 - i]) isFunc[8][moduleCount - 1 - i] = true;
      if (!isFunc[moduleCount - 1 - i][8]) isFunc[moduleCount - 1 - i][8] = true;
    }

    // Convert Final Codewords to Bits and Place in Matrix
    const dataBits = [];
    for (let byte of finalCodewords) {
      for (let i = 7; i >= 0; i--) {
        dataBits.push((byte >> i) & 1);
      }
    }

    let bitIdx = 0;
    let right = moduleCount - 1;
    let upward = true;

    while (right > 0) {
      if (right === 6) right--; // Skip vertical timing column
      const colPairs = [right, right - 1];
      const rows = upward
        ? Array.from({ length: moduleCount }, (_, idx) => moduleCount - 1 - idx)
        : Array.from({ length: moduleCount }, (_, idx) => idx);

      for (let r of rows) {
        for (let c of colPairs) {
          if (!isFunc[r][c]) {
            const bit = bitIdx < dataBits.length ? dataBits[bitIdx++] : 0;
            // Apply Standard Mask Pattern 0: (row + col) % 2 === 0
            const maskBit = (r + c) % 2 === 0 ? 1 : 0;
            matrix[r][c] = bit ^ maskBit;
          }
        }
      }
      upward = !upward;
      right -= 2;
    }

    // Write Format Information for Mask Pattern 0
    const formatBitsVal = FORMAT_INFO_M[0];
    for (let i = 0; i < 15; i++) {
      const bit = (formatBitsVal >> i) & 1;
      // Around top-left finder
      if (i <= 5) setModule(8, i, bit, true);
      else if (i === 6) setModule(8, 7, bit, true);
      else if (i === 7) setModule(8, 8, bit, true);
      else if (i === 8) setModule(7, 8, bit, true);
      else setModule(14 - i, 8, bit, true);

      // Around other two finders
      if (i < 8) setModule(moduleCount - 1 - i, 8, bit, true);
      else setModule(8, moduleCount - 15 + i, bit, true);
    }

    // Build Scalable SVG Path
    let pathD = '';
    for (let r = 0; r < moduleCount; r++) {
      for (let c = 0; c < moduleCount; c++) {
        if (matrix[r][c] === 1) {
          const x = c + margin;
          const y = r + margin;
          pathD += `M${x},${y}h1v1h-1z `;
        }
      }
    }

    const totalDim = moduleCount + margin * 2;
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${totalDim} ${totalDim}" width="${size}" height="${size}" shape-rendering="crispEdges"><rect width="${totalDim}" height="${totalDim}" fill="${bg}"/><path d="${pathD.trim()}" fill="${fg}"/></svg>`;
    const dataUrl = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;

    return {
      svg,
      dataUrl,
      size,
      moduleCount: totalDim
    };
  }

  /**
   * Generates formatted share URLs for various social and messaging platforms.
   */
  function generateShareLinks(url, options = {}) {
    const safeUrl = sanitizeUrl(url) || 'https://blist-radmuffin.fly.dev/';
    const title = options.title || 'bList - Visual Map Bucket List & Trip Planner';
    const text = options.text || `Check out bList — a fast, private travel bucket list & map trip planner! 🗺️✨\n${safeUrl}`;
    const encodedUrl = encodeURIComponent(safeUrl);
    const encodedText = encodeURIComponent(text);
    const encodedTitle = encodeURIComponent(title);

    const qrResult = generateQrSvg(safeUrl, { size: 240, margin: 2 });

    return {
      url: safeUrl,
      text,
      sms: `sms:?&body=${encodeURIComponent(`Check out bList for travel bucket lists & map planning: ${safeUrl}`)}`,
      whatsapp: `https://api.whatsapp.com/send?text=${encodeURIComponent(`Check out bList: ${safeUrl}`)}`,
      messenger: `fb-messenger://share/?link=${encodedUrl}`,
      twitter: `https://twitter.com/intent/tweet?text=${encodedText}`,
      email: `mailto:?subject=${encodedTitle}&body=${encodeURIComponent(`Hey!\n\nI thought you'd love bList for saving places and organizing travel bucket lists on a visual map:\n\n${safeUrl}\n\nHappy travels! 🗺️✈️`)}`,
      telegram: `https://t.me/share/url?url=${encodedUrl}&text=${encodeURIComponent(title)}`,
      qrUrl: qrResult.dataUrl || `https://api.qrserver.com/v1/create-qr-code/?size=240x240&data=${encodedUrl}&margin=10`,
      qrDataUrl: qrResult.dataUrl,
      qrSvg: qrResult.svg
    };
  }

  /**
   * Returns a random or indexed bucket list inspiration.
   */
  function getRandomInspiration(excludeIndex = -1) {
    if (INSPIRATIONS.length === 0) return null;
    let index;
    if (INSPIRATIONS.length === 1) {
      index = 0;
    } else {
      do {
        index = Math.floor(Math.random() * INSPIRATIONS.length);
      } while (index === excludeIndex && INSPIRATIONS.length > 1);
    }
    return { ...INSPIRATIONS[index], index };
  }

  /**
   * The 4 playful manifesto rules of bList.
   */
  const MANIFESTO_RULES = [
    { rule: 1, title: 'Pin First, Explore Forever', desc: 'Save places whenever inspiration strikes. Real adventures happen outside the algorithm.' },
    { rule: 2, title: 'Zero AI Hallucinations', desc: '100% deterministic parsing & OpenStreetMap coords. Places that actually exist with zero fluff.' },
    { rule: 3, title: 'Back-Alley Noodles Rule', desc: 'The best discoveries aren\'t on generic top-10 sponsored lists.' },
    { rule: 4, title: 'Your Data, Your Journey', desc: 'Private multi-device sync, zero trackers, full GeoJSON/CSV export ownership.' }
  ];

  /**
   * Travel Milestone and Achievement Badge Definitions.
   */
  const BADGE_DEFINITIONS = [
    {
      id: 'first_pin',
      name: 'Trailblazer',
      emoji: '🌟',
      description: 'Save your very first bucket list place',
      requirement: '1 place saved',
      check: (s) => (s.totalPins || 0) >= 1,
      progress: (s) => ({ current: Math.min(s.totalPins || 0, 1), target: 1 })
    },
    {
      id: 'first_visit',
      name: 'First Stamp',
      emoji: '🎓',
      description: 'Visit and check off your first bucket list destination',
      requirement: '1 place visited',
      check: (s) => (s.visitedPins || 0) >= 1,
      progress: (s) => ({ current: Math.min(s.visitedPins || 0, 1), target: 1 })
    },
    {
      id: 'list_completed',
      name: 'Mission Complete',
      emoji: '💯',
      description: 'Visit all places in any trip list (minimum 2 places)',
      requirement: '1 complete list',
      check: (s) => (s.completedLists || 0) >= 1,
      progress: (s) => ({ current: Math.min(s.completedLists || 0, 1), target: 1 })
    },
    {
      id: 'wanderer_10',
      name: 'Avid Wanderer',
      emoji: '✈️',
      description: 'Save 10+ dream destinations to your map',
      requirement: '10 places saved',
      check: (s) => (s.totalPins || 0) >= 10,
      progress: (s) => ({ current: Math.min(s.totalPins || 0, 10), target: 10 })
    },
    {
      id: 'master_explorer',
      name: 'Master Explorer',
      emoji: '🧭',
      description: 'Save 25+ places across the globe',
      requirement: '25 places saved',
      check: (s) => (s.totalPins || 0) >= 25,
      progress: (s) => ({ current: Math.min(s.totalPins || 0, 25), target: 25 })
    },
    {
      id: 'multi_tripper',
      name: 'Multi-Tripper',
      emoji: '🗺️',
      description: 'Organize destinations into 3+ custom trip lists',
      requirement: '3 custom lists',
      check: (s) => (s.totalLists || 0) >= 3,
      progress: (s) => ({ current: Math.min(s.totalLists || 0, 3), target: 3 })
    },
    {
      id: 'noodle_hunter',
      name: 'Noodle Hunter',
      emoji: '🍜',
      description: 'Save 3+ culinary spots, restaurants, or cafes',
      requirement: '3 food spots',
      check: (s) => (s.foodPins || 0) >= 3,
      progress: (s) => ({ current: Math.min(s.foodPins || 0, 3), target: 3 })
    },
    {
      id: 'nature_lover',
      name: 'Nature Lover',
      emoji: '🏔️',
      description: 'Save 3+ parks, trails, beaches, or outdoor spots',
      requirement: '3 nature spots',
      check: (s) => (s.naturePins || 0) >= 3,
      progress: (s) => ({ current: Math.min(s.naturePins || 0, 3), target: 3 })
    },
    {
      id: 'priority_vip',
      name: 'Must-See VIP',
      emoji: '⭐',
      description: 'Mark 3+ places as Top Priority Must-See',
      requirement: '3 priority places',
      check: (s) => (s.priorityPins || 0) >= 3,
      progress: (s) => ({ current: Math.min(s.priorityPins || 0, 3), target: 3 })
    },
    {
      id: 'sync_maverick',
      name: 'Multi-Device Maverick',
      emoji: '📲',
      description: 'Sync your bucket list with another phone or browser',
      requirement: 'Device synced',
      check: (s) => !!s.isSynced,
      progress: (s) => ({ current: s.isSynced ? 1 : 0, target: 1 })
    },
    {
      id: 'secret_cartographer',
      name: 'Secret Cartographer',
      emoji: '🏆',
      description: 'Discover the secret logo easter egg',
      requirement: 'Easter egg found',
      check: (s) => !!s.easterEggUnlocked,
      progress: (s) => ({ current: s.easterEggUnlocked ? 1 : 0, target: 1 })
    }
  ];

  /**
   * Computes achievement badge progress and unlocks based on current user pins and lists.
   */
  function calculateBadges(statsOrData = {}) {
    let stats = {};
    if (statsOrData && Array.isArray(statsOrData.pins)) {
      const pins = statsOrData.pins || [];
      const lists = statsOrData.lists || [];
      const totalPins = pins.length;
      const visitedPins = pins.filter(p => p.visited === 1 || p.visited === true).length;
      const foodPins = pins.filter(p => {
        const cat = (p.category || '').toLowerCase();
        return cat.includes('food') || cat.includes('drink') || cat.includes('cafe') || cat.includes('restaurant') || cat.includes('bar');
      }).length;
      const naturePins = pins.filter(p => {
        const cat = (p.category || '').toLowerCase();
        return cat.includes('nature') || cat.includes('outdoor') || cat.includes('park') || cat.includes('trail') || cat.includes('beach');
      }).length;
      const priorityPins = pins.filter(p => p.priority === 1 || p.priority === true).length;

      // Completed lists: lists with >= 2 pins where all pins are visited
      let completedLists = 0;
      const listMap = new Map();
      pins.forEach(p => {
        const listId = p.list_id || 1;
        if (!listMap.has(listId)) listMap.set(listId, []);
        listMap.get(listId).push(p);
      });
      listMap.forEach(listPins => {
        if (listPins.length >= 2 && listPins.every(p => p.visited === 1 || p.visited === true)) {
          completedLists++;
        }
      });

      stats = {
        totalPins,
        visitedPins,
        foodPins,
        naturePins,
        priorityPins,
        totalLists: Math.max(lists.length, 1),
        completedLists,
        isSynced: !!statsOrData.isSynced,
        easterEggUnlocked: !!statsOrData.easterEggUnlocked
      };
    } else {
      stats = { ...statsOrData };
    }

    const badges = BADGE_DEFINITIONS.map(badge => {
      const unlocked = !!badge.check(stats);
      const prog = badge.progress ? badge.progress(stats) : { current: unlocked ? 1 : 0, target: 1 };
      const current = Math.min(prog.current, prog.target);
      const percentage = prog.target > 0 ? Math.min(100, Math.round((current / prog.target) * 100)) : (unlocked ? 100 : 0);

      return {
        id: badge.id,
        name: badge.name,
        emoji: badge.emoji,
        description: badge.description,
        requirement: badge.requirement,
        unlocked,
        current,
        target: prog.target,
        percentage
      };
    });

    const unlockedCount = badges.filter(b => b.unlocked).length;
    return {
      badges,
      unlockedCount,
      totalBadges: badges.length,
      percentage: Math.round((unlockedCount / badges.length) * 100),
      stats
    };
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

  /**
   * Evaluates touch coordinates to detect deliberate swipe gestures.
   * Prevents false triggers from vertical scrolling, pinch-to-zoom, or diagonal drags.
   */
  function detectSwipeGesture(options = {}) {
    const {
      startX = 0,
      startY = 0,
      endX = 0,
      endY = 0,
      minDistance = 45,
      maxPerpendicular = 60,
      edgeThreshold = 45,
      screenWidth = 375
    } = options;

    const deltaX = endX - startX;
    const deltaY = endY - startY;
    const absX = Math.abs(deltaX);
    const absY = Math.abs(deltaY);

    const isLeftEdge = startX <= edgeThreshold;
    const isRightEdge = startX >= screenWidth - edgeThreshold;

    // Horizontal swipe (Left or Right)
    if (absX >= minDistance && absY <= maxPerpendicular && absX > absY * 1.25) {
      const direction = deltaX > 0 ? 'right' : 'left';
      return {
        isSwipe: true,
        direction,
        deltaX,
        deltaY,
        distance: absX,
        isLeftEdge,
        isRightEdge
      };
    }

    // Vertical swipe (Up or Down)
    if (absY >= minDistance && absX <= maxPerpendicular && absY > absX * 1.25) {
      const direction = deltaY > 0 ? 'down' : 'up';
      return {
        isSwipe: true,
        direction,
        deltaX,
        deltaY,
        distance: absY,
        isLeftEdge,
        isRightEdge
      };
    }

    return {
      isSwipe: false,
      direction: null,
      deltaX,
      deltaY,
      distance: Math.max(absX, absY),
      isLeftEdge,
      isRightEdge
    };
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
    formatStreetAddress,
    formatDisplayPlusCode,
    optimizeTour2Opt,
    formatFileSize,
    parseTimeToMinutes,
    formatMinutesToTime,
    getOpeningStatus,
    generateShareLinks,
    generateQrSvg,
    detectSwipeGesture,
    getRandomInspiration,
    MANIFESTO_RULES,
    INSPIRATIONS,
    BADGE_DEFINITIONS,
    calculateBadges
  };
});
