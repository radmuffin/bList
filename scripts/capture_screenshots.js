const { chromium } = require('@playwright/test');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

async function waitPort(port, timeoutMs = 15000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/api/health`);
      if (res.ok) return true;
    } catch (_) {}
    await new Promise(r => setTimeout(r, 200));
  }
  throw new Error(`Server did not start on port ${port} in ${timeoutMs}ms`);
}

async function run() {
  const port = 3099;
  const dbPath = path.join(__dirname, '..', 'screenshots.db');
  
  // Clean old db
  for (const f of [dbPath, `${dbPath}-wal`, `${dbPath}-shm`]) {
    if (fs.existsSync(f)) fs.unlinkSync(f);
  }

  console.log('🚀 Starting bList server on port', port);
  const server = spawn('cargo', ['run'], {
    env: { ...process.env, PORT: String(port), DATABASE_PATH: dbPath, RUST_LOG: 'warn' },
    cwd: path.join(__dirname, '..'),
    stdio: 'ignore'
  });

  try {
    await waitPort(port);
    console.log('✅ Server ready. Seeding places...');

    const userToken = 'usr_screenshot_demo_token_12345';

    // Create a demo trip list
    const createListRes = await fetch(`http://127.0.0.1:${port}/api/lists`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-User-Token': userToken },
      body: JSON.stringify({ name: 'Tokyo & Paris Explorer', icon: '✈️' })
    });
    const listData = await createListRes.json();
    const listId = listData.data.id;

    // Seed pins
    const samplePins = [
      {
        list_id: listId,
        title: 'Shibuya Crossing & Hachiko',
        description: 'Iconic scramble crossing with incredible neon lights and vibrancy.',
        latitude: 35.6595,
        longitude: 139.7004,
        category: 'Sightseeing',
        address: 'Shibuya City, Tokyo, Japan',
        notes: 'Best view from the 2nd floor Starbucks overlooking the intersection.',
        visited: false
      },
      {
        list_id: listId,
        title: 'Ichiran Ramen Shibuya',
        description: 'Famous solo dining tonkotsu ramen with custom flavor cards.',
        latitude: 35.6618,
        longitude: 139.7010,
        category: 'Food & Drink',
        address: '1 Chome-22-7 Jinnan, Shibuya City, Tokyo',
        notes: 'Order with extra garlic and rich broth broth level 4.',
        visited: true
      },
      {
        list_id: listId,
        title: 'Fuglen Tokyo Coffee',
        description: 'Norwegian artisanal coffee bar and cocktail spot near Yoyogi Park.',
        latitude: 35.6675,
        longitude: 139.6923,
        category: 'Cafe',
        address: '1 Chome-16-11 Tomigaya, Shibuya City, Tokyo',
        notes: 'Cozy mid-century interior, exceptional single-origin filter brews.',
        visited: false
      },
      {
        list_id: listId,
        title: 'Senso-ji Temple & Asakusa',
        description: 'Tokyo oldest and most sacred Buddhist temple complex.',
        latitude: 35.7148,
        longitude: 139.7967,
        category: 'Sightseeing',
        address: '2 Chome-3-1 Asakusa, Taito City, Tokyo',
        notes: 'Visit at sunset or early morning before Nakamise Street gets crowded.',
        visited: false
      },
      {
        list_id: listId,
        title: 'Trunk Hotel Lounge',
        description: 'Boutique hotel with great terrace, craft cocktails and local vibes.',
        latitude: 35.6635,
        longitude: 139.7042,
        category: 'Hotel & Stay',
        address: '5 Chome-31 Jingumae, Shibuya City, Tokyo',
        notes: 'Great spot to work or relax with a craft matcha cocktail.',
        visited: false
      }
    ];

    for (const pin of samplePins) {
      await fetch(`http://127.0.0.1:${port}/api/pins`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-User-Token': userToken },
        body: JSON.stringify(pin)
      });
    }

    console.log('📸 Launching Playwright browser...');
    const browser = await chromium.launch({ headless: true });
    const screenshotsDir = path.join(__dirname, '..', 'screenshots');
    if (!fs.existsSync(screenshotsDir)) fs.mkdirSync(screenshotsDir, { recursive: true });

    // 1. Desktop Light Mode Screenshot
    {
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
        deviceScaleFactor: 2
      });
      const page = await context.newPage();
      await page.addInitScript((token) => {
        localStorage.setItem('blist_device_token', token);
        localStorage.setItem('blist_theme', 'light');
      }, userToken);

      await page.goto(`http://127.0.0.1:${port}/`);
      await page.waitForSelector('.pin-card');
      await page.waitForTimeout(1500); // Allow leaflet tiles to render
      await page.screenshot({ path: path.join(screenshotsDir, 'desktop-light.png') });
      console.log('✅ Captured desktop-light.png');
      await context.close();
    }

    // 2. Desktop Dark Mode Screenshot
    {
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
        deviceScaleFactor: 2,
        colorScheme: 'dark'
      });
      const page = await context.newPage();
      await page.addInitScript((token) => {
        localStorage.setItem('blist_device_token', token);
        localStorage.setItem('blist_theme', 'dark');
      }, userToken);

      await page.goto(`http://127.0.0.1:${port}/`);
      await page.waitForSelector('.pin-card');
      await page.waitForTimeout(1500);
      await page.screenshot({ path: path.join(screenshotsDir, 'desktop-dark.png') });
      console.log('✅ Captured desktop-dark.png');
      await context.close();
    }

    // 3. Mobile Map View (iPhone 15 Pro)
    {
      const context = await browser.newContext({
        viewport: { width: 393, height: 852 },
        deviceScaleFactor: 3,
        isMobile: true,
        hasTouch: true
      });
      const page = await context.newPage();
      await page.addInitScript((token) => {
        localStorage.setItem('blist_device_token', token);
        localStorage.setItem('blist_theme', 'light');
      }, userToken);

      await page.goto(`http://127.0.0.1:${port}/`);
      await page.waitForSelector('#map');
      await page.waitForTimeout(1500);
      await page.screenshot({ path: path.join(screenshotsDir, 'mobile-map.png') });
      console.log('✅ Captured mobile-map.png');
      await context.close();
    }

    // 4. Mobile List / Drawer View
    {
      const context = await browser.newContext({
        viewport: { width: 393, height: 852 },
        deviceScaleFactor: 3,
        isMobile: true,
        hasTouch: true
      });
      const page = await context.newPage();
      await page.addInitScript((token) => {
        localStorage.setItem('blist_device_token', token);
        localStorage.setItem('blist_theme', 'light');
      }, userToken);

      await page.goto(`http://127.0.0.1:${port}/`);
      await page.waitForSelector('#toggle-sidebar-btn');
      await page.click('#toggle-sidebar-btn');
      await page.waitForSelector('.sidebar.open');
      await page.waitForTimeout(500);
      await page.screenshot({ path: path.join(screenshotsDir, 'mobile-drawer.png') });
      console.log('✅ Captured mobile-drawer.png');
      await context.close();
    }

    // 5. Sync Modal View
    {
      const context = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        deviceScaleFactor: 2
      });
      const page = await context.newPage();
      await page.addInitScript((token) => {
        localStorage.setItem('blist_device_token', token);
      }, userToken);

      await page.goto(`http://127.0.0.1:${port}/`);
      await page.waitForSelector('#sync-btn-header');
      await page.click('#sync-btn-header');
      await page.waitForSelector('#sync-modal:not(.hidden)');
      await page.waitForTimeout(500);
      await page.screenshot({ path: path.join(screenshotsDir, 'sync-modal.png') });
      console.log('✅ Captured sync-modal.png');
      await context.close();
    }

    await browser.close();
    console.log('🎉 All screenshots generated successfully!');
  } finally {
    server.kill();
    for (const f of [dbPath, `${dbPath}-wal`, `${dbPath}-shm`]) {
      if (fs.existsSync(f)) fs.unlinkSync(f);
    }
  }
}

run().catch(err => {
  console.error('Error generating screenshots:', err);
  process.exit(1);
});
