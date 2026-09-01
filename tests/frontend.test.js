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
  formatStreetAddress,
  formatDisplayPlusCode,
  optimizeTour2Opt,
  formatFileSize,
  getOpeningStatus,
  generateShareLinks,
  generateQrSvg,
  getRandomInspiration,
  MANIFESTO_RULES,
  INSPIRATIONS,
  BADGE_DEFINITIONS,
  calculateBadges,
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

    it('should parse bList shared URL with title and coordinate parameters', () => {
      const result = parseShareTargetPayload({
        url: 'https://blist.fly.dev/?lat=35.6586&lng=139.7454&title=Tokyo+Tower&address=Tokyo%2C+Japan',
        title: 'Tokyo Tower | bList',
        text: 'Check out Tokyo Tower (Tokyo, Japan) on my travel bucket list!'
      });
      assert.strictEqual(result.url, 'https://blist.fly.dev/?lat=35.6586&lng=139.7454&title=Tokyo+Tower&address=Tokyo%2C+Japan');
      assert.strictEqual(result.title, 'Tokyo Tower | bList');
      assert.strictEqual(result.isUrlCandidate, true);
    });

    it('should handle bList text shares with root URL by deriving place name and ignoring root app URL', () => {
      const result = parseShareTargetPayload({
        text: 'Check out Tokyo Tower (Minato, Tokyo) on my travel bucket list!\nhttps://blist.fly.dev/'
      });
      assert.strictEqual(result.url, '');
      assert.strictEqual(result.title, 'Tokyo Tower (Minato, Tokyo)');
      assert.strictEqual(result.isUrlCandidate, false);
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

    it('should format street address without title, city, county, state, or zip redundancies', () => {
      // Nominatim reverse-geocoded address with title & BYU campus & county
      const hbl = formatStreetAddress(
        'Harold B. Lee Library, 2060, East 1080 North, BYU, Provo, Utah County, Utah, 84604, United States',
        'Harold B. Lee Library'
      );
      assert.strictEqual(hbl, '2060 East 1080 North');

      // Standard street + city state zip
      const rugged = formatStreetAddress('397 E 200 N, Provo, UT 84606', 'Rugged Grounds');
      assert.strictEqual(rugged, '397 E 200 N');

      // POI with title in address
      const spaceNeedle = formatStreetAddress('Space Needle, 400 Broad St, Seattle, WA 98109', 'Space Needle');
      assert.strictEqual(spaceNeedle, '400 Broad St');

      // International address
      const tokyo = formatStreetAddress('Tokyo Tower, 4 Chome-2-8 Shibakoen, Minato City, Tokyo, Japan', 'Tokyo Tower');
      assert.strictEqual(tokyo, '4 Chome-2-8 Shibakoen');

      // Single segment / edge cases
      assert.strictEqual(formatStreetAddress('123 Main St', 'Cafe'), '123 Main St');
      assert.strictEqual(formatStreetAddress('', 'Cafe'), '');
      assert.strictEqual(formatStreetAddress(null, 'Cafe'), '');
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

  describe('Multi-Tagging, Priority & Day Grouping Filters (filterPins extensions)', () => {
    const testPins = [
      { id: 1, title: 'Shibuya Sky', category: 'Sightseeing', tags: '#sunset #view #tokyo', priority: 1, day_group: 1, visited: 0 },
      { id: 2, title: 'Fuunji Ramen', category: 'Food & Drink', tags: '#must-eat #ramen #lunch', priority: 1, day_group: 1, visited: 1 },
      { id: 3, title: 'Meiji Shrine', category: 'Sightseeing', tags: '#culture #nature', priority: 0, day_group: 2, visited: 0 },
      { id: 4, title: 'TeamLab Planets', category: 'Sightseeing', tags: '#art #must-see #indoor', priority: 1, day_group: 2, visited: 0 },
      { id: 5, title: 'Starbucks Reserve Roastery', category: 'Cafe', tags: '#coffee #view', priority: 0, day_group: 0, visited: 0 }
    ];

    it('should filter by specific tag chip', () => {
      const sunsetPins = filterPins(testPins, { tag: 'sunset' });
      assert.strictEqual(sunsetPins.length, 1);
      assert.strictEqual(sunsetPins[0].title, 'Shibuya Sky');

      const viewPins = filterPins(testPins, { tag: '#view' });
      assert.strictEqual(viewPins.length, 2);
    });

    it('should filter by priority ⭐ Must-See places only', () => {
      const priorityPins = filterPins(testPins, { priorityOnly: true });
      assert.strictEqual(priorityPins.length, 3);
      assert.ok(priorityPins.every(p => p.priority === 1));
    });

    it('should filter by day itinerary planner group', () => {
      const day1Pins = filterPins(testPins, { dayGroup: 1 });
      assert.strictEqual(day1Pins.length, 2);

      const day2Pins = filterPins(testPins, { dayGroup: 2 });
      assert.strictEqual(day2Pins.length, 2);

      const unassignedPins = filterPins(testPins, { dayGroup: 0 });
      assert.strictEqual(unassignedPins.length, 1);
    });

    it('should search by tag within search query', () => {
      const searched = filterPins(testPins, { search: 'ramen' });
      assert.strictEqual(searched.length, 1);
      assert.strictEqual(searched[0].title, 'Fuunji Ramen');
    });
  });

  describe('1-Click TSP 2-Opt Route Optimizer (optimizeTour2Opt)', () => {
    it('should handle small pin arrays without mutation errors', () => {
      assert.deepStrictEqual(optimizeTour2Opt([]), []);
      assert.deepStrictEqual(optimizeTour2Opt(null), []);
      const single = [{ latitude: 10, longitude: 10 }];
      assert.deepStrictEqual(optimizeTour2Opt(single), single);
    });

    it('should produce an optimal or improved sequence for collinear or clustered points', () => {
      // 4 points along a line out of order: A (0,0), D (0,30), B (0,10), C (0,20)
      const disordered = [
        { id: 'A', latitude: 0, longitude: 0 },
        { id: 'D', latitude: 0, longitude: 30 },
        { id: 'B', latitude: 0, longitude: 10 },
        { id: 'C', latitude: 0, longitude: 20 }
      ];

      const tour = optimizeTour2Opt(disordered);
      assert.strictEqual(tour.length, 4);
      assert.strictEqual(tour[0].id, 'A');

      // Calculate total route distance before and after
      const dist = (arr) => {
        let total = 0;
        for (let i = 0; i < arr.length - 1; i++) {
          total += calculateDistance(arr[i].latitude, arr[i].longitude, arr[i + 1].latitude, arr[i + 1].longitude);
        }
        return total;
      };

      const disorderedDist = dist(disordered);
      const optimizedDist = dist(tour);
      assert.ok(optimizedDist <= disorderedDist, `Optimized dist ${optimizedDist} should be <= disordered ${disorderedDist}`);
    });

    it('should handle edge cases: 0, 1, 2, identical coordinates, and invalid inputs', () => {
      assert.deepStrictEqual(optimizeTour2Opt([]), []);
      assert.deepStrictEqual(optimizeTour2Opt(null), []);
      assert.deepStrictEqual(optimizeTour2Opt(undefined), []);

      const single = [{ id: 1, latitude: 10, longitude: 10 }];
      assert.strictEqual(optimizeTour2Opt(single).length, 1);

      const pair = [
        { id: 1, latitude: 10, longitude: 10 },
        { id: 2, latitude: 20, longitude: 20 }
      ];
      assert.strictEqual(optimizeTour2Opt(pair).length, 2);

      // Identical coordinates
      const identical = [
        { id: 1, latitude: 35.6895, longitude: 139.6917 },
        { id: 2, latitude: 35.6895, longitude: 139.6917 },
        { id: 3, latitude: 35.6895, longitude: 139.6917 }
      ];
      const resIdentical = optimizeTour2Opt(identical);
      assert.strictEqual(resIdentical.length, 3);

      // Collinear points: (0,0), (0,30), (0,10), (0,20) -> should be sorted sequentially
      const collinear = [
        { id: 'start', latitude: 0, longitude: 0 },
        { id: 'end', latitude: 0, longitude: 30 },
        { id: 'mid1', latitude: 0, longitude: 10 },
        { id: 'mid2', latitude: 0, longitude: 20 }
      ];
      const resCollinear = optimizeTour2Opt(collinear);
      assert.strictEqual(resCollinear.length, 4);
      assert.strictEqual(resCollinear[0].id, 'start');
      assert.strictEqual(resCollinear[1].id, 'mid1');
      assert.strictEqual(resCollinear[2].id, 'mid2');
      assert.strictEqual(resCollinear[3].id, 'end');
    });
  });

  describe('File Size Formatting Helper (formatFileSize)', () => {
    it('should format file sizes in bytes, KB, and MB accurately', () => {
      assert.strictEqual(formatFileSize(0), '0 B');
      assert.strictEqual(formatFileSize(500), '500 B');
      assert.strictEqual(formatFileSize(1024), '1.0 KB');
      assert.strictEqual(formatFileSize(2048), '2.0 KB');
      assert.strictEqual(formatFileSize(1048576), '1.0 MB');
      assert.strictEqual(formatFileSize(5242880), '5.0 MB');
    });
  });

  describe('Opening Hours & Open Now / Closed Status Indicator (getOpeningStatus)', () => {
    it('should evaluate 24/7 locations as Open 24/7', () => {
      const res1 = getOpeningStatus('24/7');
      assert.strictEqual(res1.status, 'open');
      assert.strictEqual(res1.isOpen, true);
      assert.strictEqual(res1.label, 'Open 24/7');
      assert.strictEqual(res1.badgeClass, 'badge-open');

      const res2 = getOpeningStatus('Open 24 hours');
      assert.strictEqual(res2.status, 'open');
      assert.strictEqual(res2.isOpen, true);
    });

    it('should evaluate permanently or temporarily closed places', () => {
      const res = getOpeningStatus('Closed');
      assert.strictEqual(res.status, 'closed');
      assert.strictEqual(res.isOpen, false);
      assert.strictEqual(res.label, 'Closed');
      assert.strictEqual(res.badgeClass, 'badge-closed');
    });

    it('should accurately evaluate daily schedule with fixed times', () => {
      // Create a test date: Monday at 14:30 (2:30 PM)
      const mondayAfternoon = new Date('2026-08-31T14:30:00'); // Aug 31, 2026 is Monday
      
      const openSpot = getOpeningStatus('09:00 - 22:00', mondayAfternoon);
      assert.strictEqual(openSpot.status, 'open');
      assert.strictEqual(openSpot.isOpen, true);
      assert.strictEqual(openSpot.badgeClass, 'badge-open');

      const closedEarlySpot = getOpeningStatus('06:00 - 12:00', mondayAfternoon);
      assert.strictEqual(closedEarlySpot.status, 'closed');
      assert.strictEqual(closedEarlySpot.isOpen, false);
      assert.strictEqual(closedEarlySpot.badgeClass, 'badge-closed');
    });

    it('should detect closing soon within 45 minutes', () => {
      // Monday at 21:30 (9:30 PM) with closing time at 22:00 (10:00 PM) -> 30 mins left
      const mondayNight = new Date('2026-08-31T21:30:00');
      const closingSoon = getOpeningStatus('09:00 - 22:00', mondayNight);
      assert.strictEqual(closingSoon.status, 'closing_soon');
      assert.strictEqual(closingSoon.isOpen, true);
      assert.strictEqual(closingSoon.badgeClass, 'badge-closing-soon');
      assert.ok(closingSoon.label.includes('Closing Soon (30m)'));
    });

    it('should handle overnight hours (e.g. bars open 20:00 - 02:00)', () => {
      // 11:30 PM on Monday night
      const lateNight = new Date('2026-08-31T23:30:00');
      const barOpen = getOpeningStatus('20:00 - 02:00', lateNight);
      assert.strictEqual(barOpen.status, 'open');
      assert.strictEqual(barOpen.isOpen, true);

      // 3:30 AM on Monday morning (closed)
      const earlyMorning = new Date('2026-08-31T03:30:00');
      const barClosed = getOpeningStatus('20:00 - 02:00', earlyMorning);
      assert.strictEqual(barClosed.status, 'closed');
      assert.strictEqual(barClosed.isOpen, false);
    });

    it('should correctly evaluate overnight shifts on the following morning', () => {
      // Friday overnight shift: Friday 20:00 - 02:00.
      // Evaluated at Saturday 01:00 AM (2026-09-05T01:00:00 is Saturday 1 AM, preceding day was Friday Sept 4).
      const saturdayEarlyMorning = new Date('2026-09-05T01:00:00');
      const resNextMorning = getOpeningStatus('Fr 20:00-02:00; Sa Off; Su Off', saturdayEarlyMorning);
      assert.strictEqual(resNextMorning.isOpen, true);
      assert.strictEqual(resNextMorning.status, 'open');
      assert.ok(resNextMorning.label.includes('Open until 2:00 AM'));

      // Evaluated at Saturday 03:00 AM (after closing)
      const saturdayAfterClose = new Date('2026-09-05T03:00:00');
      const resClosed = getOpeningStatus('Fr 20:00-02:00; Sa Off; Su Off', saturdayAfterClose);
      assert.strictEqual(resClosed.isOpen, false);
      assert.strictEqual(resClosed.status, 'closed');
    });

    it('should handle 24:00 midnight closing format', () => {
      const tuesdayNight = new Date('2026-09-01T22:00:00'); // Tuesday 10 PM
      const res = getOpeningStatus('Mo-Fr 08:00-24:00; Sa-Su 10:00-20:00', tuesdayNight);
      assert.strictEqual(res.isOpen, true);
      assert.strictEqual(res.status, 'open');
      assert.ok(res.label.includes('12:00 AM'));
    });

    it('should support Day-of-week schedules and extended aliases (e.g. Thurs, Tues, Weds)', () => {
      const mondayNoon = new Date('2026-08-31T12:00:00'); // Monday
      const resMonday = getOpeningStatus('Mo-Fr 08:00-18:00; Sa 09:00-15:00; Su closed', mondayNoon);
      assert.strictEqual(resMonday.status, 'open');
      assert.strictEqual(resMonday.isOpen, true);

      const sundayNoon = new Date('2026-08-30T12:00:00'); // Sunday
      const resSunday = getOpeningStatus('Mo-Fr 08:00-18:00; Sa 09:00-15:00; Su closed', sundayNoon);
      assert.strictEqual(resSunday.status, 'closed');
      assert.strictEqual(resSunday.isOpen, false);

      const thursdayAfternoon = new Date('2026-09-03T15:00:00'); // Thursday
      const resThurs = getOpeningStatus('Thurs 10:00-17:00; Fri 10:00-17:00', thursdayAfternoon);
      assert.strictEqual(resThurs.isOpen, true);
      assert.strictEqual(resThurs.status, 'open');
    });

    it('should filter pins by openNowOnly in filterPins', () => {
      const pins = [
        { id: 1, title: '24/7 Diner', opening_hours: '24/7' },
        { id: 2, title: 'Night Club', opening_hours: 'Closed' },
        { id: 3, title: 'Mystery Place' }
      ];

      const openPins = filterPins(pins, { openNowOnly: true });
      assert.strictEqual(openPins.length, 1);
      assert.strictEqual(openPins[0].title, '24/7 Diner');
    });
  });

  describe('Social & Messaging Share Links Generator (generateShareLinks)', () => {
    it('should generate formatted share URLs with safe encoded parameters', () => {
      const links = generateShareLinks('https://blist-radmuffin.fly.dev/', {
        title: 'bList - Visual Map Bucket List',
        text: 'Check out bList!'
      });

      assert.strictEqual(links.url, 'https://blist-radmuffin.fly.dev/');
      assert.ok(links.sms.startsWith('sms:?&body='));
      assert.ok(links.whatsapp.includes('api.whatsapp.com/send?text='));
      assert.ok(links.messenger.includes('fb-messenger://share/?link='));
      assert.ok(links.twitter.includes('twitter.com/intent/tweet?text='));
      assert.ok(links.email.startsWith('mailto:?subject='));
      assert.ok(links.qrUrl.startsWith('data:image/svg+xml') || links.qrUrl.includes('create-qr-code'));
      assert.ok(links.qrDataUrl.startsWith('data:image/svg+xml'));
      assert.ok(links.qrSvg.startsWith('<svg xmlns="http://www.w3.org/2000/svg"'));
    });

    it('should fallback safely when invalid or empty URL is provided', () => {
      const links = generateShareLinks('');
      assert.strictEqual(links.url, 'https://blist-radmuffin.fly.dev/');
      assert.ok(links.whatsapp.includes('blist-radmuffin.fly.dev'));
      assert.ok(links.qrDataUrl.startsWith('data:image/svg+xml'));
    });
  });

  describe('Whimsical Inspiration Generator (getRandomInspiration & MANIFESTO_RULES)', () => {
    it('should return a valid random inspiration spot with full coordinates and notes', () => {
      const item = getRandomInspiration();
      assert.ok(item);
      assert.ok(item.title);
      assert.ok(item.emoji);
      assert.ok(typeof item.latitude === 'number');
      assert.ok(typeof item.longitude === 'number');
      assert.ok(item.address);
      assert.ok(item.notes);
    });

    it('should respect excludeIndex to avoid immediate consecutive duplicates', () => {
      const item1 = getRandomInspiration();
      const item2 = getRandomInspiration(item1.index);
      if (INSPIRATIONS.length > 1) {
        assert.notStrictEqual(item1.index, item2.index);
      }
    });

    it('should contain 4 whimsical manifesto rules', () => {
      assert.strictEqual(MANIFESTO_RULES.length, 4);
      assert.strictEqual(MANIFESTO_RULES[0].rule, 1);
      assert.ok(MANIFESTO_RULES[0].title.includes('Pin First'));
      assert.ok(MANIFESTO_RULES[1].title.includes('Zero AI'));
      assert.ok(MANIFESTO_RULES[2].title.includes('Noodles'));
      assert.ok(MANIFESTO_RULES[3].title.includes('Your Data'));
    });
  });

  describe('Travel Milestones & Achievement Badges (calculateBadges & BADGE_DEFINITIONS)', () => {
    it('should evaluate 0 unlocked badges for empty pins array', () => {
      const result = calculateBadges({ pins: [], lists: [] });
      assert.strictEqual(result.unlockedCount, 0);
      assert.strictEqual(result.totalBadges, BADGE_DEFINITIONS.length);
      assert.strictEqual(result.percentage, 0);
      assert.strictEqual(result.badges.every(b => !b.unlocked), true);
    });

    it('should unlock Trailblazer on 1 saved pin and First Stamp on 1 visited pin', () => {
      const pins = [
        { id: 1, list_id: 1, title: 'Eiffel Tower', visited: 1, category: 'Sightseeing' }
      ];
      const result = calculateBadges({ pins, lists: [{ id: 1, name: 'Paris' }] });
      const trailblazer = result.badges.find(b => b.id === 'first_pin');
      const firstStamp = result.badges.find(b => b.id === 'first_visit');
      const completedList = result.badges.find(b => b.id === 'list_completed');

      assert.strictEqual(trailblazer.unlocked, true);
      assert.strictEqual(firstStamp.unlocked, true);
      // List completion requires at least 2 places
      assert.strictEqual(completedList.unlocked, false);
    });

    it('should unlock Mission Complete when a list with 2+ pins is 100% visited', () => {
      const pins = [
        { id: 1, list_id: 1, title: 'Tokyo Tower', visited: 1, category: 'Sightseeing' },
        { id: 2, list_id: 1, title: 'Shibuya Crossing', visited: 1, category: 'Sightseeing' },
        { id: 3, list_id: 2, title: 'Kyoto Shrine', visited: 0, category: 'Culture' }
      ];
      const lists = [
        { id: 1, name: 'Tokyo 2026' },
        { id: 2, name: 'Kyoto 2026' }
      ];

      const result = calculateBadges({ pins, lists });
      const completedList = result.badges.find(b => b.id === 'list_completed');
      assert.strictEqual(completedList.unlocked, true);
      assert.strictEqual(completedList.current, 1);
      assert.strictEqual(completedList.percentage, 100);
    });

    it('should unlock category specific badges like Noodle Hunter, Nature Lover, and Priority VIP', () => {
      const pins = [
        { id: 1, title: 'Ramen Ichiran', category: 'Food & Drink', priority: true },
        { id: 2, title: 'Coffee Bar', category: 'Cafe', priority: true },
        { id: 3, title: 'Sushi Dai', category: 'Restaurant', priority: true },
        { id: 4, title: 'Fuji Trail', category: 'Nature & Outdoors' },
        { id: 5, title: 'Beach Cove', category: 'Nature & Outdoors' },
        { id: 6, title: 'National Park', category: 'Nature & Outdoors' }
      ];
      const result = calculateBadges({ pins, lists: [] });
      const foodie = result.badges.find(b => b.id === 'noodle_hunter');
      const nature = result.badges.find(b => b.id === 'nature_lover');
      const priority = result.badges.find(b => b.id === 'priority_vip');

      assert.strictEqual(foodie.unlocked, true);
      assert.strictEqual(nature.unlocked, true);
      assert.strictEqual(priority.unlocked, true);
    });

    it('should unlock Multi-Device Maverick and Secret Cartographer flags correctly', () => {
      const result = calculateBadges({
        pins: [],
        lists: [],
        isSynced: true,
        easterEggUnlocked: true
      });
      const syncBadge = result.badges.find(b => b.id === 'sync_maverick');
      const secretBadge = result.badges.find(b => b.id === 'secret_cartographer');

      assert.strictEqual(syncBadge.unlocked, true);
      assert.strictEqual(secretBadge.unlocked, true);
    });
  });

  describe('Offline SVG QR Code Generator (generateQrSvg)', () => {
    it('should generate valid crisp SVG XML and data URL for standard URLs', () => {
      const url = 'https://blist-radmuffin.fly.dev/';
      const res = generateQrSvg(url, { size: 240, margin: 2 });

      assert.ok(res.svg);
      assert.ok(res.dataUrl);
      assert.ok(res.svg.startsWith('<svg xmlns="http://www.w3.org/2000/svg"'));
      assert.ok(res.svg.includes('viewBox="0 0 25 25"') || res.svg.includes('viewBox="0 0 29 29"') || res.svg.includes('viewBox="0 0 33 33"'));
      assert.ok(res.svg.includes('<rect width='));
      assert.ok(res.svg.includes('<path d="M'));
      assert.ok(res.dataUrl.startsWith('data:image/svg+xml;charset=utf-8,'));
      assert.strictEqual(res.size, 240);
    });

    it('should encode long sync links into higher QR versions without error', () => {
      const syncUrl = 'https://blist-radmuffin.fly.dev/?sync_token=550e8400-e29b-41d4-a716-446655440000-deep-sync-token-with-extra-payload';
      const res = generateQrSvg(syncUrl, { size: 180, margin: 1 });

      assert.ok(res.svg);
      assert.ok(res.dataUrl);
      assert.strictEqual(res.size, 180);
      assert.ok(res.moduleCount > 21); // Higher version matrix
    });

    it('should support custom foreground and background colors', () => {
      const res = generateQrSvg('https://example.com', {
        foreground: '#ef4444',
        background: '#f8fafc'
      });

      assert.ok(res.svg.includes('fill="#f8fafc"'));
      assert.ok(res.svg.includes('fill="#ef4444"'));
    });

    it('should handle empty or null input gracefully', () => {
      const res1 = generateQrSvg('');
      assert.deepStrictEqual(res1, { svg: '', dataUrl: '', size: 0, moduleCount: 0 });

      const res2 = generateQrSvg(null);
      assert.deepStrictEqual(res2, { svg: '', dataUrl: '', size: 0, moduleCount: 0 });
    });
  });

});
