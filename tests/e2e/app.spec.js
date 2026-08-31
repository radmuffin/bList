const { test, expect } = require('@playwright/test');

test.describe('bList Visual & E2E Suite', () => {

  test.beforeEach(async ({ page }) => {
    // Generate unique device sync token per test run for clean isolation
    const testToken = `test_device_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
    await page.addInitScript((token) => {
      localStorage.setItem('blist_device_token', token);
    }, testToken);

    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('should load page with clean header and default bucket list', async ({ page }) => {
    await expect(page).toHaveTitle(/bList/i);
    const brand = page.locator('.brand-title');
    await expect(brand).toBeVisible();

    // Check list selector is populated with default My Bucket List
    const listSelect = page.locator('#list-select');
    await expect(listSelect).toBeVisible();
  });

  test('should toggle dark mode cleanly', async ({ page }) => {
    const themeBtn = page.locator('#theme-toggle-btn');
    if (await themeBtn.isVisible()) {
      await themeBtn.click();
      const html = page.locator('html');
      await expect(html).toHaveAttribute('data-theme', /dark|light/);
    }
  });

  test('should display clean mobile controls on mobile viewport', async ({ page, isMobile }) => {
    if (isMobile) {
      // Mobile bottom view toggle should be visible
      const mobileToggle = page.locator('.mobile-view-toggle');
      await expect(mobileToggle).toBeVisible();

      // Redundant surprise map FAB should be hidden on mobile
      const surpriseMapFab = page.locator('#btn-surprise-map');
      await expect(surpriseMapFab).toBeHidden();

      // Essential FABs should be visible
      await expect(page.locator('#layer-fab')).toBeVisible();
      await expect(page.locator('#route-fab')).toBeVisible();
      await expect(page.locator('#locate-fab')).toBeVisible();

      // Switch to List View
      await page.locator('#btn-show-list').click();
      await expect(page.locator('#pin-list-view')).toBeVisible();
    }
  });

  test('should create a place and display streamlined card and readable plus code', async ({ page, isMobile }) => {
    // Open Add Place Modal
    await page.locator('#btn-add-place').click();

    const modal = page.locator('#pin-modal');
    await expect(modal).toBeVisible();

    // Fill in place details
    await page.locator('#form-title').fill('Tokyo Tower');
    await page.locator('#form-category').selectOption('Sightseeing');
    await page.locator('#form-address').fill('4 Chome-2-8 Shibakoen, Minato City, Tokyo');

    // Open more options for coordinates
    const moreOptions = page.locator('#pin-more-options');
    await moreOptions.locator('summary').click();
    await page.locator('#form-lat').fill('35.6586');
    await page.locator('#form-lon').fill('139.7454');

    // Submit form
    await page.locator('#btn-submit-pin').click();
    await expect(modal).toBeHidden();

    // Verify pin card appears in list
    if (isMobile) {
      await page.locator('#btn-show-list').click();
    }

    const card = page.locator('.pin-card').first();
    await expect(card).toBeVisible();
    await expect(card.locator('.pin-card-title')).toHaveText('Tokyo Tower');

    // Verify streamlined card controls: exactly status pill and directions
    const statusPill = card.locator('.btn-card-status-pill');
    await expect(statusPill).toBeVisible();
    await expect(statusPill).toHaveText(/Bucket List|Visited/);

    const directionsBtn = card.locator('.btn-card-directions');
    await expect(directionsBtn).toBeVisible();
    await expect(directionsBtn).toHaveAttribute('href', /google\.com\/maps\/dir/);

    // Verify Plus Code is NOT cluttering the top badges row
    const badgesRow = card.locator('.badges-row');
    await expect(badgesRow.locator('.badge-plus-code')).toHaveCount(0);

    // Toggle visited status
    await statusPill.click();
    await expect(statusPill).toHaveClass(/is-visited/);

    // Verify trip progress bar updates
    const progressBar = page.locator('#trip-progress-bar');
    await expect(progressBar).toBeVisible();
  });

  test('should open About modal and show GitHub link', async ({ page, isMobile }) => {
    if (isMobile) {
      await page.locator('#mobile-more-btn').click();
      await page.locator('#mobile-more-menu').getByRole('button', { name: /About bList/i }).click();
    } else {
      await page.locator('#btn-about').click();
    }

    const aboutModal = page.locator('#about-modal');
    await expect(aboutModal).toBeVisible();
    await expect(aboutModal.locator('a[href*="github.com/radmuffin/bList"]').first()).toBeVisible();

    // Close modal via close button
    await aboutModal.locator('.btn-close').click();
    await expect(aboutModal).toBeHidden();
  });

  test('should support multi-stop trip route export', async ({ page }) => {
    // Add two places programmatically via frontend ApiClient
    await page.evaluate(async () => {
      await window.bList.ApiClient.createPin({
        title: 'Spot A',
        latitude: 35.6586,
        longitude: 139.7454,
        category: 'Sightseeing'
      });
      await window.bList.ApiClient.createPin({
        title: 'Spot B',
        latitude: 35.6595,
        longitude: 139.7005,
        category: 'Place'
      });
      const freshPins = await window.bList.ApiClient.fetchPins();
      window.bList.State.allPins = freshPins;
      window.bList.UIManager.renderAll();
    });

    // Toggle route polyline
    await page.locator('#route-fab').click();

    // Route info badge should appear with Google Maps navigation link
    const routeBadge = page.locator('#route-info-badge');
    await expect(routeBadge).toBeVisible();
    const gmapsBtn = routeBadge.getByRole('button', { name: 'Google Maps' });
    await expect(gmapsBtn).toBeVisible();
    const optimizeBtn = routeBadge.getByRole('button', { name: 'Optimize' });
    await expect(optimizeBtn).toBeVisible();
  });
});
