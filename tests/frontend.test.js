const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const {
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
  formatDisplayPlusCode,
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
      assert.strictEqual(isValidHttpUrl(undefined), false);
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
      assert.strictEqual(sanitizeUrl('java\nscript:alert(1)'), '');
      assert.strictEqual(sanitizeUrl('java\r\nscript:alert(1)'), '');
      assert.strictEqual(sanitizeUrl('data:text/html,<script>alert(1)</script>'), '');
      assert.strictEqual(sanitizeUrl('vbscript:msgbox(1)'), '');
      assert.strictEqual(sanitizeUrl('file:///etc/passwd'), '');
      assert.strictEqual(sanitizeUrl('blob:http://example.com/uuid'), '');
      assert.strictEqual(sanitizeUrl('about:blank'), '');
      assert.strictEqual(sanitizeUrl('tel:+1234567890'), '');
      assert.strictEqual(sanitizeUrl('mailto:user@example.com'), '');
    });

    it('should allow valid relative paths', () => {
      assert.strictEqual(sanitizeUrl('/api/pins'), '/api/pins');
      assert.strictEqual(sanitizeUrl('/static/images/logo.png'), '/static/images/logo.png');
      assert.strictEqual(sanitizeUrl('/api/pins?status=visited&category=Food'), '/api/pins?status=visited&category=Food');
    });

    it('should reject protocol-relative URLs starting with // to avoid open redirects', () => {
      assert.strictEqual(sanitizeUrl('//evil.com/phish'), '');
      assert.strictEqual(sanitizeUrl('///evil.com'), '');
    });

    it('should handle null, undefined, empty, and non-string inputs safely', () => {
      assert.strictEqual(sanitizeUrl(null), '');
      assert.strictEqual(sanitizeUrl(undefined), '');
      assert.strictEqual(sanitizeUrl(''), '');
      assert.strictEqual(sanitizeUrl(12345), '');
      assert.strictEqual(sanitizeUrl({}), '');
    });
  });

  describe('API URL Resolution (resolveApiUrl / ApiClient.getUrl)', () => {
    it('should preserve absolute HTTP and HTTPS URLs as-is', () => {
      assert.strictEqual(
        resolveApiUrl('https://api.example.com/pins'),
        'https://api.example.com/pins'
      );
      assert.strictEqual(
        resolveApiUrl('http://localhost:3000/api/info'),
        'http://localhost:3000/api/info'
      );
    });

    it('should return relative paths unchanged when no baseUrl is provided', () => {
      assert.strictEqual(resolveApiUrl('/api/pins'), '/api/pins');
      assert.strictEqual(resolveApiUrl('/api/lists'), '/api/lists');
    });

    it('should resolve relative paths against custom base URL with proper slash normalization', () => {
      assert.strictEqual(
        resolveApiUrl('/api/pins', 'http://localhost:3000'),
        'http://localhost:3000/api/pins'
      );
      assert.strictEqual(
        resolveApiUrl('api/pins', 'http://localhost:3000/'),
        'http://localhost:3000/api/pins'
      );
      assert.strictEqual(
        resolveApiUrl('/api/lists/1', 'https://blist.fly.dev/'),
        'https://blist.fly.dev/api/lists/1'
      );
    });

    it('should resolve against default native host when isNative is true and no base is specified', () => {
      assert.strictEqual(
        resolveApiUrl('/api/pins', '', true),
        'https://blist.fly.dev/api/pins'
      );
      assert.strictEqual(
        resolveApiUrl('api/geocode?q=Paris', '', true),
        'https://blist.fly.dev/api/geocode?q=Paris'
      );
    });

    it('should handle empty, null, or invalid endpoints safely', () => {
      assert.strictEqual(resolveApiUrl(''), '');
      assert.strictEqual(resolveApiUrl(null), '');
      assert.strictEqual(resolveApiUrl(undefined), '');
      assert.strictEqual(resolveApiUrl('   '), '');
    });
  });

  describe('Web Share Target Parsing (parseShareTargetPayload / handleIncomingShareTarget)', () => {
    it('should extract direct URL when provided in url parameter', () => {
      const result = parseShareTargetPayload({
        url: 'https://maps.google.com/?q=Tokyo+Tower',
        title: 'Tokyo Tower',
        text: 'Famous red landmark'
      });
      assert.strictEqual(result.url, 'https://maps.google.com/?q=Tokyo+Tower');
      assert.strictEqual(result.title, 'Tokyo Tower');
      assert.strictEqual(result.text, 'Famous red landmark');
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should extract embedded URL when hidden inside text parameter', () => {
      const result = parseShareTargetPayload({
        title: 'Check this place out',
        text: 'Best bakery in town! https://maps.app.goo.gl/Bakery123'
      });
      assert.strictEqual(result.url, 'https://maps.app.goo.gl/Bakery123');
      assert.strictEqual(result.title, 'Check this place out');
      assert.strictEqual(result.text, 'Best bakery in town!');
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should parse multi-line share text and derive title from first line when title is empty', () => {
      const result = parseShareTargetPayload({
        title: '',
        text: 'Senso-ji Temple\n2 Chome-3-1 Asakusa, Taito City, Tokyo\nhttps://maps.google.com/?cid=98765'
      });
      assert.strictEqual(result.url, 'https://maps.google.com/?cid=98765');
      assert.strictEqual(result.title, 'Senso-ji Temple');
      assert.ok(result.text.includes('2 Chome-3-1 Asakusa'));
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should handle Instagram post shares', () => {
      const result = parseShareTargetPayload({
        title: 'Instagram Post',
        text: 'Awesome ramen joint https://www.instagram.com/p/C_abc123xyz/'
      });
      assert.strictEqual(result.url, 'https://www.instagram.com/p/C_abc123xyz/');
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should handle Apple Maps share URLs', () => {
      const result = parseShareTargetPayload({
        text: 'https://maps.apple.com/?address=1+Infinite+Loop,+Cupertino,+CA'
      });
      assert.strictEqual(result.url, 'https://maps.apple.com/?address=1+Infinite+Loop,+Cupertino,+CA');
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should recognize plain text queries without URLs', () => {
      const result = parseShareTargetPayload({
        text: 'Louvre Museum Paris'
      });
      assert.strictEqual(result.url, '');
      assert.strictEqual(result.title, 'Louvre Museum Paris');
      assert.strictEqual(result.isUrlCandidate, false);
    });

    it('should parse URL-encoded query strings directly', () => {
      const query = '?title=Kyoto+Shrine&text=Historic+place&url=https%3A%2F%2Fmaps.google.com%2F%3Fq%3DKyoto';
      const result = parseShareTargetPayload(query);
      assert.strictEqual(result.url, 'https://maps.google.com/?q=Kyoto');
      assert.strictEqual(result.title, 'Kyoto Shrine');
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should handle URL passed purely in title parameter', () => {
      const result = parseShareTargetPayload({
        title: 'https://maps.google.com/?q=London',
        text: '',
        url: ''
      });
      assert.strictEqual(result.url, 'https://maps.google.com/?q=London');
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should handle null, undefined, empty, and non-object inputs safely', () => {
      assert.deepStrictEqual(parseShareTargetPayload(null), {
        url: '',
        title: '',
        text: '',
        rawText: '',
        isUrlCandidate: false
      });
      assert.deepStrictEqual(parseShareTargetPayload(undefined), {
        url: '',
        title: '',
        text: '',
        rawText: '',
        isUrlCandidate: false
      });
      assert.strictEqual(parseShareTargetPayload('').isUrlCandidate, false);
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

    it('should filter out pins with invalid or out-of-bounds coordinates during GeoJSON export', () => {
      const invalidPins = [
        ...mockPins,
        { id: 3, title: 'Invalid Lat High', latitude: 90.0001, longitude: 10.0 },
        { id: 4, title: 'Invalid Lat Low', latitude: -90.0001, longitude: 10.0 },
        { id: 5, title: 'Invalid Lon High', latitude: 45.0, longitude: 180.0001 },
        { id: 6, title: 'Invalid Lon Low', latitude: 45.0, longitude: -180.0001 },
        { id: 7, title: 'NaN Coords', latitude: NaN, longitude: 10.0 },
        { id: 8, title: 'Infinity Coords', latitude: Infinity, longitude: 0.0 },
        { id: 9, title: 'Null Coords', latitude: null, longitude: null },
        { id: 10, title: 'Boolean Coords', latitude: false, longitude: true },
        { id: 11, title: 'Array Coords', latitude: [48.8], longitude: [2.3] }
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

    it('should validate geographical coordinates within exact bounds (-90..90, -180..180)', () => {
      // Valid boundaries
      assert.strictEqual(validateCoordinates(0, 0), true);
      assert.strictEqual(validateCoordinates(90, 180), true);
      assert.strictEqual(validateCoordinates(-90, -180), true);
      assert.strictEqual(validateCoordinates(45.5, -122.6), true);
      assert.strictEqual(validateCoordinates('35.6895', '139.6917'), true);

      // Out of bounds
      assert.strictEqual(validateCoordinates(90.0001, 0), false);
      assert.strictEqual(validateCoordinates(-90.0001, 0), false);
      assert.strictEqual(validateCoordinates(0, 180.0001), false);
      assert.strictEqual(validateCoordinates(0, -180.0001), false);
      assert.strictEqual(validateCoordinates(1000, 2000), false);

      // Non-numeric & invalid types
      assert.strictEqual(validateCoordinates(null, 0), false);
      assert.strictEqual(validateCoordinates(0, null), false);
      assert.strictEqual(validateCoordinates(undefined, 0), false);
      assert.strictEqual(validateCoordinates(false, true), false);
      assert.strictEqual(validateCoordinates(NaN, 0), false);
      assert.strictEqual(validateCoordinates(0, Infinity), false);
      assert.strictEqual(validateCoordinates(-Infinity, 0), false);
      assert.strictEqual(validateCoordinates([48.8], 2.3), false);
      assert.strictEqual(validateCoordinates({ lat: 48.8 }, 2.3), false);
      assert.strictEqual(validateCoordinates('invalid', 0), false);
    });
  });

  describe('GPS Distance Calculation & Formatting', () => {
    it('should calculate accurate distance between two coordinate pairs', () => {
      // London (51.5074, -0.1278) to Paris (48.8566, 2.3522) ~ 343 km
      const distance = calculateDistance(51.5074, -0.1278, 48.8566, 2.3522);
      assert.ok(Math.abs(distance - 343.5) < 5.0, `Expected ~343.5km, got ${distance}`);
    });

    it('should return 0 distance for identical coordinates or invalid inputs', () => {
      assert.strictEqual(calculateDistance(35.6586, 139.7454, 35.6586, 139.7454), 0);
      assert.strictEqual(calculateDistance(NaN, 0, 10, 10), 0);
      assert.strictEqual(calculateDistance(0, 0, null, 10), 0);
    });

    it('should format distances properly in meters for < 1km and miles/km for >= 1km', () => {
      assert.strictEqual(formatDistance(0.45), '450 m away');
      assert.strictEqual(formatDistance(0.05), '50 m away');
      assert.strictEqual(formatDistance(10.0), '6.2 mi away (10.0 km)');
      assert.strictEqual(formatDistance(100.0), '62.1 mi away (100.0 km)');
      assert.strictEqual(formatDistance(-5), '0 m away');
      assert.strictEqual(formatDistance(NaN), '0 m away');
    });
  });

  describe('Plus Code / Open Location Code Encoding', () => {
    it('should encode Eiffel Tower coordinates into a valid Plus Code', () => {
      const code = encodePlusCode(48.8584, 2.2945);
      assert.ok(code.length >= 8);
      assert.ok(code.includes('+'));
    });

    it('should handle edge cases like poles and meridian', () => {
      assert.ok(encodePlusCode(90.0, 0.0).includes('+'));
      assert.ok(encodePlusCode(-90.0, 0.0).includes('+'));
      assert.ok(encodePlusCode(0.0, 180.0).includes('+'));
      assert.strictEqual(encodePlusCode(NaN, 0.0), '');
    });

    it('should format Plus Codes into Google Maps short code + locality format when address is present', () => {
      const formattedBYU = formatDisplayPlusCode('85GC68XX+RM', 'Talmage Building, BYU, Provo, UT 84602');
      assert.strictEqual(formattedBYU, '68XX+RM Provo, UT');

      const formattedSimple = formatDisplayPlusCode('85GC68XX+RM', 'Provo, Utah');
      assert.strictEqual(formattedSimple, '68XX+RM Provo, Utah');

      const formattedParis = formatDisplayPlusCode('8FW4V8FX+9H', 'Champ de Mars, 5 Av. Anatole France, 75007 Paris, France');
      assert.strictEqual(formattedParis, 'V8FX+9H Paris, France');

      // Fall back to full code if address is empty
      assert.strictEqual(formatDisplayPlusCode('85GC68XX+RM', ''), '85GC68XX+RM');
      assert.strictEqual(formatDisplayPlusCode('85GC68XX+RM', null), '85GC68XX+RM');
    });

    it('should extract clean city/state locality from address strings', () => {
      assert.strictEqual(extractLocality('Provo, UT 84602'), 'Provo, UT');
      assert.strictEqual(extractLocality('Highland Park, Los Angeles, CA'), 'Los Angeles, CA');
      assert.strictEqual(extractLocality('4 Chome-2-8 Shibakoen, Minato City, Tokyo, Japan'), 'Tokyo, Japan');
      assert.strictEqual(extractLocality(''), '');
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
      const searchTitle = filterPins(pins, { search: 'ichiran' });
      assert.strictEqual(searchTitle.length, 1);
      assert.strictEqual(searchTitle[0].title, 'Ichiran Ramen');

      const searchAddress = filterPins(pins, { search: 'Harajuku' });
      assert.strictEqual(searchAddress.length, 1);
      assert.strictEqual(searchAddress[0].title, 'Meiji Shrine');

      const searchNotes = filterPins(pins, { search: 'cold brew' });
      assert.strictEqual(searchNotes.length, 1);
      assert.strictEqual(searchNotes[0].title, 'Blue Bottle Coffee');

      const searchDesc = filterPins(pins, { search: 'morning market' });
      assert.strictEqual(searchDesc.length, 1);
      assert.strictEqual(searchDesc[0].title, 'Tsukiji Outer Market');

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

    it('should handle empty or null pin list safely', () => {
      assert.deepStrictEqual(filterPins(null), []);
      assert.deepStrictEqual(filterPins(undefined), []);
      assert.deepStrictEqual(filterPins([]), []);
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

    it('should handle null or empty lists safely', () => {
      assert.deepStrictEqual(sortPins(null), []);
      assert.deepStrictEqual(sortPins([]), []);
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

  describe('Trip Planning Progress & Route Calculations', () => {
    it('should correctly calculate trip progress percentage', () => {
      const calculateProgress = (pins) => {
        if (!pins || pins.length === 0) return { total: 0, visited: 0, percentage: 0 };
        const total = pins.length;
        const visited = pins.filter(p => p.visited).length;
        const percentage = Math.round((visited / total) * 100);
        return { total, visited, percentage };
      };

      assert.deepStrictEqual(calculateProgress([]), { total: 0, visited: 0, percentage: 0 });
      assert.deepStrictEqual(
        calculateProgress([{ id: 1, visited: false }, { id: 2, visited: true }, { id: 3, visited: false }, { id: 4, visited: true }]),
        { total: 4, visited: 2, percentage: 50 }
      );
      assert.deepStrictEqual(
        calculateProgress([{ id: 1, visited: true }, { id: 2, visited: true }]),
        { total: 2, visited: 2, percentage: 100 }
      );
    });

    it('should calculate sequential route distances between multi-stop pins', () => {
      const calculateTotalRoute = (pins) => {
        if (!pins || pins.length < 2) return 0;
        let total = 0;
        for (let i = 0; i < pins.length - 1; i++) {
          total += calculateDistance(pins[i].latitude, pins[i].longitude, pins[i + 1].latitude, pins[i + 1].longitude);
        }
        return total;
      };

      const testPins = [
        { latitude: 48.8584, longitude: 2.2945 }, // Eiffel Tower, Paris
        { latitude: 48.8606, longitude: 2.3376 }, // Louvre Museum, Paris (~3.2 km)
        { latitude: 48.8529, longitude: 2.3500 }  // Notre-Dame, Paris (~1.2 km)
      ];

      const routeKm = calculateTotalRoute(testPins);
      assert.ok(routeKm > 4.0 && routeKm < 5.0, `Expected total route between 4 and 5 km, got ${routeKm}`);
    });

    it('should generate a valid Google Maps multi-stop directions URL', () => {
      const testPins = [
        { latitude: 35.6586, longitude: 139.7454 }, // Tokyo Tower
        { latitude: 35.7148, longitude: 139.7967 }, // Senso-ji
        { latitude: 35.6595, longitude: 139.7005 }  // Shibuya Crossing
      ];

      const url = generateGoogleMapsRouteUrl(testPins);
      assert.strictEqual(
        url,
        'https://www.google.com/maps/dir/35.6586,139.7454/35.7148,139.7967/35.6595,139.7005'
      );
    });

    it('should cap Google Maps route stops at maxStops and reject invalid inputs', () => {
      assert.strictEqual(generateGoogleMapsRouteUrl(null), null);
      assert.strictEqual(generateGoogleMapsRouteUrl([]), null);
      assert.strictEqual(generateGoogleMapsRouteUrl([{ latitude: 10, longitude: 20 }]), null);

      const manyPins = Array.from({ length: 15 }, (_, i) => ({
        latitude: 10 + i,
        longitude: 20 + i
      }));
      const url = generateGoogleMapsRouteUrl(manyPins, 5);
      const parts = url.replace('https://www.google.com/maps/dir/', '').split('/');
      assert.strictEqual(parts.length, 5);
    });
  });

});
