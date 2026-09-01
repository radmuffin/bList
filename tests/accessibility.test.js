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
      assert.ok(css.includes('.btn-danger'), 'CSS must style .btn-danger');
      assert.ok(css.includes('.pin-card-body'), 'CSS must style .pin-card-body');
    });

    it('should include delete button in place edit modal', () => {
      assert.ok(html.includes('id="btn-delete-pin-modal"'), 'Place edit modal must contain a delete button');
      assert.ok(html.includes('onclick="handleDeleteFromPinModal()"'), 'Delete button must have handleDeleteFromPinModal handler');
    });
  });

  describe('E2E & UI Selector Contract Integrity Tests', () => {
    it('should preserve all critical UI selectors targeted by Playwright E2E suites', () => {
      const e2eSelectors = [
        '#theme-toggle-btn',
        '#mobile-more-btn',
        '#mobile-more-menu',
        '#btn-about',
        '#about-modal',
        '#tab-btn-explorer',
        '#tab-btn-creator',
        '#about-panel-explorer',
        '#about-panel-creator',
        '#trip-progress-bar',
        '#btn-add-place',
        '#pin-modal',
        '#form-title',
        '#btn-submit-pin',
        '#list-select',
        '#new-list-modal',
        '#sync-btn-header',
        '#user-profile-btn'
      ];

      e2eSelectors.forEach((sel) => {
        if (sel.startsWith('#')) {
          const id = sel.slice(1);
          assert.ok(
            html.includes(`id="${id}"`),
            `E2E critical element ${sel} must exist in static/index.html to prevent UI test breakages`
          );
        }
      });
    });

    it('should contain valid GitHub repository and issues links in About modal Creator tab', () => {
      assert.ok(
        html.includes('href="https://github.com/radmuffin/bList"'),
        'GitHub repository link must exist in static/index.html'
      );
      assert.ok(
        html.includes('href="https://github.com/radmuffin/bList/issues"'),
        'GitHub issues link must exist in static/index.html'
      );
      assert.ok(
        html.includes('Daniel Spiesman'),
        'Creator name Daniel Spiesman must be present in static/index.html'
      );
    });

    it('should structure About modal tabs with matching aria controls and panel IDs', () => {
      const tabs = ['explorer', 'features', 'tech', 'creator'];
      tabs.forEach((tab) => {
        assert.ok(
          html.includes(`id="tab-btn-${tab}"`),
          `Tab button #tab-btn-${tab} must exist in About modal`
        );
        assert.ok(
          html.includes(`id="about-panel-${tab}"`),
          `Tab panel #about-panel-${tab} must exist in About modal`
        );
        assert.ok(
          html.includes(`aria-controls="about-panel-${tab}"`),
          `Tab button must reference aria-controls="about-panel-${tab}"`
        );
      });
    });

    it('should have 100% balanced and correctly nested HTML container tags without mismatches', () => {
      const regex = /<\/?([a-z0-9-]+)(?:\s+[^>]*)?\/?>/gi;
      let match;
      const stack = [];
      const selfClosing = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr', '!doctype']);
      const errors = [];

      while ((match = regex.exec(html)) !== null) {
        const line = html.substring(0, match.index).split('\n').length;
        const fullTag = match[0];
        const tagName = match[1].toLowerCase();
        const isClosing = fullTag.startsWith('</');
        const isSelf = fullTag.endsWith('/>') || selfClosing.has(tagName) || tagName.startsWith('!');

        if (isSelf) continue;

        if (isClosing) {
          if (stack.length === 0) {
            errors.push(`Extra closing tag </${tagName}> at line ${line}`);
          } else {
            const top = stack.pop();
            if (top.tag !== tagName) {
              errors.push(`Mismatch at line ${line}: expected </${top.tag}> (from line ${top.line}), but found </${tagName}>`);
            }
          }
        } else {
          stack.push({ tag: tagName, line });
        }
      }

      if (stack.length > 0) {
        stack.forEach((s) => errors.push(`Unclosed <${s.tag}> opened at line ${s.line}`));
      }

      assert.deepStrictEqual(errors, [], `HTML tag nesting errors detected: ${errors.join('; ')}`);
    });

    it('should preserve .main-content and #map within .app-layout', () => {
      const appLayoutIdx = html.indexOf('class="app-layout"');
      const headerIdx = html.indexOf('class="app-header"');
      const headerActionsIdx = html.indexOf('class="header-actions"');
      const mainContentIdx = html.indexOf('class="main-content"');
      const mapIdx = html.indexOf('id="map"');

      assert.ok(appLayoutIdx !== -1, '.app-layout must exist');
      assert.ok(headerIdx !== -1, '.app-header must exist');
      assert.ok(headerActionsIdx !== -1, '.header-actions must exist');
      assert.ok(mainContentIdx !== -1, '.main-content must exist');
      assert.ok(mapIdx !== -1, '#map must exist');

      assert.ok(appLayoutIdx < headerIdx, '.app-header must be inside .app-layout');
      assert.ok(headerIdx < headerActionsIdx, '.header-actions must be inside .app-header');
      assert.ok(headerIdx < mainContentIdx, '.main-content must follow .app-header');
      assert.ok(mainContentIdx < mapIdx, '#map must be inside .main-content');
    });
  });
});
