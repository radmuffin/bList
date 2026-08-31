const { defineConfig, devices } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests/e2e',
  timeout: 30000,
  expect: {
    timeout: 5000,
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.05,
    },
  },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? [['html', { open: 'never' }], ['list']] : 'list',
  use: {
    baseURL: 'http://127.0.0.1:3000',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'Desktop Chrome',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1280, height: 800 },
      },
    },
    {
      name: 'Mobile Pixel',
      use: {
        ...devices['Pixel 7'],
      },
    },
    {
      name: 'Mobile Safari',
      use: {
        ...devices['iPhone 14'],
      },
    },
  ],
  webServer: {
    command: 'DATABASE_PATH=:memory: cargo run',
    url: 'http://127.0.0.1:3000/api/info',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
