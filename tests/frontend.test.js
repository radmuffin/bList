const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const {
  escapeHtml,
  isValidHttpUrl,
  sanitizeUrl,
  calculateDistance,
  formatDistance,
  validateCoordinates,
  pinsToGeoJSON,
  geoJSONToPins,
  filterPins,
  sortPins,
  getEffectiveTheme,
  APP_INFO,
  getAppInfo
} = require('../static/helpers.js');

describe('Frontend Unit Tests: Helpers Suite', () => {

  describe('HTML Escaping (escapeHtml)', () => {
    it('should escape dangerous HTML characters to prevent XSS', () => {
      const input = '<script>alert("XSS & \'attack\'")</script>';
      const expected = '&lt;script&gt;alert(&quot;XSS &amp; &#39;attack&#39;&quot;)&lt;/script&gt;';
      assert.strictEqual(escapeHtml(input), expected);
    });

    it('should handle special HTML entity characters individually', () => {
      assert.strictEqual(escapeHtml('&'), '&amp;');
      assert.strictEqual(escapeHtml('<'), '&lt;');
      assert.strictEqual(escapeHtml('>'), '&gt;');
      assert.strictEqual(escapeHtml('"'), '&quot;');
      assert.strictEqual(escapeHtml("'"), '&#39;');
    });

    it('should handle null and undefined safely without throwing', () => {
      assert.strictEqual(escapeHtml(null), '');
      assert.strictEqual(escapeHtml(undefined), '');
      assert.strictEqual(escapeHtml(''), '');
    });

    it('should convert numbers and booleans to safe strings', () => {
      assert.strictEqual(escapeHtml(12345), '12345');
      assert.strictEqual(escapeHtml(true), 'true');
      assert.strictEqual(escapeHtml(false), 'false');
    });

    it('should preserve standard safe text unchanged', () => {
      const text = 'Eiffel Tower, Champ de Mars, 75007 Paris';
      assert.strictEqual(escapeHtml(text), text);
    });
  });

  describe('URL Validation & Sanitization (isValidHttpUrl & sanitizeUrl)', () => {
    it('should validate proper HTTP and HTTPS URLs', () => {
      assert.strictEqual(isValidHttpUrl('https://maps.google.com/?q=Paris'), true);
      assert.strictEqual(isValidHttpUrl('http://example.com/place'), true);
      assert.strictEqual(isValidHttpUrl('https://www.instagram.com/p/C_12345/'), true);
    });

    it('should reject invalid or non-HTTP URLs', () => {
      assert.strictEqual(isValidHttpUrl('ftp://ftp.example.com'), false);
      assert.strictEqual(isValidHttpUrl('javascript:alert(1)'), false);
      assert.strictEqual(isValidHttpUrl(''), false);
      assert.strictEqual(isValidHttpUrl(null), false);
      assert.strictEqual(isValidHttpUrl('not a url'), false);
    });

    it('should sanitize and allow clean HTTP/HTTPS URLs', () => {
      assert.strictEqual(
        sanitizeUrl('https://maps.google.com/place/Tokyo'),
        'https://maps.google.com/place/Tokyo'
      );
      assert.strictEqual(
        sanitizeUrl('http://example.com/'),
        'http://example.com/'
      );
    });

    it('should block javascript: and malicious protocols', () => {
      assert.strictEqual(sanitizeUrl('javascript:alert("XSS")'), '');
      assert.strictEqual(sanitizeUrl('JAVASCRIPT:alert(1)'), '');
      assert.strictEqual(sanitizeUrl('data:text/html,<script>alert(1)</script>'), '');
      assert.strictEqual(sanitizeUrl('vbscript:msgbox(1)'), '');
      assert.strictEqual(sanitizeUrl('file:///etc/passwd'), '');
    });

    it('should allow valid relative paths', () => {
      assert.strictEqual(sanitizeUrl('/api/pins'), '/api/pins');
      assert.strictEqual(sanitizeUrl('/static/images/logo.png'), '/static/images/logo.png');
    });

    it('should reject protocol-relative URLs starting with // to avoid open redirects', () => {
      assert.strictEqual(sanitizeUrl('//evil.com/phish'), '');
    });
  });

  describe('GeoJSON Formatting & Parsing (pinsToGeoJSON & geoJSONToPins)', () => {
    const mockPins = [
      {
        id: 1,
        list_id: 1,
        title: 'Eiffel Tower',
        description: 'Iconic iron tower',
        latitude: 48.8584,
        longitude: 2.2945,
        category: 'Sightseeing',
        source_url: 'https://example.com/eiffel',
        image_url: 'https://example.com/eiffel.jpg',
        address: 'Champ de Mars, Paris',
        notes: 'Visit at sunset',
        visited: true,
        created_at: '2026-08-30T10:00:00Z'
      },
      {
        id: 2,
        list_id: 2,
        title: 'Tokyo Skytree',
        description: 'Tall broadcasting tower',
        latitude: 35.7100,
        longitude: 139.8107,
        category: 'Sightseeing',
        source_url: null,
        image_url: null,
        address: 'Sumida City, Tokyo',
        notes: 'Great observation deck',
        visited: false,
        created_at: '2026-08-30T11:00:00Z'
      }
    ];

    it('should convert an array of pins to standard GeoJSON FeatureCollection', () => {
      const geojson = pinsToGeoJSON(mockPins);

      assert.strictEqual(geojson.type, 'FeatureCollection');
      assert.strictEqual(geojson.features.length, 2);

      const feat1 = geojson.features[0];
      assert.strictEqual(feat1.type, 'Feature');
      assert.strictEqual(feat1.geometry.type, 'Point');
      // Standard GeoJSON coordinate order: [lon, lat]
      assert.strictEqual(feat1.geometry.coordinates[0], 2.2945);
      assert.strictEqual(feat1.geometry.coordinates[1], 48.8584);
      assert.strictEqual(feat1.properties.title, 'Eiffel Tower');
      assert.strictEqual(feat1.properties.visited, true);
      assert.strictEqual(feat1.properties.category, 'Sightseeing');
    });

    it('should filter out pins with invalid coordinates during GeoJSON export', () => {
      const invalidPins = [
        ...mockPins,
        { id: 3, title: 'Invalid Lat', latitude: 999.0, longitude: 10.0 },
        { id: 4, title: 'NaN Coords', latitude: NaN, longitude: 10.0 },
        { id: 5, title: 'Invalid Lon', latitude: 45.0, longitude: -500.0 }
      ];

      const geojson = pinsToGeoJSON(invalidPins);
      assert.strictEqual(geojson.features.length, 2);
    });

    it('should parse a GeoJSON FeatureCollection back into pins', () => {
      const geojson = pinsToGeoJSON(mockPins);
      const parsedPins = geoJSONToPins(geojson);

      assert.strictEqual(parsedPins.length, 2);
      assert.strictEqual(parsedPins[0].title, 'Eiffel Tower');
      assert.strictEqual(parsedPins[0].latitude, 48.8584);
      assert.strictEqual(parsedPins[0].longitude, 2.2945);
      assert.strictEqual(parsedPins[0].visited, true);
      assert.strictEqual(parsedPins[1].title, 'Tokyo Skytree');
      assert.strictEqual(parsedPins[1].latitude, 35.7100);
      assert.strictEqual(parsedPins[1].longitude, 139.8107);
      assert.strictEqual(parsedPins[1].visited, false);
    });

    it('should parse a single GeoJSON Feature object', () => {
      const singleFeature = {
        type: 'Feature',
        geometry: { type: 'Point', coordinates: [-122.4194, 37.7749] },
        properties: { id: 42, title: 'San Francisco Bay', visited: true }
      };

      const parsed = geoJSONToPins(singleFeature);
      assert.strictEqual(parsed.length, 1);
      assert.strictEqual(parsed[0].title, 'San Francisco Bay');
      assert.strictEqual(parsed[0].latitude, 37.7749);
      assert.strictEqual(parsed[0].longitude, -122.4194);
      assert.strictEqual(parsed[0].visited, true);
    });

    it('should validate geographical coordinates within bounds (-90..90, -180..180)', () => {
      assert.strictEqual(validateCoordinates(0, 0), true);
      assert.strictEqual(validateCoordinates(90, 180), true);
      assert.strictEqual(validateCoordinates(-90, -180), true);
      assert.strictEqual(validateCoordinates(90.1, 0), false);
      assert.strictEqual(validateCoordinates(-90.1, 0), false);
      assert.strictEqual(validateCoordinates(0, 180.1), false);
      assert.strictEqual(validateCoordinates(0, -180.1), false);
      assert.strictEqual(validateCoordinates('invalid', 0), false);
    });
  });

  describe('GPS Distance Calculation & Formatting', () => {
    it('should calculate accurate distance between two coordinate pairs', () => {
      // London (51.5074, -0.1278) to Paris (48.8566, 2.3522) ~ 343 km
      const distance = calculateDistance(51.5074, -0.1278, 48.8566, 2.3522);
      assert.ok(Math.abs(distance - 343.5) < 5.0, `Expected ~343.5km, got ${distance}`);
    });

    it('should return 0 distance for identical coordinates', () => {
      const distance = calculateDistance(35.6586, 139.7454, 35.6586, 139.7454);
      assert.strictEqual(distance, 0);
    });

    it('should format distances properly in meters for < 1km and miles/km for >= 1km', () => {
      assert.strictEqual(formatDistance(0.45), '450 m away');
      assert.strictEqual(formatDistance(0.05), '50 m away');
      assert.strictEqual(formatDistance(10.0), '6.2 mi away (10.0 km)');
      assert.strictEqual(formatDistance(100.0), '62.1 mi away (100.0 km)');
    });
  });

  describe('Search & Filter Predicates (filterPins)', () => {
    const pins = [
      { id: 1, list_id: 1, title: 'Blue Bottle Coffee', category: 'Cafe', visited: false, address: 'Shinjuku, Tokyo', notes: 'Great cold brew', description: 'Specialty cafe' },
      { id: 2, list_id: 1, title: 'Ichiran Ramen', category: 'Food & Drink', visited: true, address: 'Shibuya, Tokyo', notes: 'Tonkotsu broth', description: 'Solo booth dining' },
      { id: 3, list_id: 2, title: 'Meiji Shrine', category: 'Sightseeing', visited: false, address: 'Harajuku, Tokyo', notes: 'Peaceful forest', description: 'Historical shrine' },
      { id: 4, list_id: 2, title: 'Tsukiji Outer Market', category: 'Food & Drink', visited: true, address: 'Tsukiji, Tokyo', notes: 'Fresh seafood bowls', description: 'Bustling morning market' }
    ];

    it('should filter by list ID', () => {
      const list1Pins = filterPins(pins, { listFilter: '1' });
      assert.strictEqual(list1Pins.length, 2);
      assert.ok(list1Pins.every(p => p.list_id === 1));

      const list2Pins = filterPins(pins, { listFilter: '2' });
      assert.strictEqual(list2Pins.length, 2);
      assert.ok(list2Pins.every(p => p.list_id === 2));
    });

    it('should filter by visited vs bucket list status', () => {
      const bucketPins = filterPins(pins, { status: 'bucket' });
      assert.strictEqual(bucketPins.length, 2);
      assert.ok(bucketPins.every(p => !p.visited));

      const visitedPins = filterPins(pins, { status: 'visited' });
      assert.strictEqual(visitedPins.length, 2);
      assert.ok(visitedPins.every(p => p.visited));
    });

    it('should filter by category', () => {
      const cafes = filterPins(pins, { category: 'Cafe' });
      assert.strictEqual(cafes.length, 1);
      assert.strictEqual(cafes[0].title, 'Blue Bottle Coffee');

      const food = filterPins(pins, { category: 'Food & Drink' });
      assert.strictEqual(food.length, 2);

      const all = filterPins(pins, { category: 'All' });
      assert.strictEqual(all.length, 4);
    });

    it('should search case-insensitively across title, address, notes, and description', () => {
      // Search title
      const searchTitle = filterPins(pins, { search: 'ichiran' });
      assert.strictEqual(searchTitle.length, 1);
      assert.strictEqual(searchTitle[0].title, 'Ichiran Ramen');

      // Search address
      const searchAddress = filterPins(pins, { search: 'Harajuku' });
      assert.strictEqual(searchAddress.length, 1);
      assert.strictEqual(searchAddress[0].title, 'Meiji Shrine');

      // Search notes
      const searchNotes = filterPins(pins, { search: 'cold brew' });
      assert.strictEqual(searchNotes.length, 1);
      assert.strictEqual(searchNotes[0].title, 'Blue Bottle Coffee');

      // Search description
      const searchDesc = filterPins(pins, { search: 'morning market' });
      assert.strictEqual(searchDesc.length, 1);
      assert.strictEqual(searchDesc[0].title, 'Tsukiji Outer Market');

      // Non-matching search
      const noMatch = filterPins(pins, { search: 'nonexistentquery123' });
      assert.strictEqual(noMatch.length, 0);
    });

    it('should support combined filtering (list + category + status + search)', () => {
      const combined = filterPins(pins, {
        listFilter: '1',
        category: 'Food & Drink',
        status: 'visited',
        search: 'broth'
      });
      assert.strictEqual(combined.length, 1);
      assert.strictEqual(combined[0].title, 'Ichiran Ramen');
    });
  });

  describe('Pin Sorting (sortPins)', () => {
    const pins = [
      { id: 1, title: 'Zebra Safari', category: 'Nature', latitude: 10.0, longitude: 10.0 },
      { id: 2, title: 'Apple Orchard', category: 'Food', latitude: 20.0, longitude: 20.0 },
      { id: 3, title: 'Grand Canyon', category: 'Sightseeing', latitude: 30.0, longitude: 30.0 }
    ];

    it('should sort by newest (ID descending) by default', () => {
      const sorted = sortPins(pins, 'newest');
      assert.deepStrictEqual(sorted.map(p => p.id), [3, 2, 1]);
    });

    it('should sort alphabetically by title (A-Z)', () => {
      const sorted = sortPins(pins, 'az');
      assert.deepStrictEqual(sorted.map(p => p.title), ['Apple Orchard', 'Grand Canyon', 'Zebra Safari']);
    });

    it('should sort alphabetically by category', () => {
      const sorted = sortPins(pins, 'category');
      assert.deepStrictEqual(sorted.map(p => p.category), ['Food', 'Nature', 'Sightseeing']);
    });

    it('should sort by nearest relative to user location', () => {
      const userLoc = { lat: 10.1, lng: 10.1 }; // Closest to Zebra Safari (10, 10)
      const sorted = sortPins(pins, 'nearest', userLoc);
      assert.deepStrictEqual(sorted.map(p => p.id), [1, 2, 3]);
    });
  });

  describe('Theme Management (getEffectiveTheme)', () => {
    it('should return explicit light or dark settings directly', () => {
      assert.strictEqual(getEffectiveTheme('light', false), 'light');
      assert.strictEqual(getEffectiveTheme('light', true), 'light');
      assert.strictEqual(getEffectiveTheme('dark', false), 'dark');
      assert.strictEqual(getEffectiveTheme('dark', true), 'dark');
    });

    it('should resolve auto setting based on system preferences', () => {
      assert.strictEqual(getEffectiveTheme('auto', true), 'dark');
      assert.strictEqual(getEffectiveTheme('auto', false), 'light');
    });
  });

  describe('App Metadata & Repository Links (getAppInfo & APP_INFO)', () => {
    it('should expose app name, version, and valid repository links', () => {
      const info = getAppInfo();
      assert.strictEqual(info.name, 'bList');
      assert.strictEqual(info.version, '0.1.0');
      assert.strictEqual(info.repositoryUrl, 'https://github.com/radmuffin/bList');
      assert.strictEqual(info.issuesUrl, 'https://github.com/radmuffin/bList/issues');
      assert.strictEqual(info.license, 'MIT');
    });

    it('should return a frozen/immutable or isolated object', () => {
      assert.strictEqual(typeof APP_INFO, 'object');
      assert.strictEqual(Object.isFrozen(APP_INFO), true);
    });
  });

});
