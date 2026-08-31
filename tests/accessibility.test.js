const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const helpers = require('../static/helpers.js');

describe('Accessibility & UI Layout Contract Tests', () => {
  const html = fs.readFileSync(path.join(__dirname, '../static/index.html'), 'utf8');
  const css = fs.readFileSync(path.join(__dirname, '../static/style.css'), 'utf8');

  describe('Form Accessibility & Labels', () => {
    it('should have accessible labels or placeholders for all modal inputs', () => {
      // Check required input IDs exist and have matching labels
      const formInputIds = [
        'form-title',
        'form-category',
        'form-address',
        'form-visited',
        'form-lat',
        'form-lon',
        'form-image',
        'form-source',
        'form-notes',
        'new-list-name'
      ];

      formInputIds.forEach((id) => {
        const hasLabel = html.includes(`for="${id}"`);
        const hasInput = html.includes(`id="${id}"`);
        assert.ok(hasInput, `Input #${id} should exist in HTML`);
        assert.ok(hasLabel, `Label for="${id}" should exist in HTML for accessibility`);
      });
    });

    it('should have accessible close buttons with aria-label on all modals', () => {
      const closeBtnMatches = html.match(/class="btn-close"[^>]*>/g) || [];
      assert.ok(closeBtnMatches.length >= 3, 'Should have at least 3 modal close buttons');

      closeBtnMatches.forEach((btnTag) => {
        assert.ok(
          btnTag.includes('aria-label') || btnTag.includes('title'),
          `Modal close button ${btnTag} must have aria-label for screen readers`
        );
      });
    });

    it('should have all modal dialogs as direct isolated siblings without nesting', () => {
      const modalIds = ['loading-overlay', 'pin-modal', 'new-list-modal', 'share-list-modal', 'sync-modal', 'about-modal', 'import-modal'];
      modalIds.forEach((id) => {
        assert.ok(html.includes(`id="${id}"`), `Modal #${id} must exist in HTML`);
      });
      // Verify import-modal is not nested inside about-modal
      const aboutIdx = html.indexOf('id="about-modal"');
      const importIdx = html.indexOf('id="import-modal"');
      assert.ok(aboutIdx !== -1 && importIdx !== -1);
      const between = html.slice(aboutIdx, importIdx);
      const openDivs = (between.match(/<div\b/g) || []).length;
      const closeDivs = (between.match(/<\/div>/g) || []).length;
      assert.strictEqual(openDivs, closeDivs, 'About modal must be properly closed before Import modal starts');
    });
  });

  describe('Mobile Floating Controls & Button Cleanliness', () => {
    it('should hide redundant FABs on mobile via desktop-only class', () => {
      // Surprise Map FAB must have desktop-only
      const surpriseMapFabRegex = /id="btn-surprise-map"[^>]*class="[^"]*desktop-only[^"]*"/;
      assert.ok(
        surpriseMapFabRegex.test(html) || html.includes('id="btn-surprise-map" onclick="surpriseMe()" title="Surprise Me (Random Place)"') && html.includes('desktop-only'),
        'Surprise Me button on map must have desktop-only class to avoid cluttering mobile view'
      );
    });

    it('should retain primary navigation tools on mobile', () => {
      assert.ok(html.includes('id="layer-fab"'), 'Layer Switcher FAB must exist');
      assert.ok(html.includes('id="route-fab"'), 'Route Toggle FAB must exist');
      assert.ok(html.includes('id="locate-fab"'), 'GPS Locate FAB must exist');
    });

    it('should include Google Maps multi-stop export in route badge', () => {
      assert.ok(html.includes('id="route-info-badge"'), 'Route info badge must exist');
      assert.ok(html.includes('onclick="openGoogleMapsRoute()"'), 'Route badge must have openGoogleMapsRoute action');
    });

    it('should include Google Maps Route export in mobile more menu', () => {
      assert.ok(html.includes('id="mobile-more-menu"'), 'Mobile more menu must exist');
      assert.ok(
        html.includes('onclick="openGoogleMapsRoute(); closeMobileMoreMenu();"'),
        'Mobile more menu must include Google Maps route item'
      );
    });
  });

  describe('Plus Code Relocation & Readability', () => {
    it('should provide clean Plus Code preview container in place edit modal', () => {
      assert.ok(html.includes('id="form-plus-code-display"'), 'Plus code display row must exist in modal');
      assert.ok(html.includes('id="form-plus-code-val"'), 'Plus code value container must exist');
      assert.ok(html.includes('onclick="copyFormPlusCode()"'), 'Plus code copy button must exist in modal');
    });

    it('should format Plus Codes cleanly with valid open location codes', () => {
      const eiffelPlusCode = helpers.encodePlusCode(48.8584, 2.2945);
      assert.ok(eiffelPlusCode, 'Plus code must not be empty');
      assert.ok(eiffelPlusCode.includes('+'), 'Plus code must have standard plus symbol');
      assert.strictEqual(typeof eiffelPlusCode, 'string');
    });
  });

  describe('CSS Styling & Focus Visibility', () => {
    it('should contain focus ring accessibility rules in stylesheet', () => {
      assert.ok(css.includes(':focus-visible'), 'CSS must define :focus-visible rules for keyboard accessibility');
      assert.ok(css.includes('--focus-ring'), 'CSS must define --focus-ring variable');
    });

    it('should contain styles for card status pills and directions button', () => {
      assert.ok(css.includes('.btn-card-status-pill'), 'CSS must style .btn-card-status-pill');
      assert.ok(css.includes('.btn-card-directions'), 'CSS must style .btn-card-directions');
      assert.ok(css.includes('.popup-plus-code-row'), 'CSS must style .popup-plus-code-row');
    });
  });
});
